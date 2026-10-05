---
title: 'RFC-028: JIT Compiler — Multi-level Execution Engine within VM'
status: 'Draft'
author: 'Chenxu'
created: '2026-06-11'
updated: '2026-07-05'
issue: '#101'
---

# RFC-028: JIT Compiler — Multi-level Execution Engine within VM

> **References**:
>
> - [RFC-018: LLVM AOT Compiler Design](../accepted/018-llvm-aot-compiler.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)

## Summary

This document proposes introducing the Cranelift JIT compiler into the YaoXiang VM backend,
upgrading the VM from a pure interpreter to a **multi-level execution engine**: cold code is
interpreted, while hot functions are compiled by Cranelift into native code. The JIT path shares the
IR normalization pass with the LLVM AOT path from RFC-018 — Cranelift handles fast JIT compilation,
while LLVM handles deep AOT optimization. Each takes what it does best.

**Core positioning: JIT serves the VM, not replaces the VM.**

## Motivation

### Why do we need JIT?

The current VM backend is a pure interpreter, executing 10–100× slower than native code. During
development, tests, scripts, and local debugging are run frequently — these scenarios don't need the
extreme optimization of AOT, but they do need execution speeds noticeably faster than an
interpreter.

### Why not only LLVM AOT?

LLVM AOT compilation takes a long time (seconds), which is unsuitable for development iteration.
Development needs a "change and run" experience: change one line of code → re-run → see results
almost immediately. Cranelift JIT compiles a single function in just 1–5ms, with no perceptible
compilation delay for the user.

### Why Cranelift instead of LLVM ORC JIT?

| Dimension       | Cranelift JIT             | LLVM ORC JIT                    |
| --------------- | ------------------------- | ------------------------------- |
| Compile speed   | 1–5ms/function            | 10–100ms/function               |
| Dependency size | Small                     | Large (full LLVM required)      |
| Code quality    | 70–80% of LLVM -O2        | Extremely high                  |
| Use case        | Dev/debug, fast iteration | Not applicable (see trade-offs) |

Cranelift compiles fast and produces code of sufficient quality. LLVM is left to AOT for offline
deep optimization. One tool, one job done well.

## Proposal

### Core architecture

```
VM execution engine
├── Interpreter layer
│   ├── Execute bytecode instructions
│   ├── Collect hotness data (invocation count + loop backedge count)
│   └── When threshold is reached → submit compilation task
│
├── JIT compilation layer (Cranelift backend)
│   ├── Compilation queue (background thread, non-blocking for interpreter)
│   ├── IR → normalization → Cranelift IR → native code
│   └── Reuse the IR normalization pass from RFC-018 §4.0 (stack → SSA)
│
├── Code cache
│   ├── Function table: function ID → {interpreter entry, JIT entry (optional)}
│   ├── Atomic replacement of compiled function entry
│   └── Grouped by module (reserving interface for hot-reloading)
│
└── Hotness analysis
    ├── Per-function call count + loop backedge count
    ├── Periodic decay (avoid one-shot warmup triggering compilation)
    └── Three-level hotness: Cold → Warm → Hot → Compiled
```

### Integration with existing architecture

```
Source code → Frontend (shared) → IR → ┬→ Bytecode codegen → VM interpreter → [hot functions] → Cranelift JIT
                                       │
                                       └→ LLVM AOT codegen → .o → link → exe (production)
```

JIT and AOT share the **IR normalization pass** (`middle/passes/ir_normalize.rs`); the underlying
codegen is swapped from LLVM to Cranelift.

### Execution flow

```
Function call
  → fn_entry.code_ptr.load()
  → ┬─ Interpreter stub (cold state): interpret bytecode one by one
    └─ JIT native code (hot state): execute machine code directly
  → Return
```

## Detailed design

### 1. Directory structure

