---
title: "RFC-018: LLVM AOT Compiler Design"
status: "Under Review"
author: "Chenxu"
created: "2026-02-15"
updated: "2026-06-10 (aligned with RFC-024 spawn block concurrency model, RFC-009 v9 ownership model, RFC-026 FFI mechanism)"
---

# RFC-018: LLVM AOT Compiler Design

> **References**:
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
> - [RFC-026: FFI Core Mechanism](./026-ffi-core-mechanism.md)
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)

> **Deprecated**:
> - Legacy "bottom-up automatic DAG analysis" model — replaced by the RFC-024 spawn block direct subexpression model
> - `@IO`/`@Pure` implicit side-effect inference — replaced by the RFC-024 resource type mechanism
> - `Arc(T)` type mapping — replaced by the RFC-009 v9 `ref` keyword

## Summary

This document designs the LLVM AOT (Ahead-of-Time) compiler for the YaoXiang language. The LLVM backend shares the same compilation frontend as the VM backend (interpreter), forming the dual-backend architecture defined in [RFC-008](../accepted/008-runtime-concurrency-model.md): the VM is used for development and debugging, while LLVM is used for production releases.

**Core Responsibilities**:

```
Source Code → Frontend (shared) → IR → LLVM Codegen → .o → Link Scheduler Static Library → exe
```

The compiler translates YaoXiang source code into native machine code, where:

| Language Feature | Compilation Strategy |
|----------|----------|
| Ordinary code | Sequential machine code, zero scheduling overhead |
| `spawn { }` block | Direct subexpression → task dispatch + synchronous wait (aligned with [RFC-024](../accepted/024-concurrency-model.md)) |
| `native("symbol")` | LLVM `declare external` + parameter marshalling (aligned with [RFC-026](./026-ffi-core-mechanism.md)) |
| `.drop` destructor | RAII cleanup code insertion (aligned with [RFC-009](../accepted/009-ownership-model.md)) |
| `&T` / `&mut T` tokens | Zero-sized types, disappear after compilation |
| `ref T` shared | `{ refcount_ptr, data_ptr }` fat pointer, compiler automatically chooses Rc/Arc |

**Relationship with RFC-024**: RFC-024 defines the **user semantics** of spawn blocks (direct subexpressions create tasks, synchronous blocking wait). This document defines **how these semantics are compiled into machine code**.

**Relationship with RFC-026**: RFC-026 defines the **user syntax** for FFI (`native()`, `[0]` method binding, `.drop`). This document defines **how FFI calls generate LLVM IR**.

---

## Motivation

### Why is an LLVM AOT Compiler Needed?

Currently, YaoXiang has only the interpreter as its execution backend:

| Problem | Impact |
|------|------|
| Performance bottleneck | Interpretation is 10-100x slower than machine code |
| Deployment complexity | Requires carrying the interpreter and runtime |
| Production environment | The interpreter is not suitable for performance-sensitive scenarios |

### LLVM in the Dual-Backend Model

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 defines the dual-backend architecture:

```
                    ┌─────────────────────┐
                    │  Compilation Frontend (unified)  │
                    │  Lexer → Parser     │
                    │  → TypeCheck        │
                    │  → spawn analysis   │
                    │  → escape analysis  │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │  VM Backend (development) │  LLVM Backend (production)  │
      │  IR → interpretation     │  IR → native code           │
      │  step debugging          │  link scheduler static lib  │
      │  rapid iteration         │  output .exe                │
      └───────────────────┘     └───────────────────┘
```

The two backends have **completely consistent behavior**—the only difference is the execution method. The same source code, the same type checking, the same spawn analysis results.

---

## Proposal

### 1. Compiler Architecture

The LLVM backend sits at the final stage of the compilation pipeline, receiving IR from the frontend and generating native code:

```
Source Code
  → Lexer / Parser (frontend/core/)
  → TypeCheck + spawn analysis (frontend/core/typecheck/)
  → IR Generation (middle/core/ir_gen.rs)
  → LLVM Codegen (middle/passes/codegen/llvm/)
      ├── Type mapping: YaoXiang types → LLVM IR types
      ├── Function translation: IR instructions → LLVM IR instructions
      ├── spawn expansion: direct subexpressions → task functions + scheduling calls
      ├── FFI expansion: native() calls → declare + marshalling
      └── Destructor insertion: scope end → .drop() calls
  → LLVM optimization + target code generation
  → Link runtime static library → executable
```

