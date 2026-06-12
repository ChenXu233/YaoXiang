---
title: "RFC-028: JIT Compiler — Multi-Tier Execution Engine in VM"
status: "Draft"
author: "Chenxu"
created: "2026-06-11"
---

# RFC-028: JIT Compiler — Multi-Tier Execution Engine in VM

> **References**:
> - [RFC-018: LLVM AOT Compiler Design](../review/018-llvm-aot-compiler.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)

## Summary

This document proposes introducing a Cranelift JIT compiler into the YaoXiang VM backend, upgrading the VM from a pure interpreter to a **multi-tier execution engine**: cold code is interpreted, hot functions are compiled by Cranelift into native code. The JIT path shares IR normalization passes with the LLVM AOT path defined in RFC-018 — Cranelift handles fast JIT compilation, LLVM handles deep AOT optimization, each playing to its strengths.

**Core positioning: JIT serves the VM, it does not replace the VM.**

## Motivation

### Why is JIT needed?

The current VM backend is a pure interpreter, running 10-100x slower than native code. During development we frequently run tests, scripts, and local debugging — these scenarios do not need AOT's extreme optimization, but they need execution speed noticeably faster than an interpreter.

### Why not just use LLVM AOT?

LLVM AOT compilation takes a long time (seconds), which is unsuitable for development iteration. Development needs a "change and run" experience: change one line of code → rerun → see results almost instantly. Cranelift JIT compiles a single function in just 1-5ms, which users do not perceive as compilation delay.

### Why Cranelift and not LLVM ORC JIT?

| Dimension | Cranelift JIT | LLVM ORC JIT |
|------|--------------|--------------|
| Compilation speed | 1-5ms/function | 10-100ms/function |
| Dependency size | Small | Large (requires full LLVM) |
| Code quality | 70-80% of LLVM -O2 | Extremely high |
| Suitable scenarios | Development debugging, fast iteration | Not suitable (see trade-offs in this document) |

Cranelift compiles fast, and the code quality is sufficient. LLVM is reserved for AOT offline deep optimization. One tool does one thing well.

## Proposal

### Core Architecture

```
VM Execution Engine
├── Interpreter Tier
│   ├── Executes bytecode instructions
│   ├── Collects hotness data (invocation count + loop backedge count)
│   └── Reaches threshold → submits compilation task
│
├── JIT Compilation Tier (Cranelift Backend)
│   ├── Compilation queue (background thread, does not block interpreter)
│   ├── IR → normalization → Cranelift IR → native code
│   └── Reuses IR normalization pass from RFC-018 §4.0 (stack→SSA)
│
├── Code Cache
│   ├── Function table: function ID → {interpreter entry, JIT entry (optional)}
│   ├── Atomic replacement of compiled function entry
│   └── Grouped by module (reserved hot-reload interface)
│
└── Hotness Profiler
    ├── Per-function invocation count + loop backedge count
    ├── Periodic decay (avoid one-time warmup triggering compilation)
    └── Four hotness levels: Cold → Warm → Hot → Compiled
```

### Integration with Existing Architecture

```
Source code → Frontend (shared) → IR → ┬→ Bytecode codegen → VM Interpreter → [hot functions] → Cranelift JIT
                                         │
                                         └→ LLVM AOT codegen → .o → link → exe (production)
```

JIT and AOT share the **IR normalization pass** (`middle/passes/ir_normalize.rs`), with the underlying codegen switched from LLVM to Cranelift.

### Execution Flow

```
Function call
  → fn_entry.code_ptr.load()
  → ┬─ Interpreter stub (cold state): interprets bytecode one by one
    └─ JIT native code (hot state): directly executes machine code
  → Return
```

## Detailed Design

### 1. Directory Structure

