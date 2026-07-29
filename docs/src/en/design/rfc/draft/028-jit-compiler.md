---
title: 'RFC-028: JIT Compiler — Multi-Level Execution Engine within VM'
status: 'Draft'
author: 'Chen Xu'
created: '2026-06-11'
updated: '2026-07-05'
issue: '#101'
---

# RFC-028: JIT Compiler — Multi-Level Execution Engine within VM

> **References**:
>
> - [RFC-018: LLVM AOT Compiler Design](../review/018-llvm-aot-compiler.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)

## Summary

This document proposes introducing a Cranelift JIT compiler to YaoXiang's VM backend, upgrading the
VM from a pure interpreter to a **multi-level execution engine**: cold code executes via
interpretation, hot functions are compiled by Cranelift to native code. The JIT path shares IR
normalization passes with RFC-018's LLVM AOT path; Cranelift handles fast JIT compilation while LLVM
handles deep AOT optimization—each doing what it does best.

**Core positioning: JIT serves VM, not replaces VM.**

## Motivation

### Why JIT is Needed

The current VM backend is a pure interpreter, executing 10-100x slower than native code. During
development, frequent test runs, script executions, and local debugging—these scenarios don't need
AOT's extreme optimization, but require significantly faster execution than an interpreter.

### Why Not LLVM AOT Only?

LLVM AOT compilation takes a long time (seconds), unsuitable for development iteration. Development
requires a "edit and run" experience: change one line of code → rerun → see results almost
instantly. Cranelift JIT compiles a single function in only 1-5ms; users perceive no compilation
delay.

### Why Cranelift Instead of LLVM ORC JIT?

| Dimension       | Cranelift JIT                          | LLVM ORC JIT                 |
| --------------- | -------------------------------------- | ---------------------------- |
| Compile speed   | 1-5ms/function                         | 10-100ms/function            |
| Dependency size | Small                                  | Large (full LLVM needed)     |
| Code quality    | 70-80% of LLVM -O2                     | Extremely high               |
| Use case        | Development/debugging, rapid iteration | Not suitable (see tradeoffs) |

Cranelift compiles fast with sufficient code quality. LLVM is reserved for AOT's offline deep
optimization. One tool does one thing well.

## Proposal

### Core Architecture

```
VM Execution Engine
├── Interpreter Layer
│   ├── Execute bytecode instructions
│   ├── Collect heat data (invocation count + loop backedge count)
│   └── When threshold reached → Submit compilation task
│
├── JIT Compilation Layer (Cranelift Backend)
│   ├── Compilation queue (background thread, doesn't block interpreter)
│   ├── IR → Normalize → Cranelift IR → Native code
│   └── Reuse RFC-018 §4.0 IR normalization pass (stack→SSA)
│
├── Code Cache
│   ├── Function table: function ID → {interpreter entry, JIT entry (optional)}
│   ├── Atomic replacement of compiled function entries
│   └── Grouped by module (hot-reload interface reserved)
│
└── Heat Analysis
    ├── Per-function call count + loop backedge count
    ├── Periodic decay (prevent one-time warmup from triggering compilation)
    └── Three-tier heat: Cold → Warm → Hot → Compiled
```

### Integration with Existing Architecture

```
Source Code → Frontend (shared) → IR → ┬→ Bytecode codegen → VM Interpreter → [hot functions] → Cranelift JIT
                                        │
                                        └→ LLVM AOT codegen → .o → Link → exe (production)
```

JIT and AOT share the **IR normalization pass** (`middle/passes/ir_normalize.rs`), with the
underlying codegen switching from LLVM to Cranelift.

### Execution Flow

```
Function call
  → fn_entry.code_ptr.load()
  → ┬─ Interpreter stub (cold state): interpret bytecode instruction by instruction
    └─ JIT native code (hot state): execute machine code directly
  → Return
```

## Detailed Design

### 1. Directory Structure