```
src/
├── backends/
│   ├── interpreter/              # Existing — VM interpreter
│   │   └── executor/
│   │       ├── engine.rs         # Modified — call entry changed from direct interpretation to FunctionEntry dispatch
│   │       └── ...
│   │
│   ├── jit/                      # New — JIT compilation layer
│   │   ├── mod.rs                # JIT module entry, initialize Cranelift context
│   │   ├── profiler.rs           # Hotness counting + decay + threshold decision
│   │   ├── entry.rs              # FunctionEntry + AtomicPtr management
│   │   ├── cache.rs              # Code cache (mmap executable page management)
│   │   ├── compiler.rs           # IR → Cranelift IR → native code
│   │   ├── types.rs              # YaoXiang types → Cranelift type mapping
│   │   └── abi.rs                # Function calling convention (System V / Microsoft x64)
│   │
│   ├── llvm/                     # Planned — LLVM AOT (RFC-018)
│   ├── common/                   # Existing
│   └── runtime/                  # Existing
│
└── middle/
    └── passes/
        └── ir_normalize.rs       # New — shared IR normalization (stack → SSA)
                                  #   Shared between JIT and LLVM AOT
```

**Key constraints**:

- `backends/jit/` only depends on `middle/` (IR definitions, normalization pass), standard library,
  and the Cranelift crate
- `backends/jit/` does not depend on `backends/llvm/`; the two are peer-level backends
- `backends/jit/` does not depend on `backends/interpreter/`; it interacts through the
  `FunctionEntry` interface

### 2. Hotness analysis and tiered triggering

#### 2.1 Hotness state machine

```
Cold ──(invocation > 50 or backedge > 500)──→ Warm
Warm ──(invocation > 200)────────────────────→ Hot
Hot ──(submitted to compilation queue, compilation complete)──→ Compiled
```

> Thresholds are configurable; the above are defaults. Refer to the actual threshold ranges of
> LuaJIT, JVM C1, and V8 Sparkplug (50–1000).

#### 2.2 Counters

Each function maintains two atomic counters in its `FunctionEntry` (see §4.1 for details):

```rust
// Hotness fields of FunctionEntry (full definition in §4.1)
invocation_count: AtomicU32,   // Number of times the function has been called
backedge_count: AtomicU32,     // Number of loop backedge jumps
state: AtomicU8,              // Cold | Warm | Hot | Compiled
```

#### 2.3 Decay mechanism

Every 5 seconds, all counters are right-shifted by 1 bit (multiplied by 0.5). This prevents code
that runs frequently only at startup (such as initialization traversal) from triggering meaningless
JIT compilation.

```rust
fn decay(entry: &FunctionEntry) {
    entry.invocation_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
    entry.backedge_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
}
```

Uses bitwise operations, with zero division overhead.

#### 2.4 Compilation queue

```
Interpreter thread                    Background JIT thread
    │                                         │
    ├─ Hotness reaches Hot                    │
    ├─ Push compilation request ──────────→   │
    │  (non-blocking for interpreter)          ├─ Take out function IR
    │                                         ├─ IR normalization (stack → SSA)
    │                                         ├─ Cranelift compilation
    │                                         ├─ Write to code cache
    │                                         └─ Atomically update function entry pointer
    │  Next call to this function ←──────────  │
    │  Goes directly to native code            │
```

During compilation, the function continues to be executed by the interpreter. After compilation
completes, the next call atomically switches to the JIT code.

### 3. IR → Cranelift compilation pipeline

#### 3.1 Pipeline

```
YaoXiang IR (stack form)
  → IR normalization pass (stack → register/SSA)    ← Reused from RFC-018 §4.0
  → Cranelift IR construction
  → Cranelift optimization + machine code generation
  → Write to code cache
```

#### 3.2 YaoXiang types → Cranelift types

