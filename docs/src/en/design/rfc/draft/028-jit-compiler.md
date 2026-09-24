---
title: 'RFC-028: JIT Compiler — Multi-tier Execution Engine in VM'
status: 'Draft'
author: 'Chenxu'
created: '2026-06-11'
updated: '2026-07-05'
issue: '#101'
---

# RFC-028: JIT Compiler — Multi-tier Execution Engine in VM

> **References**:
>
> - [RFC-018: LLVM AOT Compiler Design](../accepted/018-llvm-aot-compiler.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)

## Summary

This document proposes introducing a Cranelift JIT compiler for the YaoXiang VM backend, upgrading
the VM from a pure interpreter to a **multi-tier execution engine**: cold code is interpreted, while
hot functions are compiled by Cranelift into native code. The JIT path shares IR normalization
passes with the LLVM AOT path from RFC-018: Cranelift handles fast JIT compilation, LLVM handles
deep AOT optimization—each plays to its strengths.

**Core positioning: JIT serves the VM; it does not replace the VM.**

## Motivation

### Why do we need a JIT?

The current VM backend is a pure interpreter, executing 10-100x slower than native code. During
development, we frequently run tests, scripts, and local debugging—these scenarios don't need AOT's
extreme optimization, but do need execution speed noticeably faster than an interpreter.

### Why not just use LLVM AOT?

LLVM AOT compilation takes a long time (seconds), which is unsuitable for development iteration.
Development requires a "change and run" experience: modify one line of code → rerun → see results
almost instantly. Cranelift JIT compiles a single function in just 1-5ms, with no perceptible
compilation delay.

### Why Cranelift instead of LLVM ORC JIT?

| Dimension         | Cranelift JIT             | LLVM ORC JIT                   |
| ----------------- | ------------------------- | ------------------------------ |
| Compilation speed | 1-5ms/function            | 10-100ms/function              |
| Dependency size   | Small                     | Large (full LLVM needed)       |
| Code quality      | 70-80% of LLVM -O2        | Extremely high                 |
| Use case          | Dev/debug, fast iteration | Not applicable (see tradeoffs) |

Cranelift compiles quickly with sufficient code quality. LLVM is reserved for AOT's offline deep
optimization. One tool, one job done well.

## Proposal

### Core Architecture

```
VM Execution Engine
├── Interpreter Layer
│   ├── Executes bytecode instructions
│   ├── Collects hotness data (invocation count + loop backedge count)
│   └── Reaches threshold → submit compile task
│
├── JIT Compilation Layer (Cranelift Backend)
│   ├── Compile queue (background thread, doesn't block interpreter)
│   ├── IR → normalize → Cranelift IR → native code
│   └── Reuses IR normalization pass from RFC-018 §4.0 (stack→SSA)
│
├── Code Cache
│   ├── Function table: function ID → {interpreter entry, JIT entry (optional)}
│   ├── Atomic replacement of compiled function entries
│   └── Grouped by module (reserves hot-reload interface)
│
└── Hotness Analysis
    ├── Per-function call count + loop backedge count
    ├── Periodic decay (avoids one-time warmup triggering compilation)
    └── Three-tier hotness: Cold → Warm → Hot → Compiled
```

### Integration with Existing Architecture

```
Source code → Frontend (shared) → IR → ┬→ Bytecode codegen → VM Interpreter → [hot functions] → Cranelift JIT
                                        │
                                        └→ LLVM AOT codegen → .o → link → exe (production)
```

JIT and AOT share the **IR normalization pass** (`middle/passes/ir_normalize.rs`); the underlying
codegen is switched from LLVM to Cranelift.

### Execution Flow

```
Function call
  → fn_entry.code_ptr.load()
  → ┬─ Interpreter stub (cold state): interpret bytecode one by one
    └─ JIT native code (hot state): execute machine code directly
  → Return
```

## Detailed Design

### 1. Directory Structure