```
src/
├── backends/
│   ├── interpreter/              # Existing — VM Interpreter
│   │   └── executor/
│   │       ├── engine.rs         # Modified — call entry changed from direct interpretation to FunctionEntry dispatch
│   │       └── ...
│   │
│   ├── jit/                      # New — JIT Compilation Layer
│   │   ├── mod.rs                # JIT module entry, initialize Cranelift context
│   │   ├── profiler.rs           # Heat counting + decay + threshold decision
│   │   ├── entry.rs              # FunctionEntry + AtomicPtr management
│   │   ├── cache.rs              # Code cache (mmap executable page management)
│   │   ├── compiler.rs           # IR → Cranelift IR → Native code
│   │   ├── types.rs              # YaoXiang type → Cranelift type mapping
│   │   └── abi.rs                # Function calling convention (System V / Microsoft x64)
│   │
│   ├── llvm/                     # Planned — LLVM AOT (RFC-018)
│   ├── common/                   # Existing
│   └── runtime/                  # Existing
│
└── middle/
    └── passes/
        └── ir_normalize.rs       # New — Shared IR normalization (stack→SSA)
                                  #   Used by both JIT and LLVM AOT
```

**Key Constraints**:

- `backends/jit/` only depends on `middle/` (IR definitions, normalization passes), standard
  library, and Cranelift crate
- `backends/jit/` does not depend on `backends/llvm/`; they are peer backends
- `backends/jit/` does not depend on `backends/interpreter/`; interacts via `FunctionEntry`
  interface

### 2. Heat Analysis and Tiered Triggering

#### 2.1 Heat State Machine

```
Cold ──(invocation > 50 OR backedge > 500)──→ Warm
Warm ──(invocation > 200)────────────────────→ Hot
Hot ──(submit to compilation queue, compilation complete)──→ Compiled
```

> Thresholds are configurable; above are default values. Reference: actual threshold ranges from
> LuaJIT, JVM C1, V8 Sparkplug (50-1000).

#### 2.2 Counters

Each function maintains two atomic counters in `FunctionEntry` (see §4.1):

```rust
// Heat fields in FunctionEntry (full definition in §4.1)
invocation_count: AtomicU32,   // Function call count
backedge_count: AtomicU32,     // Loop backedge jump count
state: AtomicU8,              // Cold | Warm | Hot | Compiled
```

#### 2.3 Decay Mechanism

Every 5 seconds all counters shift right by 1 bit (multiply by 0.5). Prevents code that runs high
frequency but only once at startup (e.g., initialization traversal) from triggering meaningless JIT
compilation.

```rust
fn decay(entry: &FunctionEntry) {
    entry.invocation_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
    entry.backedge_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
}
```

Uses bit operations; zero division overhead.

#### 2.4 Compilation Queue

```
Interpreter Thread                         Background JIT Thread
    │                                           │
    ├─ Heat reaches Hot                          │
    ├─ Push compilation request ────────────────→  │
    │  (doesn't block interpreter)               ├─ Extract function IR
    │                                            ├─ IR normalization (stack→SSA)
    │                                            ├─ Cranelift compilation
    │                                            ├─ Write to code cache
    │                                            └─ Atomic update function entry pointer
    │  Next call to this function ←───────────── │
    │  Goes directly to native code              │
```

During compilation, the function still executes via interpreter. After compilation completes, the
next call atomically switches to JIT code.

### 3. IR → Cranelift Compilation Pipeline

#### 3.1 Pipeline

```
YaoXiang IR (stack form)
  → IR normalization pass (stack → register/SSA)    ← Reuse RFC-018 §4.0
  → Cranelift IR construction
  → Cranelift optimization + machine code generation
  → Write to code cache
```

#### 3.2 YaoXiang Type → Cranelift Type

| YaoXiang Type | Cranelift Type           | Notes                                    |
| ------------- | ------------------------ | ---------------------------------------- |
| `Int`         | `i64`                    |                                          |
| `Int32`       | `i32`                    |                                          |
| `Float`       | `f64`                    |                                          |
| `Float32`     | `f32`                    |                                          |
| `Bool`        | `i8`                     | Cranelift has no `i1`, use `i8`          |
| `Char`        | `i32`                    | Unicode code point                       |
| `String`      | `{ i64, i64 }`           | Pointer + length                         |
| `Void`        | Empty tuple              |                                          |
| `&T`          | —                        | Zero-sized, eliminated after compilation |
| `&mut T`      | —                        | Zero-sized, eliminated after compilation |
| `ref T`       | `{ i64, i64 }`           | Reference count pointer + data pointer   |
| `*T`          | `i64`                    | Raw pointer                              |
| `List(T)`     | `{ i64, i64, i64 }`      | Data pointer + length + capacity         |
| Struct        | Cranelift struct         |                                          |
| Record enum   | `{ i64, [max_payload] }` | Tag + union                              |
| `?T`          | `{ i8, T }`              | Has-value flag + data                    |