| YaoXiang Type | Cranelift Type           | Notes                                    |
| ------------- | ------------------------ | ---------------------------------------- |
| `Int`         | `i64`                    |                                          |
| `Int32`       | `i32`                    |                                          |
| `Float`       | `f64`                    |                                          |
| `Float32`     | `f32`                    |                                          |
| `Bool`        | `i8`                     | Cranelift has no `i1`, use `i8`          |
| `Char`        | `i32`                    | Unicode code point                       |
| `String`      | `{ i64, i64 }`           | Pointer + length                         |
| `Void`        | empty tuple              |                                          |
| `&T`          | —                        | Zero-sized, disappears after compilation |
| `&mut T`      | —                        | Zero-sized, disappears after compilation |
| `ref T`       | `{ i64, i64 }`           | Reference-count pointer + data pointer   |
| `*T`          | `i64`                    | Raw pointer                              |
| `List(T)`     | `{ i64, i64, i64 }`      | Data pointer + length + capacity         |
| Struct        | Cranelift struct         |                                          |
| Record enum   | `{ i64, [max_payload] }` | Tag + union                              |
| `?T`          | `{ i8, T }`              | Has-value tag + data                     |

> Compared with the LLVM type table in RFC-018 §3: Cranelift does not distinguish pointer types and
> has no `i1`; overall simpler.

#### 3.3 Key instruction translation

| IR Instruction             | Cranelift IR                                |
| -------------------------- | ------------------------------------------- |
| `Add { dst, lhs, rhs }`    | `iadd` (integer) / `fadd` (float)           |
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

> See the full translation table in the RFC main text. Core principle: the Cranelift instruction set
> covers all YaoXiang IR operations; there is no semantic gap.

#### 3.4 Two normalizations coexist

The VM interpreter needs stack semantics (`Push`/`Pop`/`Dup`/`Swap`), while Cranelift JIT and LLVM
AOT need register/SSA. The IR normalization pass performs one conversion (RFC-018 §4.0); JIT and AOT
share it, without changing the IR's own representation. Each backend consumes the same IR according
to its own needs.

### 4. Function entry table and atomic replacement

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

#### 4.2 Entry dispatch

```
Caller
  → fn_entry.code_ptr.load(Ordering::Acquire)
  → ┬─ Interpreter stub address → execute interpreter, interpret bytecode one by one
    └─ JIT code address         → jump directly to native code
```

One pointer dereference. Modern CPU branch predictors' handling of indirect jumps: the first
prediction is wrong, then all correct. Overhead is about 1 cycle.

#### 4.3 Atomic switching

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

No pausing the interpreter, no safepoint waits, no call-site traversal. One atomic operation
completes the switch.

### 5. Code cache

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

#### 5.2 Executable memory management

```rust
struct NativePage {
    ptr: *mut u8,
    size: usize,
    used: AtomicUsize,     // Bytes used
    remaining: usize,      // Remaining capacity
}

impl CodeCache {
    fn allocate(&self, code_size: usize) -> *mut u8;
    fn deallocate(&self, ptr: *mut u8, code_size: usize);  // Only called when a module is invalidated
}
```

Each module is allocated contiguous mmap executable pages; all JIT functions within a module are
allocated from the same page. When a module is invalidated, the entire page is reclaimed — no
per-function free required.

### 6. Reserved extension points for hot-reloading

The following interfaces compile but are not invoked before hot-reloading is implemented. Interface
design principle: **JIT implementation only needs `insert` and per-function `compare_exchange`;
module-level operations are left to hot-reloading.**

```rust
/// Code cache extension interface (reserved, not implemented)
trait CodeCacheExt {
    /// Invalidate all JIT code for a module, fall back to the interpreter
    fn invalidate_module(&self, module_path: &str);

    /// Invalidate specific functions based on source location range
    fn invalidate_range(&self, file: &str, start: u32, end: u32);

    /// Atomically replace an entire module's function table
    fn swap_module(&self, module_path: &str, new_functions: HashMap<String, FunctionEntry>);
}

/// Compilation queue extension interface (reserved, not implemented)
trait CompileQueueExt {
    /// Priority insertion (hot-reload compilation takes priority over normal JIT compilation)
    fn submit_priority(&self, task: CompileTask);
}
```