```
src/
├── backends/
│   ├── interpreter/              # Existing — VM interpreter
│   │   └── executor/
│   │       ├── engine.rs         # Modified — call entry changes from direct interpretation to FunctionEntry dispatch
│   │       └── ...
│   │
│   ├── jit/                      # New — JIT compilation tier
│   │   ├── mod.rs                # JIT module entry, initializes Cranelift context
│   │   ├── profiler.rs           # Hotness counting + decay + threshold decision
│   │   ├── entry.rs              # FunctionEntry + AtomicPtr management
│   │   ├── cache.rs              # Code cache (mmap executable page management)
│   │   ├── compiler.rs           # IR → Cranelift IR → native code
│   │   ├── types.rs              # YaoXiang type → Cranelift type mapping
│   │   └── abi.rs                # Function calling convention (System V / Microsoft x64)
│   │
│   ├── llvm/                     # Planned — LLVM AOT (RFC-018)
│   ├── common/                   # Existing
│   └── runtime/                  # Existing
│
└── middle/
    └── passes/
        └── ir_normalize.rs       # New — shared IR normalization (stack→SSA)
                                  #   Shared by JIT and LLVM AOT
```

**Key constraints**:
- `backends/jit/` only depends on `middle/` (IR definitions, normalization pass), the standard library, and the Cranelift crate
- `backends/jit/` does not depend on `backends/llvm/`; the two are peer backends
- `backends/jit/` does not depend on `backends/interpreter/`; interaction is through the `FunctionEntry` interface

### 2. Hotness Analysis and Tiered Triggering

#### 2.1 Hotness State Machine

```
Cold ──(invocation > 50 or backedge > 500)──→ Warm
Warm ──(invocation > 200)────────────────────→ Hot
Hot ──(submit to compilation queue, compilation complete)──→ Compiled
```

> The thresholds are configurable; the values above are defaults. Refer to the actual threshold ranges (50-1000) used by LuaJIT, JVM C1, and V8 Sparkplug.

#### 2.2 Counters

Each function maintains two atomic counters in `FunctionEntry` (see §4.1 for the full definition):

```rust
// Hotness fields of FunctionEntry (see §4.1 for full definition)
invocation_count: AtomicU32,   // Number of times the function has been called
backedge_count: AtomicU32,     // Number of loop backedge jumps
state: AtomicU8,               // Cold | Warm | Hot | Compiled
```

#### 2.3 Decay Mechanism

Every 5 seconds, all counters are right-shifted by 1 bit (multiplied by 0.5). This prevents code that runs frequently only once during startup (such as initialization traversal) from triggering meaningless JIT compilation.

```rust
fn decay(entry: &FunctionEntry) {
    entry.invocation_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
    entry.backedge_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
}
```

Using bit operations, zero division overhead.

#### 2.4 Compilation Queue

```
Interpreter thread                    Background JIT thread
    │                                       │
    ├─ Hotness reaches Hot                   │
    ├─ Push compilation request ───────────→ │
    │  (does not block interpreter)          ├─ Take out function IR
    │                                       ├─ IR normalization (stack→SSA)
    │                                       ├─ Cranelift compilation
    │                                       ├─ Write to code cache
    │                                       └─ Atomically update function entry pointer
    │  Next call to this function ←──────────┤
    │  Goes directly to native code           │
```

During compilation, the function continues to be executed by the interpreter. After compilation completes, the next call atomically switches to JIT code.

### 3. IR → Cranelift Compilation Pipeline

#### 3.1 Pipeline

```
YaoXiang IR (stack form)
  → IR normalization pass (stack → register/SSA)    ← Reused from RFC-018 §4.0
  → Cranelift IR construction
  → Cranelift optimization + machine code generation
  → Write to code cache
```

#### 3.2 YaoXiang Type → Cranelift Type

| YaoXiang Type | Cranelift Type | Notes |
|---------------|---------------|------|
| `Int` | `i64` | |
| `Int32` | `i32` | |
| `Float` | `f64` | |
| `Float32` | `f32` | |
| `Bool` | `i8` | Cranelift has no `i1`, use `i8` |
| `Char` | `i32` | Unicode code point |
| `String` | `{ i64, i64 }` | Pointer + length |
| `Void` | Empty tuple | |
| `&T` | — | Zero size, disappears after compilation |
| `&mut T` | — | Zero size, disappears after compilation |
| `ref T` | `{ i64, i64 }` | Reference-counted pointer + data pointer |
| `*T` | `i64` | Raw pointer |
| `List(T)` | `{ i64, i64, i64 }` | Data pointer + length + capacity |
| Struct | Cranelift struct | |
| Record enum | `{ i64, [max_payload] }` | Tag + union |
| `?T` | `{ i8, T }` | Has-value flag + data |