> Compared with RFC-018 §3's LLVM type table: Cranelift doesn't distinguish pointer types and has no
> `i1`, making it overall simpler.

#### 3.3 Key Instruction Translation

| IR Instruction             | Cranelift IR                                |
| -------------------------- | ------------------------------------------- |
| `Add { dst, lhs, rhs }`    | `iadd` (integer) / `fadd` (floating-point)  |
| `Sub { dst, lhs, rhs }`    | `isub` / `fsub`                             |
| `Mul { dst, lhs, rhs }`    | `imul` / `fmul`                             |
| `Div { dst, lhs, rhs }`    | `sdiv` / `udiv` / `fdiv`                    |
| `Eq { dst, lhs, rhs }`     | `icmp eq` / `fcmp eq`                       |
| `Jmp(label)`               | `jump`                                      |
| `JmpIf(cond, label)`       | `brnz`                                      |
| `Ret(Some(v))`             | `return`                                    |
| `Call { dst, func, args }` | `call`                                      |
| `Load { dst, src }`        | `load`                                      |
| `Store { dst, src }`       | `store`                                     |
| `Spawn { ... }`            | Call runtime `task_spawn` + `task_wait_all` |

> See RFC body for complete translation table. Core principle: Cranelift instruction set covers all
> YaoXiang IR operations; there is no semantic gap.

#### 3.4 Two Normalizations Coexist

VM interpreter needs stack semantics (`Push`/`Pop`/`Dup`/`Swap`), while Cranelift JIT and LLVM AOT
need register/SSA. The IR normalization pass does one conversion (RFC-018 §4.0), shared by JIT and
AOT, without changing IR's representation itself. Each backend consumes the same IR according to its
own needs.

### 4. Function Entry Table and Atomic Replacement

#### 4.1 FunctionEntry

```rust
struct FunctionEntry {
    /// Atomically replaceable execution target
    code_ptr: AtomicPtr<u8>,
    /// Immutable metadata
    bytecode: &'static [u8],        // Interpreter fallback
    ir: &'static FunctionIR,        // JIT compilation input
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
  → ┬─ Interpreter stub address → Execute interpreter, interpret bytecode instruction by instruction
    └─ JIT code address          → Direct jump to native code
```

One pointer dereference. Modern CPU branch predictors for indirect jumps: first prediction miss,
then all correct. Overhead ~1 cycle.

#### 4.3 Atomic Switching

One CAS after compilation completes:

```rust
fn install_jit_code(entry: &FunctionEntry, jit_code: *mut u8) -> bool {
    entry.code_ptr.compare_exchange(
        INTERPRETER_STUB,      // Expected: still pointing to interpreter
        jit_code,              // Replace with: JIT code
        Ordering::AcqRel,
        Ordering::Acquire,
    ).is_ok()
}
```

No interpreter pause, no safepoint wait, no call site traversal. One atomic operation completes the
switch.

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
    used: AtomicUsize,     // Bytes used
    remaining: usize,      // Remaining capacity
}

impl CodeCache {
    fn allocate(&self, code_size: usize) -> *mut u8;
    fn deallocate(&self, ptr: *mut u8, code_size: usize);  // Called only on module invalidation
}
```

Each module allocates contiguous mmap executable pages; all JIT functions within a module are
allocated from the same page. When a module is invalidated, entire pages are reclaimed without
per-function deallocation.

### 6. Hot-Reload Reserved Extension Points

The following interfaces compile but are not invoked before hot-reload implementation. Interface
design principle: **JIT implementation only needs `insert` and single-function `compare_exchange`;
module-level operations are left for hot-reload.**

```rust
/// Code cache extension interface (reserved, not implemented)
trait CodeCacheExt {
    /// Invalidate all JIT code for an entire module, fall back to interpreter
    fn invalidate_module(&self, module_path: &str);