### 2. Compilation Process

```
Phase 1: Frontend (shared with VM backend)
  - Parsing, type checking, spawn block analysis, escape analysis
  - Output: type-annotated IR

Phase 2: LLVM IR Generation
  - Type mapping, function declaration, instruction translation
  - Output: LLVM Module

Phase 3: LLVM Optimization
  - Standard LLVM optimization pipeline (O0/O1/O2/O3)
  - Inlining, constant folding, dead code elimination

Phase 4: Target Code Generation
  - LLVM TargetMachine → .o file
  - Platforms: Linux (ELF), macOS (Mach-O), Windows (COFF)

Phase 5: Linking
  - Link runtime static library (scheduler, allocator)
  - Output: executable
```

### 3. Type Mapping

#### 3.1 YaoXiang → LLVM IR Type Mapping

| YaoXiang Type | LLVM IR Type | Description |
|---------------|-------------|------|
| `Int` | `i64` | Default 64-bit signed integer |
| `Int32` | `i32` | Explicit 32-bit integer (mainly for FFI) |
| `Float` | `f64` | Default 64-bit float |
| `Float32` | `f32` | Explicit 32-bit float (mainly for FFI) |
| `Bool` | `i1` | Boolean value |
| `Char` | `i32` | Unicode code point |
| `String` | `{ i8*, i64 }` | Pointer + byte length |
| `Void` | `{}` | Zero-sized empty type |
| `&T` | — | Zero-sized token, disappears after compilation, produces no IR |
| `&mut T` | — | Zero-sized token, disappears after compilation, produces no IR |
| `ref T` | `{ i32*, T* }` | Fat pointer (reference count pointer + data pointer) |
| `*T` | `T*` | Raw pointer |
| `[T; N]` | `[N x T]` | Fixed-length array |
| `List(T)` | `{ T*, i64, i64 }` | Data pointer + length + capacity |
| Struct | Corresponding LLVM struct | Fields laid out in declaration order |
| Tagged enum | `{ i64, [max_payload_size] }` | Tag + union of maximum payload size |
| `?T` | `{ i1, T }` | Has-value flag + data (general representation) |
| FFI opaque type | `{ i8* }` | Wrapped C pointer |
| Function pointer | `T (...)*` | Function pointer type |

> **`&T` / `&mut T` zero runtime overhead**: [RFC-009](../accepted/009-ownership-model.md) §2.7 defines that the compiler internally assigns brand identifiers to tokens (compile-time unique integers); after monomorphization and inlining, the brands disappear completely—no trace of tokens exists in the generated machine code.

#### 3.2 FFI Parameter Type Mapping

Aligned with [RFC-026](./026-ffi-core-mechanism.md) §2.2, with an additional LLVM IR column:

| C Type | YaoXiang Type | LLVM IR | Description |
|--------|---------------|---------|------|
| `int` | `Int32` | `i32` | |
| `long` | `Int64` | `i64` | |
| `float` | `Float32` | `f32` | |
| `double` | `Float64` | `f64` | |
| `char` | `Char` | `i32` | C char → YaoXiang Char (Unicode compatible) |
| `char*` | `String` | `{ i8*, i64 }` | marshalling: C string → YaoXiang String |
| `bool` | `Bool` | `i1` | |
| `size_t` | `Uint` | `i64` | |
| `void*` | `*Void` | `i8*` | |
| `struct T*` | `T` (transparent type) | `T*` | Pass pointer |
| `typedef struct T T` | `T` (opaque type) | `{ i8* }` | Wrapped C pointer |

### 4. Instruction Translation

Each IR instruction directly maps to the corresponding LLVM IR instruction. Brief mapping table:

| IR Instruction | LLVM IR |
|---------|---------|
| `BinaryOp { add }` | `add` |
| `BinaryOp { sub }` | `sub` |
| `BinaryOp { mul }` | `mul` |
| `BinaryOp { div }` | `sdiv` / `fdiv` |
| `Compare { eq }` | `icmp eq` / `fcmp oeq` |
| `CallStatic` | `call` |
| `CallIndirect` | `call` (via function pointer) |
| `Load` | `load` |
| `Store` | `store` |
| `LoadElement` | `getelementptr` + `load` |
| `Alloca` | `alloca` |
| `Branch` | `br` |
| `BranchCond` | `br i1` |
| `Return` | `ret` |

See the specific implementation in `middle/passes/codegen/llvm/` for detailed instruction translation.

### 5. spawn Block Code Generation

Aligned with [RFC-024](../accepted/024-concurrency-model.md), the compilation of spawn blocks is divided into the following steps.

#### 5.1 Semantic Recap

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // direct subexpression → task 1
    t2 = fetch("url2"),   // direct subexpression → task 2
    return (t1, t2)       // synchronous wait, assemble result
}
```

**Rules** (RFC-024 §2.1):
- **Direct subexpressions** of a spawn block (top-level comma-separated statements) create parallel tasks
- Expressions inside nested `{}` are not direct subexpressions and do not become independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning

#### 5.2 Compilation Steps

```
Step 1: Identify direct subexpressions
  Iterate through the spawn block body, collect top-level statements

Step 2: Dependency analysis
  For each direct subexpression, analyze which variables produced by preceding tasks it references
  No dependencies → can be scheduled immediately in parallel
  Has dependencies → queue and wait for dependent tasks to complete

Step 3: Resource conflict detection (RFC-024 §2.5)
  Check whether the same resource type instance is used by multiple tasks
  Same-instance conflict → mark serial execution order

Step 4: Generate task functions
  Each direct subexpression generates an independent LLVM function (closure)

Step 5: Generate scheduling code
  Call runtime scheduler's task_spawn / task_wait

Step 6: Result assembly
  Collect all task outputs, assemble return tuple
```

#### 5.3 LLVM IR Generation Pattern

```llvm
; spawn block entry
%task_count = 2
%tasks = alloca [2 x %TaskHandle]

; Create task 1: fetch("url1")
%task1_fn = @spawn_closure_1
call @runtime_task_spawn(%tasks[0], %task1_fn, ...)

; Create task 2: fetch("url2")
%task2_fn = @spawn_closure_2
call @runtime_task_spawn(%tasks[1], %task2_fn, ...)

; Synchronously wait for all tasks
call @runtime_task_wait_all(%tasks, %task_count)

; Assemble return value
%r1 = call @runtime_task_result(%tasks[0])
%r2 = call @runtime_task_result(%tasks[1])
ret { %r1, %r2 }
```

#### 5.4 Dependent Tasks

```yaoxiang
result = spawn {
    data = fetch("url"),       // task 1: no dependencies
    processed = parse(data),   // task 2: depends on task 1's data
    return processed
}
```

The compiler detects that `parse(data)` references `data` produced by task 1, and marks the dependency when generating scheduling code:

```llvm
; Task 2 is created with a dependency on task 1
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
;                                                              ↑
;                                                 depends on task 0 (fetch) completion
```

#### 5.5 Automatic Serialization for Resource Types

Resource types defined in [RFC-024 §2.5](../accepted/024-concurrency-model.md) (`FilePath`, `HttpUrl`, `DBUrl`, `Console`, and user-defined resource types) are automatically serialized in spawn blocks:

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // uses SqliteDb (resource type)
    r2 = db.exec("INSERT ...")    // same instance → automatically serialized
}
```

The compiler detects that the same resource instance is used by two tasks and generates a serial dependency:

```llvm
; Task 2 depends on task 1 (same resource automatically serialized)
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
```

#### 5.6 spawn for Data Parallelism

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

The compiler expands this into N independent tasks (N = length of items), limited by the maximum concurrency.

### 6. FFI Code Generation

Aligned with [RFC-026](./026-ffi-core-mechanism.md), this section defines the LLVM IR generation strategy for FFI calls.

#### 6.1 native() Function Declaration

```yaoxiang
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
```

Compiles to LLVM IR:

```llvm
; Declare external C function
declare i8* @sqlite3_open(i8*)

; YaoXiang wrapper function (handles marshalling)
define { i8* } @__yx_sqlite3_open({ i8*, i64 } %filename) {
    ; marshalling: YaoXiang String → C string
    %c_str = extractvalue { i8*, i64 } %filename, 0
    ; Call C function
    %raw = call i8* @sqlite3_open(i8* %c_str)
    ; unmarshalling: C pointer → opaque type
    %result = insertvalue { i8* } undef, i8* %raw, 0
    ret { i8* } %result
}
```

**Key points**:
- `native("sqlite3_open")` → `declare external @sqlite3_open`
- The compiler automatically generates a marshalling wrapper function
- The wrapper function's signature uses YaoXiang types, internally converting to C types

#### 6.2 Parameter Marshalling

| Direction | Conversion |
|------|------|
| YaoXiang `String` → C `char*` | Extract `.ptr` field and pass |
| YaoXiang `Int32` → C `int` | Pass directly (`i32`) |
| YaoXiang `*Void` → C `void*` | Pass directly (`i8*`) |
| YaoXiang `T` (transparent type) → C `struct T*` | Take address and pass |
| YaoXiang `T` (opaque type) → C `struct T*` | Extract pointer from `{ i8* }` and pass |

#### 6.3 LLVM Layout of Opaque Types

The opaque type defined in [RFC-026](./026-ffi-core-mechanism.md) §4.1:

```yaoxiang
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}
```

LLVM layout: `{ i8* }` — a struct containing a C pointer.

**Layout optimization**: When the opaque type has only a single `handle: *Void` field, it can be optimized to use `i8*` directly (omitting the outer struct). The optimized ABI is completely consistent with the C pointer, with zero marshalling overhead. The compiler enables this optimization by default; users are unaware.

#### 6.4 LLVM Representation of `?T` Nullable Return Values

The FFI nullable return value defined in [RFC-026](./026-ffi-core-mechanism.md) §7.6:

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

General LLVM representation: `{ i1, { i8* } }` — has-value flag + data.

**Optimization for FFI null pointers**: If the `T` in `?T` is an opaque type (internally a pointer), the compiler uses a **null pointer = None** optimization:

```llvm
; Optimized LLVM representation: directly use a nullable pointer
define i8* @__yx_sqlite3_open(...) {
    %raw = call i8* @sqlite3_open(...)
    ; null → None, non-null → Some(wrapped as opaque type)
    ret i8* %raw
}
```

Caller:
```llvm
%raw = call i8* @__yx_sqlite3_open(...)
%is_null = icmp eq i8* %raw, null
br i1 %is_null, label %none_branch, label %some_branch
```

This optimization makes FFI calls of `?SqliteDb` have **zero additional overhead**—completely equivalent to a C null check.

#### 6.5 yx-bindgen Integration

The binding files automatically generated by [yx-bindgen](./026-ffi-core-mechanism.md) §6 are treated as ordinary YaoXiang source code during compilation. The compiler does not need to know the code comes from bindgen—the handling of `native()` declarations and `unsafe {}` type definitions is completely consistent.

### 7. Destructor Code Generation

Aligned with the RAII semantics of [RFC-009](../accepted/009-ownership-model.md) and the `.drop` convention of [RFC-026](./026-ffi-core-mechanism.md) §7.

#### 7.1 .drop Binding Identification

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

The compiler identifies the `.drop` binding and marks the destructor function pointer in the type metadata.

#### 7.2 Cleanup Insertion at Scope End

```
User code:
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT ...")
    stmt.step()
    // ← scope end
}

Cleanup inserted by the compiler (reverse order):
    call @sqlite3_finalize(%stmt)    // stmt.drop()
    call @sqlite3_close(%db)          // db.drop()
```

**Insertion points**:
- Normal scope end (`}`)
- Early return (before `return`)
- `?` error propagation path (before `?`)
- End of spawn block (destructor of variables within a task)