> Compared with the LLVM type table in RFC-018 §3: Cranelift does not distinguish pointer types and has no `i1`, making it overall simpler.

#### 3.3 Key Instruction Translation

| IR Instruction | Cranelift IR |
|---------|-------------|
| `Add { dst, lhs, rhs }` | `iadd` (integer) / `fadd` (float) |
| `Sub { dst, lhs, rhs }` | `isub` / `fsub` |
| `Mul { dst, lhs, rhs }` | `imul` / `fmul` |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv` |
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp eq` |
| `Jmp(label)` | `jump` |
| `JmpIf(cond, label)` | `brnz` |
| `Ret(Some(v))` | `return` |
| `Call { dst, func, args }` | `call` |
| `Load { dst, src }` | `load` |
| `Store { dst, src }` | `store` |
| `Spawn { ... }` | Call runtime `task_spawn` + `task_wait_all` |

> See the full translation table in the RFC main body. Core principle: the Cranelift instruction set covers all operations in YaoXiang IR, with no semantic gaps.

#### 3.4 Coexistence of Two Normalization Modes

The VM interpreter requires stack semantics (`Push`/`Pop`/`Dup`/`Swap`), while Cranelift JIT and LLVM AOT require register/SSA form. The IR normalization pass performs one conversion (RFC-018 §4.0), shared by JIT and AOT, without changing the IR's own representation. Each backend consumes the same IR according to its own needs.

### 4. Function Entry Table and Atomic Replacement

#### 4.1 FunctionEntry

```rust
struct FunctionEntry {
    /// Atomically replaceable execution target
    code_ptr: AtomicPtr<u8>,
    /// Immutable metadata
    bytecode: &'static [u8],        // Interpreter fallback
    ir: &'static FunctionIR,        // Input for JIT compilation
    /// Runtime statistics
    invocation_count: AtomicU32,
    backedge_count: AtomicU32,
    state: AtomicU8,                // Cold | Warm | Hot | Compiled
}
```

#### 4.2 Entry Dispatch

```
Caller
  → fn_entry.code_ptr.load(Ordering::Acquire)
  → ┬─ Interpreter stub address → execute interpreter, interpret bytecode one by one
    └─ JIT code address          → directly jump to native code
```

One pointer dereference. Modern CPU branch predictor handling of indirect jumps: first prediction miss, then all correct afterward. Overhead is about 1 cycle.

#### 4.3 Atomic Switching

One CAS after compilation completes:

```rust
fn install_jit_code(entry: &FunctionEntry, jit_code: *mut u8) -> bool {
    entry.code_ptr.compare_exchange(
        INTERPRETER_STUB,      // Expected: still points to interpreter
        jit_code,              // Replace with: JIT code
        Ordering::AcqRel,
        Ordering::Acquire,
    ).is_ok()
}
```

No pausing the interpreter, no safepoint wait, no call-site traversal. One atomic operation completes the switch.

### 5. Code Cache

#### 5.1 Structure

```
CodeCache:
  modules:
    "main.yao":
      functions:
        "compute"    → FunctionEntry (state: Compiled)
        "process"    → FunctionEntry (state: Cold)
        "init"       → FunctionEntry (state: Compiled)
      native_pages:   [ mmap'd executable memory pages ]
    "lib.yao":
      functions:
        "helper"     → FunctionEntry (state: Compiled)
      native_pages:   [ mmap'd executable memory pages ]
```

#### 5.2 Executable Memory Management

```rust
struct NativePage {
    ptr: *mut u8,
    size: usize,
    used: AtomicUsize,      // Bytes used
    remaining: usize,       // Remaining capacity
}

impl CodeCache {
    fn allocate(&self, code_size: usize) -> *mut u8;
    fn deallocate(&self, ptr: *mut u8, code_size: usize);  // Only called when a module is invalidated
}
```

Each module allocates contiguous mmap executable pages; all JIT functions within a module are allocated from the same page. When a module is invalidated, the entire page is reclaimed — no per-function release needed.

### 6. Reserved Extension Points for Hot Reloading