**Why group by module?** JIT itself only needs functions. Organizing by module exists entirely to
serve hot-reloading: after a module is recompiled, the entire module's function set can be replaced
atomically, rather than per-function CAS — the latter would cause inconsistent state when there are
cyclic dependencies between functions.

## Trade-offs

### Advantages

1. **Zero-perceived compilation delay**: Cranelift 1–5ms/function, background thread compilation,
   interpreter never pauses
2. **Shared infrastructure**: JIT and AOT share the IR normalization pass (RFC-018 §4.0), no
   reinventing the wheel
3. **Non-breaking**: Pure incremental feature. VM unchanged, interpreter unchanged — just one faster
   hot path added
4. **No LLVM dependency**: VM doesn't introduce LLVM, stays lightweight
5. **Naturally multi-platform**: Cranelift natively supports x86_64 and ARM64, covering all target
   platforms
6. **Hot-reloading reserved**: Code cache grouped by module + function entry indirect jumps lay the
   structural foundation for future hot-reloading

### Disadvantages

1. **New Cranelift dependency**: Introduces a new external crate, requires familiarity with its API
2. **Debugging complexity**: JIT-generated code stack frames need to be compatible with interpreter
   stack frames; debug info mapping requires additional handling
3. **Cold-start hotness delay**: No JIT acceleration in the first few seconds after program start;
   hotness needs to accumulate
4. **Platform ABI**: Different platforms (Linux/macOS/Windows) require separate adaptation for mmap
   and calling conventions

### Consistency with related RFCs

| RFC                             | Consistency                                                         |
| ------------------------------- | ------------------------------------------------------------------- |
| RFC-018 LLVM AOT                | ✅ Share IR normalization pass; JIT and AOT are peer-level backends |
| RFC-024 spawn block concurrency | ✅ spawn blocks compiled to runtime function calls                  |
| RFC-008 Runtime architecture    | ✅ All three runtime tiers (Embedded/Standard/Full) support JIT     |

## Alternatives

| Option                             | Why not chosen                                                                           |
| ---------------------------------- | ---------------------------------------------------------------------------------------- |
| Only use LLVM AOT, no JIT          | Development requires recompiling the whole program, losing the fast iteration experience |
| LLVM ORC JIT                       | High compile latency (10–100ms), large LLVM dependency, unsuitable for embedding in VM   |
| Custom lightweight JIT (dynasm)    | Hand-written backend has high maintenance cost, not as mature as Cranelift               |
| Template JIT                       | Zero optimization, poor code quality, wastes JIT compilation time                        |
| Whole-program JIT (no interpreter) | Slow cold start; simple scripts don't warrant compilation                                |

## Dependencies

- RFC-018 (LLVM AOT) → share IR normalization pass
- RFC-024 (spawn block concurrency) → JIT compilation of spawn blocks
- RFC-008 (runtime architecture) → three-tier runtime JIT support
- Cranelift crate → JIT backend

## References

- [Cranelift IR Documentation](https://github.com/bytecodealliance/wasmtools/tree/main/cranelift)
- [RFC-018: LLVM AOT Compiler Design](../accepted/018-llvm-aot-compiler.md)
- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
- Hölzle, U. (1994). _Adaptive Optimization for Self: Reconciling High Performance with Exploratory
  Programming_. Stanford.

---

## Lifecycle and final destination

| Status        | Location                 | Notes                                      |
| ------------- | ------------------------ | ------------------------------------------ |
| **Draft**     | `docs/src/rfc/draft/`    | Author's draft, awaiting submission review |
| **In review** | `docs/src/rfc/review/`   | Open community discussion and feedback     |
| **Accepted**  | `docs/src/rfc/accepted/` | Becomes an official design document        |