#### 7.3 Move and Destructor

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: ownership transferred to db2
// db is now invalid, no drop is inserted for db here
// ← scope end: drop is only inserted for db2
```

The compiler tracks Move semantics ([RFC-009](../accepted/009-ownership-model.md) §1) and only inserts destructor calls at the final holder of a variable.

#### 7.4 Destructor Failure Handling

```llvm
; debug mode: check the destructor return value
%ret = call i32 @sqlite3_close(i8* %handle)
%ok = icmp eq i32 %ret, 0
br i1 %ok, label %done, label %panic
panic:
  call @__yx_panic("destructor failed")
  unreachable
done:
  ret void

; release mode: ignore the return value
call i32 @sqlite3_close(i8* %handle)
ret void
```

### 8. Compilation Artifact Structure

```rust
/// Compilation artifact: machine code + metadata
pub struct CompiledArtifact {
    /// Machine code compiled by LLVM (object file)
    machine_code: Vec<u8>,

    /// spawn block metadata: task function pointers + dependency relations
    spawn_metadata: SpawnMetadata,

    /// FFI symbol table: external symbol references
    ffi_symbols: Vec<FfiSymbol>,

    /// Entry point table
    entries: Vec<EntryPoint>,

    /// Type information (reflection metadata, written into .reflect section, mmap on demand)
    type_info: TypeInfo,
}

/// spawn block metadata
pub struct SpawnMetadata {
    /// Description of each spawn block
    blocks: Vec<SpawnBlockInfo>,
}

pub struct SpawnBlockInfo {
    /// Task function corresponding to each direct subexpression within the spawn block
    tasks: Vec<TaskInfo>,
    /// Resource conflicts: which task pairs need to be serialized
    serialize_pairs: Vec<(usize, usize)>,
}

pub struct TaskInfo {
    /// LLVM function pointer of the task function
    pub func_ptr: usize,
    /// Dependent task indices (empty = no dependencies, can execute immediately)
    pub deps: Vec<usize>,
}

/// FFI symbol reference
pub struct FfiSymbol {
    /// C symbol name
    pub symbol_name: String,
    /// Whether it's a weak reference (missing is allowed)
    pub weak: bool,
}
```

### 9. Runtime Library

Aligned with [RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md), the runtime is linked as a **static library** into the final exe.

```
Final exe internal structure:

┌────────────────────────────────────────────┐
│  User code (native machine code)            │
│  ├── Ordinary functions (sequential exec)   │
│  ├── spawn block expansion (task functions + scheduling calls) │
│  ├── FFI marshalling wrapper functions      │
│  └── RAII destructor code                   │
├────────────────────────────────────────────┤
│  Runtime static library (~200-500KB)        │
│  ├── Thread pool (num_workers)              │
│  ├── Event loop (libuv / io_uring)          │
│  ├── Work-stealing queue (Full Runtime only)│
│  ├── Memory allocator (jemalloc / mimalloc) │
│  └── Reflection metadata (.reflect section, mmap on demand)│
│                                              │
│  Not included:                              │
│  ❌ Bytecode interpreter                     │
│  ❌ JIT compiler                             │
│  ❌ GC                                       │
│  ❌ Virtual machine                          │
└────────────────────────────────────────────┘
```

**Key design**: Task identification and dependency analysis for spawn blocks are done at compile time; the runtime only does "create task → dispatch to thread pool → wait for completion"—data structures are fixed and behavior is predictable.

**Relationship between the three-tier runtime and LLVM** (aligned with RFC-008 §1):

| Runtime | LLVM AOT Behavior |
|--------|---------------|
| **Embedded** | No spawn support, directly generates sequential machine code |
| **Standard** | Supports spawn blocks, DAG within spawn blocks + single-threaded scheduling (num_workers=1) |
| **Full** | Supports spawn blocks, DAG within spawn blocks + multi-threaded scheduling (num_workers>1), supports WorkStealing |

---

## Detailed Design

### Module Directory Structure

Aligned with the directory layout in [RFC-008](../accepted/008-runtime-concurrency-model.md) §6:

```
src/
├── frontend/                          # Compilation frontend (shared by all backends)
│   └── core/typecheck/
│       └── spawn_placement.rs         # spawn block analysis (task identification, dependency analysis, resource conflict detection)
│
├── middle/
│   ├── core/
│   │   ├── ir.rs                      # IR definition (shared by VM and LLVM)
│   │   └── ir_gen.rs                  # IR generation
│   └── passes/
│       ├── codegen/
│       │   ├── mod.rs
│       │   ├── translator.rs          # IR → LLVM IR main translation
│       │   └── llvm/
│       │       ├── mod.rs             # LLVM backend entry
│       │       ├── context.rs         # LLVM context management
│       │       ├── types.rs           # Type mapping (YaoXiang → LLVM IR)
│       │       ├── values.rs          # Value mapping
│       │       ├── func.rs            # Function translation
│       │       ├── spawn.rs           # spawn block expansion
│       │       ├── ffi.rs             # FFI call code generation
│       │       └── drop.rs            # Destructor function insertion
│       ├── lifetime/                  # Lifetime / token liveness analysis
│       └── mono/                      # Monomorphization
│
├── backends/
│   ├── common/                        # Shared values/heap/opcodes
│   ├── interpreter/                   # Tree-walking interpreter (VM backend)
│   └── runtime/                       # Compiled runtime (static library linked into exe)
│       ├── engine.rs                  # Task scheduling engine
│       ├── facade.rs                  # External interface
│       └── task.rs                    # Task representation
│
└── util/
    └── diagnostic/                    # Error diagnostics (shared)