The following interfaces are written but not called before hot reloading is implemented. Interface design principle: **the JIT implementation only needs `insert` and per-function `compare_exchange`; module-level operations are reserved for hot reloading.**

```rust
/// Code cache extension interface (reserved, not implemented)
trait CodeCacheExt {
    /// Invalidate all JIT code of an entire module, fall back to interpreter
    fn invalidate_module(&self, module_path: &str);

    /// Invalidate specific functions based on source location range
    fn invalidate_range(&self, file: &str, start: u32, end: u32);

    /// Atomically replace the function table of an entire module
    fn swap_module(&self, module_path: &str, new_functions: HashMap<String, FunctionEntry>);
}

/// Compilation queue extension interface (reserved, not implemented)
trait CompileQueueExt {
    /// Priority insertion (hot-reload compilation takes priority over normal JIT compilation)
    fn submit_priority(&self, task: CompileTask);
}
```

**Why group by module?** JIT itself only needs functions. Organizing by module exists entirely to serve hot reloading: after a module is recompiled, the entire module's function set can be atomically replaced, rather than CAS-ing function by function — the latter would lead to inconsistent states when there are circular dependencies between functions.

## Trade-offs

### Advantages

1. **Zero-perceived compilation delay**: Cranelift 1-5ms/function, background thread compilation, no interpreter pause
2. **Shared infrastructure**: JIT and AOT share the IR normalization pass (RFC-018 §4.0), no reinventing the wheel
3. **Non-breaking**: Pure incremental feature. VM unchanged, interpreter unchanged — just an additional faster hot path
4. **No LLVM dependency**: VM does not introduce LLVM, stays lightweight
5. **Natively supports multiple platforms**: Cranelift natively supports x86_64 and ARM64, covering all target platforms
6. **Hot-reload reserved**: Code cache grouped by module + function entry indirect jumps, laying structural foundation for future hot reloading

### Disadvantages

1. **New Cranelift dependency**: Introduces a new external crate, requires familiarity with its API
2. **Debugging complexity**: JIT-generated code stack frames must be compatible with interpreter stack frames, debug information mapping requires additional handling
3. **Cold-start hotness latency**: First few seconds after program start have no JIT acceleration, requires hotness accumulation
4. **Platform ABI**: Different platforms (Linux/macOS/Windows) require separate adaptation for mmap and calling conventions

### Consistency with Related RFCs

| RFC | Consistency |
|-----|--------|
| RFC-018 LLVM AOT | ✅ Shares IR normalization pass, JIT and AOT are peer backends |
| RFC-024 spawn block concurrency | ✅ spawn blocks compile to runtime function calls |
| RFC-008 Runtime architecture | ✅ All three runtime layers (Embedded/Standard/Full) support JIT |

## Alternatives

| Alternative | Why not chosen |
|------|-----------|
| Use LLVM AOT only, no JIT | Development requires recompiling the entire program, losing the fast iteration experience |
| LLVM ORC JIT | High compilation latency (10-100ms), large LLVM dependency, not suitable for embedding in VM |
| Custom lightweight JIT (dynasm) | Hand-written backend has high maintenance cost, less mature than Cranelift |
| Template JIT | Zero optimization, poor code quality, wastes JIT compilation time |
| Whole-program JIT (no interpreter) | Slow cold start, simple scripts do not deserve compilation |

## Dependencies

- RFC-018 (LLVM AOT) → shared IR normalization pass
- RFC-024 (spawn block concurrency) → JIT compilation of spawn blocks
- RFC-008 (Runtime architecture) → three-layer runtime JIT support
- Cranelift crate → JIT backend

## References

- [Cranelift IR Documentation](https://github.com/bytecodealliance/wasmtools/tree/main/cranelift)
- [RFC-018: LLVM AOT Compiler Design](../review/018-llvm-aot-compiler.md)
- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
- Hölzle, U. (1994). *Adaptive Optimization for Self: Reconciling High Performance with Exploratory Programming*. Stanford.

---
## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Draft** | `docs/src/design/rfc/draft/` | Author's draft, awaiting submission for review |
| **In Review** | `docs/src/design/rfc/review/` | Open community discussion and feedback |
| **Accepted** | `docs/src/design/rfc/accepted/` | Becomes an official design document |