    /// Invalidate specific functions based on source location range
    fn invalidate_range(&self, file: &str, start: u32, end: u32);

    /// Atomically replace function table for an entire module
    fn swap_module(&self, module_path: &str, new_functions: HashMap<String, FunctionEntry>);
}

/// Compilation queue extension interface (reserved, not implemented)
trait CompileQueueExt {
    /// Priority insertion (hot-reload compilation has higher priority than normal JIT compilation)
    fn submit_priority(&self, task: CompileTask);
}
```

**Why group by module?** JIT itself only needs functions. Organizing by module is entirely for
hot-reload service: after module recompilation, the entire module's function set can be atomically
replaced, rather than per-function CAS—which could lead to inconsistent state when functions have
circular dependencies.

## Tradeoffs

### Advantages

1. **Zero perceived compilation delay**: Cranelift 1-5ms/function, background thread compilation,
   interpreter doesn't pause
2. **Shared infrastructure**: JIT and AOT share IR normalization pass (RFC-018 §4.0); no wheel
   reinvention
3. **Non-disruptive**: Pure incremental feature. VM unchanged, interpreter unchanged, just an
   additional faster hot path
4. **No LLVM dependency**: VM doesn't introduce LLVM; stays lightweight
5. **Native multi-platform support**: Cranelift natively supports x86_64 and ARM64, covering all
   target platforms
6. **Hot-reload reserved**: Code cache grouped by module + function entry indirect jump, laying
   structural foundation for future hot-reload

### Disadvantages

1. **Cranelift new dependency**: Introduces new external crate; API familiarity needed
2. **Debugging complexity**: JIT-generated code stack frames need compatibility with interpreter
   stack frames; debug info mapping requires extra handling
3. **Cold-start heat delay**: First few seconds after program start have no JIT acceleration; heat
   needs to accumulate
4. **Platform ABI**: mmap and calling conventions on different platforms (Linux/macOS/Windows) need
   separate adaptation

### Consistency with Related RFCs

| RFC                             | Consistency                                                     |
| ------------------------------- | --------------------------------------------------------------- |
| RFC-018 LLVM AOT                | ✅ Shared IR normalization pass; JIT and AOT are peer backends  |
| RFC-024 spawn block concurrency | ✅ spawn blocks compiled as runtime function calls              |
| RFC-008 Runtime architecture    | ✅ All three runtime tiers (Embedded/Standard/Full) support JIT |

## Alternative Approaches

| Approach                           | Why Not Chosen                                                                           |
| ---------------------------------- | ---------------------------------------------------------------------------------------- |
| LLVM AOT only, no JIT              | Development requires recompiling entire program; loses rapid iteration experience        |
| LLVM ORC JIT                       | High compilation delay (10-100ms), large LLVM dependency; unsuitable for embedding in VM |
| Custom lightweight JIT (dynasm)    | High maintenance cost for hand-written backend; Cranelift is more mature                 |
| Template JIT                       | Zero optimization, poor code quality, wastes JIT compilation time                        |
| Whole-program JIT (no interpreter) | Slow cold-start; simple scripts aren't worth compiling                                   |

## Dependencies

- RFC-018 (LLVM AOT) → Shared IR normalization pass
- RFC-024 (spawn block concurrency) → JIT compilation of spawn blocks
- RFC-008 (runtime architecture) → Three-tier runtime JIT support
- Cranelift crate → JIT backend

## References

- [Cranelift IR Documentation](https://github.com/bytecodealliance/wasmtools/tree/main/cranelift)
- [RFC-018: LLVM AOT Compiler Design](../review/018-llvm-aot-compiler.md)
- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
- Hölzle, U. (1994). _Adaptive Optimization for Self: Reconciling High Performance with Exploratory
  Programming_. Stanford.

---

## Lifecycle and Disposition

| Status           | Location                        | Notes                                      |
| ---------------- | ------------------------------- | ------------------------------------------ |
| **Draft**        | `docs/src/design/rfc/draft/`    | Author draft, pending review submission    |
| **Under Review** | `docs/src/design/rfc/review/`   | Open for community discussion and feedback |
| **Accepted**     | `docs/src/design/rfc/accepted/` | Becomes official design document           |