```

> **Key change**: spawn block analysis is in `frontend/core/typecheck/spawn_placement.rs` (shared frontend), not in the LLVM backend. The LLVM backend only consumes the analysis results and generates the corresponding scheduling code.

### Platform ABI Support

| Platform | Target Triple | Output Format | Calling Convention (FFI default) |
|------|-----------|----------|---------------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | ELF | System V AMD64 |
| macOS x86_64 | `x86_64-apple-darwin` | Mach-O | System V AMD64 |
| macOS ARM64 | `aarch64-apple-darwin` | Mach-O | ARM64 AAPCS |
| Windows x86_64 | `x86_64-pc-windows-msvc` | COFF | Microsoft x64 |

FFI calls use the platform's C calling convention by default. Users can override via options like `native("symbol", cc = "stdcall")` (aligned with future extensions of [RFC-026](./026-ffi-core-mechanism.md)).

---

## Trade-offs

### Advantages

1. **Performance**: AOT compilation is 10-100x faster than interpretation
2. **Unified frontend**: VM and LLVM share the same frontend with completely consistent behavior
3. **Zero scheduling overhead**: Ordinary code is directly generated as sequential machine code, with no DAG overhead outside spawn blocks
4. **Static linking**: No external runtime dependencies, a single exe can be deployed
5. **Zero GC**: RAII deterministic destruction, no pauses
6. **Zero-overhead FFI**: `?T` null pointer optimization and opaque type layout optimization make FFI call cost equivalent to C
7. **Compile-time analysis**: spawn block task identification and dependency analysis are done at compile time, runtime only executes

### Disadvantages

1. **LLVM integration complexity**: Requires deep understanding of the inkwell API and LLVM IR
2. **Compilation time**: AOT compilation is slower than the interpreter (a one-time cost)
3. **Debugging experience**: Native code debugging requires DWARF/PDB symbol support (compiler must generate debug info)
4. **Incremental compilation**: Incremental compilation for large projects requires additional design

### Consistency with Related RFCs

| RFC | Consistency |
|-----|--------|
| RFC-024 spawn block concurrency model | ✅ spawn block direct subexpressions → task dispatch |
| RFC-008 runtime architecture | ✅ Dual backend + scheduler static library + module directory structure |
| RFC-009 v9 ownership model | ✅ `&T`/`&mut T` tokens (zero size), `ref T` (fat pointer), `?T` (Option) |
| RFC-026 FFI core mechanism | ✅ `native()` → declare + marshalling, `.drop` → RAII cleanup |

---

## Alternatives

| Alternative | Description | Why Not Chosen |
|------|------|------|
| Interpreter only | No AOT needed | Insufficient performance |
| Pure static compilation (no runtime) | No scheduler linking | spawn blocks require runtime task scheduling |
| Cranelift backend | Faster compilation speed | Runtime performance inferior to LLVM, can be a future optional backend |
| Link external LLVM runtime | Use LLVM built-in runtime | Introduces unnecessary dependencies |

---

## Implementation Strategy

### Phases

#### Phase 1: Foundation Framework
- [ ] Add inkwell dependency
- [ ] Implement LLVM context initialization (`context.rs`)
- [ ] Implement basic type mapping (`types.rs`)

#### Phase 2: Function Translation
- [ ] Implement function declaration translation (`func.rs`)
- [ ] Implement basic instruction translation (arithmetic, control flow, calls) (`translator.rs`)
- [ ] Implement value mapping (`values.rs`)

#### Phase 3: Ownership Type Translation
- [ ] Implement `&T`/`&mut T` tokens (zero size, disappear after compilation)
- [ ] Implement `ref T` (fat pointer `{ i32*, T* }`)
- [ ] Implement `?T` (`{ i1, T }` tagged union)
- [ ] Implement `List(T)` (`{ T*, i64, i64 }`)
- [ ] Implement Move semantics tracking (for destructor insertion decisions)

#### Phase 4: spawn Block Code Generation
- [ ] Consume analysis results from `spawn_placement.rs`
- [ ] Direct subexpression → task function generation
- [ ] Dependent task scheduling code generation
- [ ] Resource conflict serialization
- [ ] spawn for expansion

#### Phase 5: FFI Code Generation
- [ ] `native()` → `declare external` (`ffi.rs`)
- [ ] Parameter marshalling / return value unmarshalling
- [ ] Opaque type layout (including single-field optimization)
- [ ] `?T` null pointer optimization (FFI-specific)

#### Phase 6: Destructor Code Generation
- [ ] `.drop` binding identification
- [ ] Scope-end cleanup insertion (reverse order) (`drop.rs`)
- [ ] Early return path cleanup
- [ ] `?` error propagation path cleanup

#### Phase 7: Runtime Library Linking
- [ ] Implement `runtime_task_spawn` / `runtime_task_wait_all` and other runtime functions
- [ ] Link runtime static library
- [ ] End-to-end integration testing

### Dependencies

- RFC-024 (spawn block concurrency) → input for Phase 4
- RFC-009 v9 (ownership) → input for Phases 3 and 6
- RFC-008 (runtime architecture) → input for Phase 7
- RFC-026 (FFI mechanism) → input for Phase 5

---

## Related Work

### Lazy Task Creation (1990)[^1]

| Attribute | Description |
|------|------|
| Institution | MIT |
| Authors | James R. Larus, Robert H. Halstead Jr. |
| Core | Defer child task creation, create on demand |
| Reference value | Theoretical basis for on-demand scheduling of tasks within spawn blocks |

**Core idea**: Tasks are not created immediately but deferred. When the parent task needs the value of a child task, the child task is then created. This addresses the performance overhead problem of fine-grained parallel tasks[^1]. YaoXiang's spawn block scheduling draws on this idea—tasks are identified at compile time, but dispatched to the thread pool on demand at runtime.

### Lazy Scheduling (2014)[^2]

| Attribute | Description |
|------|------|
| Institution | University of Maryland |
| Authors | Tzannes, Caragea |
| Core | Runtime adaptive scheduling, no additional state |
| Reference value | Reference for Full Runtime WorkStealing scheduler design |

### SISAL Language[^3]

| Attribute | Description |
|------|------|
| Institution | Lawrence Livermore National Laboratory (LLNL) |
| Core | Single-assignment language, Dataflow graph, implicit parallelism |
| Reference value | Feasibility proof of the Dataflow model in industrial applications |

**Key difference**: SISAL's parallelism is **implicit**—the language has single-assignment semantics, and the compiler automatically analyzes the data dependency graph of the entire program to decide parallelism. YaoXiang's parallelism is **explicit**—users mark parallel regions with `spawn {}` blocks, and the compiler analyzes dependencies only within spawn blocks. This avoids the complexity of SISAL's whole-program analysis while preserving user control over parallel behavior.

### Mul-T Parallel Scheme[^4]

| Attribute | Description |
|------|------|
| Institution | MIT |
| Core | Future construct, Lazy Task Creation implementation |
| Reference value | Concrete implementation reference |

### Comparison Summary

| Technique | Lazy Creation | Parallelism Marker | Analysis Scope | Ownership |
|------|----------|----------|----------|--------|
| Lazy Task Creation[^1] | ✅ | Implicit | Whole program | N/A |
| Lazy Scheduling[^2] | ✅ | Implicit | Whole program | N/A |
| SISAL[^3] | ✅ | Implicit (single-assignment) | Whole program | N/A |
| Mul-T[^4] | ✅ | Explicit (future) | Call site | N/A |
| **YaoXiang** | ✅ | **Explicit (spawn block)** | **Within spawn block** | **✅ (Move + token + ref)** |

**YaoXiang's innovation**: It elevates the parallelism marker from "per function call" (future) to "structured block" (spawn). Users write ordinary code and place spawn blocks where parallelism is needed. Analysis scope is constrained within spawn blocks, making compilation efficient and behavior controllable.

---

## Appendix

### Appendix A: Comparison with Rust async

| Feature | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| Compilation artifact | State machine + machine code | Machine code + spawn task metadata |
| Runtime | tokio | Statically linked scheduler (~200-500KB) |
| Concurrency marker | async/await keywords | `spawn { }` block |
| Task creation | State machine generated at compile time | Direct subexpressions identified at compile time → task functions |
| Function coloring | async infection | **No function coloring** |
| Synchronous wait | `.await` | spawn block auto synchronous blocking |
| Memory management | GC (runtime) | **RAII (deterministic)** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler automatically chooses Rc/Arc)** |

### Appendix B: Design Decision Records

| Decision | Resolution | Date |
|------|------|------|
| Adopt LLVM AOT | Direct Codegen, no excessive abstraction | 2026-02-15 |
| Concurrency model alignment | Aligned with RFC-024 spawn block direct subexpression model | 2026-06-10 |
| DAG analysis scope | Within spawn blocks, not across spawn blocks (aligned with RFC-024) | 2026-06-05 |
| Ownership model alignment | Aligned with RFC-009 v9: `&T`/`&mut T` tokens + `ref` keyword | 2026-06-10 |
| Dual-backend model | VM (development) + LLVM (production), aligned with RFC-008 | 2026-05-11 |
| Scheduler form | Static library linked into exe, ~200-500KB, no GC | 2026-05-11 |
| FFI code generation | Integrated with RFC-026: `native()` declare + marshalling | 2026-06-10 |
| Destructor function | `.drop` → RAII cleanup insertion, aligned with RFC-026 §7 | 2026-06-10 |
| Side-effect handling | Removed `@IO`/`@Pure` inference, replaced with RFC-024 resource types | 2026-06-10 |
| Reflection metadata | Compiled into exe .reflect section, mmap on demand | 2026-05-11 |
| Paper citations | Retained Lazy Task Creation et al., clarified differences with YaoXiang | 2026-02-16 |

---

## References

[^1]: Larus, J. R., & Halstead, R. H. (1990). *Lazy Task Creation: A Technique for Increasing the Granularity of Parallel Programs*. MIT.

[^2]: Tzannes, A., & Caragea, G. (2014). *Lazy Scheduling: A Runtime Adaptive Scheduler for Declarative Parallelism*. University of Maryland.

[^3]: Feo, J. T., et al. (1990). *A report on the SISAL language project*. Lawrence Livermore National Laboratory.

[^4]: Mohr, E., et al. (1991). *Mul-T: A high-performance parallel lisp*. MIT.

- [inkwell LLVM bindings](https://github.com/TheDan64/inkwell)
- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
- [RFC-026: FFI Core Mechanism](./026-ffi-core-mechanism.md)

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting submission for review |
| **Under Review** | `docs/design/rfc/review/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/rfc/accepted/` | Becomes an official design document |
| **Rejected** | `docs/design/rfc/` | Retained in the RFC directory |

> Current status: **Under Review** — Aligned with RFC-024 spawn block concurrency model, RFC-009 v9 ownership model, and RFC-026 FFI mechanism