```
src/
├── backends/
│   ├── interpreter/              # Existing — VM interpreter
│   │   └── executor/
│   │       ├── engine.rs         # Modified — call entry changed from direct interpretation to FunctionEntry dispatch
│   │       └── ...
│   │
│   ├── jit/                      # New — JIT compilation layer
│   │   ├── mod.rs                # JIT module entry, initializes Cranelift context
│   │   ├── profiler.rs           # Hotness counting + decay + threshold decisions
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

- `backends/jit/` only depends on `middle/` (IR definitions, normalization passes), standard
  library, and Cranelift crate
- `backends/jit/` does not depend on `backends/llvm/`; the two are parallel backends
- `backends/jit/` does not depend on `backends/interpreter/`; interaction happens through the
  `FunctionEntry` interface

### 2. Hotness Analysis and Tiered Triggering

#### 2.1 Hotness State Machine

```
Cold ──(invocation > 50 or backedge > 500)──→ Warm
Warm ──(invocation > 200)────────────────────→ Hot
Hot ──(submit to compile queue, compilation complete)──→ Compiled
```

> Thresholds are configurable; the above are defaults. Refer to the actual threshold ranges of
> LuaJIT, JVM C1, and V8 Sparkplug (50-1000).

#### 2.2 Counters

Each function maintains two atomic counters in `FunctionEntry` (see §4.1 for full definition):

```rust
// FunctionEntry's hotness fields (see §4.1 for full definition)
invocation_count: AtomicU32,   // Number of times function was called
backedge_count: AtomicU32,     // Number of loop backedge jumps
state: AtomicU8,              // Cold | Warm | Hot | Compiled
```

#### 2.3 Decay Mechanism

Every 5 seconds, all counters are right-shifted by 1 bit (multiplied by 0.5). Prevents
high-frequency but one-time code at startup (such as initialization traversal) from triggering
meaningless JIT compilation.

```rust
fn decay(entry: &FunctionEntry) {
    entry.invocation_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
    entry.backedge_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
}
```

Uses bit operations—zero division overhead.

#### 2.4 Compile Queue

```
Interpreter thread                    Background JIT thread
    │                                        │
    ├─ Hotness reaches Hot                   │
    ├─ Push compile request ──────────────→  │
    │  (doesn't block interpreter)            ├─ Take out function IR
    │                                        ├─ IR normalization (stack→SSA)
    │                                        ├─ Cranelift compilation
    │                                        ├─ Write to code cache
    │                                        └─ Atomically update function entry pointer
    │  Next call to this function ←──────────  │
    │  Goes directly through native code       │
```

During compilation, the function still executes through the interpreter. After compilation
completes, the next call atomically switches to JIT code.

### 3. IR → Cranelift Compilation Pipeline

#### 3.1 Pipeline

```
YaoXiang IR (stack form)
  → IR normalization pass (stack → register/SSA)    ← Reuses RFC-018 §4.0
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
| `Bool`        | `i8`                     | Cranelift has no `i1`, uses `i8`         |
| `Char`        | `i32`                    | Unicode code point                       |
| `String`      | `{ i64, i64 }`           | Pointer + length                         |
| `Void`        | Empty tuple              |                                          |
| `&T`          | —                        | Zero-sized, disappears after compilation |
| `&mut T`      | —                        | Zero-sized, disappears after compilation |
| `ref T`       | `{ i64, i64 }`           | Refcount pointer + data pointer          |
| `*T`          | `i64`                    | Raw pointer                              |
| `List(T)`     | `{ i64, i64, i64 }`      | Data pointer + length + capacity         |
| Struct        | Cranelift struct         |                                          |
| Record enum   | `{ i64, [max_payload] }` | Tag + union                              |
| `?T`          | `{ i8, T }`              | Has-value flag + data                    |

> Compared with the LLVM type table in RFC-018 §3: Cranelift doesn't distinguish pointer types, has
> no `i1`, and is overall more concise.

#### 3.3 Key Instruction Translation

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

> See RFC body for the complete translation table. Core principle: Cranelift's instruction set
> covers all YaoXiang IR operations; there is no semantic gap.

#### 3.4 Coexistence of Two Normalizations

The VM interpreter needs stack semantics (`Push`/`Pop`/`Dup`/`Swap`), while Cranelift JIT and LLVM
AOT need register/SSA. The IR normalization pass performs one conversion (RFC-018 §4.0), shared by
JIT and AOT, without changing the IR's own representation. Each backend consumes the same IR
according to its own needs.

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
  → ┬─ Interpreter stub address → Execute interpreter, interpret bytecode one by one
    └─ JIT code address           → Jump directly to native code
```

One pointer dereference. Modern CPU branch predictor handling of indirect jumps: first prediction
may be wrong, then all correct. Overhead is about 1 cycle.

#### 4.3 Atomic Switch

After compilation completes, a single CAS:

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

No pausing the interpreter, no safepoint waits, no call site traversal. One atomic operation
completes the switch.

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
    remaining: usize,       // Remaining capacity
}

impl CodeCache {
    fn allocate(&self, code_size: usize) -> *mut u8;
    fn deallocate(&self, ptr: *mut u8, code_size: usize);  // Called only when module becomes invalid
}
```

Each module allocates a contiguous mmap executable page, and all JIT functions within a module are
allocated from the same page. When a module becomes invalid, the entire page is reclaimed—no need to
release function by function.

### 6. Hot Reload Reserved Extension Points

The following interfaces compile but are not called before hot reload is implemented. Interface
design principle: **JIT implementation only needs `insert` and single-function `compare_exchange`;
module-level operations are left for hot reload.**

```rust
/// Code cache extension interface (reserved, not implemented)
trait CodeCacheExt {
    /// Invalidate all JIT code for an entire module, fall back to interpreter
    fn invalidate_module(&self, module_path: &str);

    /// Invalidate specific functions based on source location range
    fn invalidate_range(&self, file: &str, start: u32, end: u32);

    /// Atomically replace the entire module's function table
    fn swap_module(&self, module_path: &str, new_functions: HashMap<String, FunctionEntry>);
}

/// Compile queue extension interface (reserved, not implemented)
trait CompileQueueExt {
    /// Priority insertion (hot reload compilation takes precedence over normal JIT compilation)
    fn submit_priority(&self, task: CompileTask);
}
```

**Why group by module?** JIT itself only needs functions. Organizing by module is entirely for hot
reload: after a module is recompiled, the entire module's function set can be atomically replaced
rather than CAS-ing function by function—the latter would lead to inconsistent state when there are
circular dependencies between functions.

## Tradeoffs

### Advantages

1. **Zero-perception compilation delay**: Cranelift 1-5ms/function, background thread compilation,
   interpreter not paused
2. **Shared infrastructure**: JIT and AOT share the IR normalization pass (RFC-018 §4.0), no
   reinventing the wheel
3. **Non-destructive**: Pure incremental feature. VM unchanged, interpreter unchanged, just an
   additional faster hot path
4. **No LLVM dependency**: VM doesn't introduce LLVM, stays lightweight
5. **Multi-platform by nature**: Cranelift natively supports x86_64 and ARM64, covering all target
   platforms
6. **Hot reload reserved**: Code cache grouped by module + function entry indirect jumps lay the
   structural foundation for future hot reload

### Disadvantages

1. **New Cranelift dependency**: Introduces a new external crate, requires familiarity with its API
2. **Debugging complexity**: JIT-generated code stack frames need to be compatible with interpreter
   stack frames; debug information mapping requires additional handling
3. **Cold start hotness delay**: No JIT acceleration in the first few seconds after program startup;
   hotness accumulation is required
4. **Platform ABI**: Different platforms (Linux/macOS/Windows) require separate adaptation for mmap
   and calling conventions

### Consistency with Related RFCs

| RFC                             | Consistency                                                        |
| ------------------------------- | ------------------------------------------------------------------ |
| RFC-018 LLVM AOT                | ✅ Shared IR normalization pass, JIT and AOT are parallel backends |
| RFC-024 spawn block concurrency | ✅ spawn blocks compile to runtime function calls                  |
| RFC-008 Runtime architecture    | ✅ All three runtime tiers (Embedded/Standard/Full) support JIT    |

## Alternatives

| Option                             | Why not chosen                                                                             |
| ---------------------------------- | ------------------------------------------------------------------------------------------ |
| Only LLVM AOT, no JIT              | Development requires recompiling the entire program, losing the fast iteration experience  |
| LLVM ORC JIT                       | High compilation delay (10-100ms), large LLVM dependency, not suitable for embedding in VM |
| Custom lightweight JIT (dynasm)    | Hand-written backend has high maintenance cost, not as mature as Cranelift                 |
| Template JIT                       | Zero optimization, poor code quality, wastes JIT compilation time                          |
| Whole-program JIT (no interpreter) | Slow cold start, simple scripts don't deserve compilation                                  |

## Dependencies

- RFC-018 (LLVM AOT) → shared IR normalization pass
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

## Lifecycle and Destination

| State         | Location                        | Description                                |
| ------------- | ------------------------------- | ------------------------------------------ |
| **Draft**     | `docs/src/design/rfc/draft/`    | Author's draft, awaiting review submission |
| **In Review** | `docs/src/design/rfc/review/`   | Open community discussion and feedback     |
| **Accepted**  | `docs/src/design/rfc/accepted/` | Becomes official design document           |
