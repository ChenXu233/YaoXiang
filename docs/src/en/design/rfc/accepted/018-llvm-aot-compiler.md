---
title: "RFC-018: LLVM AOT Compiler Design"
status: "Accepted"
author: "Chenxu"
created: "2026-02-15"
updated: "2026-06-11 (Fixed §3.1 ref T type {i32*→i64*}, §4.1 MakeClosure func field type, §6 RFC-026 architecture and rules separation, §8 compilation artifact structure de-concretization, §9 spawn module directory refactoring; added floating-point semantics consistency subsection, runtime library size estimate correction; pre-acceptance additions: §4.0 stack balance precondition statement, §6 RFC-026 freeze precondition, §9 explanation of size estimate difference with RFC-008)"
---

# RFC-018: LLVM AOT Compiler Design

> **References**:
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
> - [RFC-026: FFI Core Mechanism](./026-ffi-core-mechanism.md)
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)

> **Deprecated**:
> - Old "bottom-up automatic DAG analysis" model — replaced by RFC-024 direct subexpression model for spawn blocks
> - `@IO`/`@Pure` implicit side-effect inference — replaced by RFC-024 resource type mechanism
> - `Arc(T)` type mapping — replaced by RFC-009 v9 `ref` keyword

## Summary

This document designs the LLVM AOT (Ahead-of-Time) compiler for the YaoXiang language. The LLVM backend and the VM backend (interpreter) share the same compilation frontend, forming the dual-backend architecture defined in [RFC-008](../accepted/008-runtime-concurrency-model.md): VM is used for development and debugging, while LLVM is used for production release.

**Core responsibilities**:

```
Source code → Frontend (shared) → IR → LLVM Codegen → .o → Link scheduler static library → exe
```

The compiler compiles YaoXiang source code into native machine code, where:

| Language feature | Compilation strategy |
|----------|----------|
| Ordinary code | Sequential machine code, zero scheduling overhead |
| `spawn { }` block | Direct subexpression → task dispatch + synchronous wait (aligned with [RFC-024](../accepted/024-concurrency-model.md)) |
| `native("symbol")` | LLVM `declare external` + parameter marshalling (aligned with [RFC-026](./026-ffi-core-mechanism.md)) |
| `.drop` destructor | RAII cleanup code insertion (aligned with [RFC-009](../accepted/009-ownership-model.md)) |
| `&T` / `&mut T` tokens | Zero-sized types, disappear after compilation |
| `ref T` shared | `{ refcount_ptr, data_ptr }` fat pointer, compiler automatically selects Rc/Arc |

**Relationship with RFC-024**: RFC-024 defines the **user semantics** of spawn blocks (direct subexpression creates task, synchronous blocking wait). This document defines how these semantics **are compiled into machine code**.

**Relationship with RFC-026**: RFC-026 defines the **user syntax** of FFI (`native()`, `[0]` method binding, `.drop`). This document defines how FFI calls **generate LLVM IR**.

---

## Motivation

### Why is an LLVM AOT compiler needed?

Currently, YaoXiang only has an interpreter as its execution backend:

| Problem | Impact |
|------|------|
| Performance bottleneck | Interpreted execution is 10-100x slower than machine code |
| Deployment complexity | Requires carrying the interpreter and runtime |
| Production environment | Interpreter is unsuitable for performance-sensitive scenarios |

### LLVM in the dual-backend model

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 defines the dual-backend architecture:

```
                    ┌─────────────────────┐
                    │   Compilation Frontend (Unified) │
                    │   Lexer → Parser     │
                    │   → TypeCheck        │
                    │   → spawn analysis   │
                    │   → escape analysis  │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │   VM Backend (Development) │     │  LLVM Backend (Production)  │
      │   IR → Interpretation   │     │  IR → Native Code      │
      │   Step Debugging        │     │  Link Scheduler Static Lib │
      │   Rapid Iteration       │     │  Output .exe         │
      └───────────────────┘     └───────────────────┘
```

The **behavior of both backends is completely identical** — the difference lies only in the execution method. The same source code, the same type checking, and the same spawn analysis result.

---

## Proposal

### 1. Compiler Architecture

The LLVM backend is located in the final stage of the compilation pipeline, receiving IR from the frontend and generating native code:

```
Source code
  → Lexer / Parser (frontend/core/)
  → TypeCheck + spawn analysis (frontend/core/typecheck/)
  → IR Generation (middle/core/ir_gen.rs)
  → LLVM Codegen (backends/llvm/)
      ├── Type Mapping: YaoXiang types → LLVM IR types
      ├── Function Translation: IR instructions → LLVM IR instructions
      ├── spawn Expansion: direct subexpression → task function + scheduling call
      ├── FFI Expansion: native() call → declare + marshalling
      └── Destructor Insertion: scope end → .drop() call
  → LLVM Optimization + Target Code Generation
  → Link Runtime Static Library → Executable File
```

### 2. Compilation Flow

```
Phase 1: Frontend (Shared with VM Backend)
  - Parsing, type checking, spawn block analysis, escape analysis
  - Output: Type-annotated IR

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
  - Output: Executable file
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
| `ref T` | `{ i64*, T* }` | Fat pointer (reference count pointer + data pointer) |
| `*T` | `T*` | Raw pointer |
| `[T; N]` | `[N x T]` | Fixed-length array |
| `List(T)` | `{ T*, i64, i64 }` | Data pointer + length + capacity |
| Struct | Corresponding LLVM struct | Fields laid out in declaration order |
| Record enum | `{ i64, [max_payload_size] }` | Tag + union of max payload |
| `?T` | `{ i1, T }` | Some flag + data (general representation) |
| FFI opaque type | `{ i8* }` | Wraps C pointer |
| Function pointer | `T (...)*` | Function pointer type |

> **`&T` / `&mut T` Zero Runtime Overhead**: [RFC-009](../accepted/009-ownership-model.md) §2.7 defines that the compiler internally assigns brand identifiers (unique compile-time integers) to tokens, and after monomorphization and inlining, the brand completely disappears — no token trace exists in the generated machine code.

#### 3.2 FFI Parameter Type Mapping

Aligned with [RFC-026](./026-ffi-core-mechanism.md) §2.2, supplemented with the LLVM IR column:

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
| `typedef struct T T` | `T` (opaque type) | `{ i8* }` | Wraps C pointer |

### 4. IR Normalization and Instruction Translation

#### 4.0 IR Normalization (Stack → Register)

The current IR (`src/middle/core/ir.rs`) contains stack operation instructions (`Push`/`Pop`/`Dup`/`Swap`), which are designed for the bytecode VM. LLVM IR is in SSA form and does not accept stack operations.

**Handling strategy**: The LLVM path first passes through a lightweight normalization pass before instruction translation:

| Stack Instruction | Normalization Strategy |
|--------|-----------|
| `Push(r)` | Record `stack.push(r)`, produces no IR |
| `Pop(r)` | `r = stack.pop()`, produces `load` (from stack slot) |
| `Dup` | `stack.push(stack.top())`, produces no IR |
| `Swap` | Swap top two elements on stack, produces no IR |

After normalization, all operands become register/local variable references, and all stack operations are eliminated. This pass is executed as the first step of `translator.rs`.

> **Why not eliminate stack instructions at the IR level?** Because the VM backend requires stack semantics. Normalizing at the LLVM translation entry point keeps the IR shared between the two backends — each backend consumes the same IR according to its own needs.
>
> **Precondition**: The IR generation phase guarantees stack balance — all control flow paths arrive at the same stack depth when they reach the same program point (the VM bytecode backend depends on the same precondition, otherwise bytecode execution will fail). The normalization pass does not check this precondition; violation results in undefined behavior in the LLVM backend.

#### 4.1 Instruction Translation Table

The following lists the LLVM IR translation strategy for each variant in the `Instruction` enum. Instruction names are exactly consistent with `src/middle/core/ir.rs`.

**Arithmetic instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `Add { dst, lhs, rhs }` | `add` (integer) / `fadd` (float) | Choose integer or float addition based on type |
| `Sub { dst, lhs, rhs }` | `sub` / `fsub` | |
| `Mul { dst, lhs, rhs }` | `mul` / `fmul` | |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv` | Signed/unsigned/float division |
| `Mod { dst, lhs, rhs }` | `srem` / `urem` | Signed/unsigned modulo |
| `Neg { dst, src }` | `sub 0, src` (integer) / `fneg` (float) | |

**Bitwise instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `And { dst, lhs, rhs }` | `and` | |
| `Or { dst, lhs, rhs }` | `or` | |
| `Xor { dst, lhs, rhs }` | `xor` | |
| `Shl { dst, lhs, rhs }` | `shl` | Left shift |
| `Shr { dst, lhs, rhs }` | `lshr` | Logical right shift |
| `Sar { dst, lhs, rhs }` | `ashr` | Arithmetic right shift |

**Comparison instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp oeq` | |
| `Ne { dst, lhs, rhs }` | `icmp ne` / `fcmp one` | |
| `Lt { dst, lhs, rhs }` | `icmp slt` / `fcmp olt` | |
| `Le { dst, lhs, rhs }` | `icmp sle` / `fcmp ole` | |
| `Gt { dst, lhs, rhs }` | `icmp sgt` / `fcmp ogt` | |
| `Ge { dst, lhs, rhs }` | `icmp sge` / `fcmp oge` | |

**Control flow instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `Jmp(label)` | `br label %L` | Unconditional jump |
| `JmpIf(cond, label)` | `br i1 %cond, label %L, label %fallthrough` | Conditional jump |
| `JmpIfNot(cond, label)` | `br i1 %cond, label %fallthrough, label %L` | Conditional not-jump |
| `Ret(Some(v))` | `ret T %v` | With return value |
| `Ret(None)` | `ret void` | No return value |

**Call instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `Call { dst, func, args }` | `%r = call T @func(...)` | Static call |
| `CallVirt { dst, obj, method_name, args }` | vtable GEP + `call` (function pointer) | Virtual method call, looked up through vtable |
| `CallDyn { dst, func, args }` | `%r = call T %func(...)` | Dynamic call (closure/function pointer) |
| `TailCall { func, args }` | `musttail call` / `tail call` | Tail call optimization |

**Memory instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `Move { dst, src }` | — | After normalization, becomes register copy; SSA construction can eliminate most |
| `Load { dst, src }` | `%v = load T, T* %src` | |
| `Store { dst, src }` | `store T %src, T* %dst` | |
| `Alloc { dst, size }` | `%p = alloca T` (stack) / `call @malloc` (escape to heap) | Escape analysis determines allocation location |
| `Free(ptr)` | `call @free(%ptr)` (heap) / — (stack, automatic reclaim) | |
| `AllocArray { dst, size, elem_size }` | `%p = alloca [N x T]` (stack) / `call @malloc` (heap) | |

**Struct/array access instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `LoadField { dst, src, field }` | `%ptr = getelementptr T, T* %src, 0, field` + `load` | |
| `StoreField { dst, field, src }` | `%ptr = getelementptr T, T* %dst, 0, field` + `store` | |
| `LoadIndex { dst, src, index }` | `%ptr = getelementptr T, T* %src, 0, %index` + `load` | |
| `StoreIndex { dst, index, src }` | `%ptr = getelementptr T, T* %dst, 0, %index` + `store` | |
| `CreateStruct { dst, type_name, fields }` | `insertvalue` chain | Construct LLVM struct in field order |

**Type conversion instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `Cast { dst, src, target_type }` | `bitcast` / `trunc` / `zext` / `sext` / `fptrunc` / `fpext` / `sitofp` / `fptosi` / `inttoptr` / `ptrtoint` | Choose appropriate cast instruction based on source/target type combination |
| `TypeTest(val, type)` | — | Compile-time type test, generates `icmp eq` to compare type tags |

**Ownership and borrow instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `Borrow { dst, src, mutable }` | — | **Zero-sized token, completely disappears after compilation**, produces no IR |
| `Release(val)` | — | **Zero-sized token, completely disappears after compilation** |
| `Move { dst, src }` | — | Ownership transfer, becomes register copy after normalization |
| `Drop(val)` | `call void @T.drop(T* %val)` | Call type's destructor function (see §7) |
| `ShareRef { dst, src }` | `call %T* @Arc_new(%src)` / `call %T* @Rc_new(%src)` | Compiler automatically selects Arc/Rc based on cross-thread or not |
| `ArcNew { dst, src }` | `call %T* @Arc_new(%src)` | Atomic reference count = 1 |
| `ArcClone { dst, src }` | `call %T* @Arc_clone(%src)` | Atomic increment reference count |
| `ArcDrop(val)` | `call void @Arc_drop(%val)` | Atomic decrement + conditional release |

**Concurrency instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `Spawn { closures, plan, result }` | Expanded into scheduler call sequence | See §5 for details, runtime `task_spawn` + `task_wait_all` |
| `Yield` | — | AOT path uses synchronous wait for spawn blocks, no yield needed; ignored |

**unsafe blocks and raw pointer instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `UnsafeBlockStart` | — | **Compile-time marker, produces no IR** |
| `UnsafeBlockEnd` | — | **Compile-time marker, produces no IR** |
| `PtrFromRef { dst, src }` | `%p = ptrtoint T* %src to i64` (or directly copy pointer) | |
| `PtrDeref { dst, src }` | `%v = load T, T* %src` | |
| `PtrStore { dst, src }` | `store T %src, T* %dst` | |
| `PtrLoad { dst, src }` | `%v = load T, T* %src` | |

**String instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `StringLength { dst, src }` | `%len = extractvalue { i8*, i64 } %src, 1` | String is `{ ptr, len }`, length in field 1 |
| `StringConcat { dst, lhs, rhs }` | `call String @yx_string_concat(%lhs, %rhs)` | Runtime helper function |
| `StringGetChar { dst, src, index }` | `getelementptr` + `load i32` | With bounds check |
| `StringFromInt { dst, src }` | `call String @yx_string_from_int(%src)` | Runtime helper function |
| `StringFromFloat { dst, src }` | `call String @yx_string_from_f64(%src)` | Runtime helper function |

**Closure instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `MakeClosure { dst, func: String, env }` | Allocate closure struct + fill function pointer (looked up by function name) and environment | `{ fn_ptr, env_fields... }` |
| `LoadUpvalue { dst, upvalue_idx }` | `%v = extractvalue %env, upvalue_idx` | Read upvalue from closure environment |
| `StoreUpvalue { src, upvalue_idx }` | `%env = insertvalue %env, %src, upvalue_idx` | Write to closure environment |
| `CloseUpvalue(val)` | Copy upvalue from stack to heap | |

**Other instructions**:

| IR Instruction | LLVM IR | Description |
|---------|---------|------|
| `HeapAlloc { dst, type_id }` | `call i8* @malloc(i64 size)` + type tag write | Heap allocation + type info |
| `NewDict { dst, keys, values }` | `call Dict @yx_dict_new(%keys, %values)` | Runtime helper function |

> **Note**: `Push`/`Pop`/`Dup`/`Swap` are eliminated in the §4.0 normalization phase and do not appear in the translation table. `Borrow`/`Release` are zero-sized compile-time tokens that produce no machine code.

### 5. spawn Block Code Generation

Aligned with [RFC-024](../accepted/024-concurrency-model.md), the compilation of spawn blocks is divided into the following steps.

#### 5.1 Semantic Review

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // Direct subexpression → task 1
    t2 = fetch("url2"),   // Direct subexpression → task 2
    return (t1, t2)       // Synchronous wait, assemble results
}
```

**Rules** (RFC-024 §2.1):
- The **direct subexpressions** of a spawn block (top-level comma-separated statements) create parallel tasks
- Expressions inside nested `{}` are not direct subexpressions and do not become independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning

#### 5.2 Compilation Steps

```
Step 1: Identify direct subexpressions
  Traverse the spawn block body, collect top-level statements

Step 2: Dependency analysis
  For each direct subexpression, analyze which variables produced by previous tasks it references
  No dependency → Can be immediately scheduled in parallel
  Has dependency → Queue and wait for dependency tasks to complete

Step 3: Resource conflict detection (RFC-024 §2.5)
  Check whether the same resource type instance is used by multiple tasks
  Same instance conflict → Mark serial execution order

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
    data = fetch("url"),       // Task 1: no dependency
    processed = parse(data),   // Task 2: depends on task 1's data
    return processed
}
```

The compiler detects that `parse(data)` references `data` produced by task 1, and marks the dependency when generating scheduling code:

```llvm
; Task 2 is created with dependency on task 1
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
;                                                              ↑
;                                                 Depends on task 0 (fetch) completing
```

#### 5.5 Automatic Serialization for Resource Types

The resource types (`FilePath`, `HttpUrl`, `DBUrl`, `Console`, and user-defined resource types) defined in [RFC-024 §2.5](../accepted/024-concurrency-model.md) are automatically serialized within spawn blocks:

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // Uses SqliteDb (resource type)
    r2 = db.exec("INSERT ...")    // Same instance → automatically serialized
}
```

The compiler detects that the same resource instance is used by two tasks and generates serial dependency:

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

The compiler expands into N independent tasks (N = length of items), limited by maximum concurrency.

### 6. FFI Code Generation

> ⚠️ **Dependency Note**: The FFI code generation **architecture** defined in this section (`native("x")` → `declare external @x` → marshalling wrapper function → call) is stable and will not change with RFC-026 syntax changes. The specific parameter marshalling rule tables (§6.2) and opaque type layout (§6.3) reference the definitions in RFC-026 — if RFC-026's `native()` syntax or marshalling rules change, only the corresponding mapping tables in this document need to be updated, and the architecture layer is unaffected. RFC-026 current status: **Under Review**, in the same `review/` directory as this document.
>
> **Pre-acceptance precondition**: Before this RFC is accepted, the parts of RFC-026 related to §6 of this document (`native()` declaration syntax, parameter marshalling rules, opaque type `{ i8* }` layout, `.drop` binding conventions) should be frozen first or accepted together with 026. Otherwise the mapping tables in §6.2/§6.3/§7 may become obsolete before implementation.

Aligned with [RFC-026](./026-ffi-core-mechanism.md), this section defines the LLVM IR generation strategy for FFI calls.

#### 6.1 native() Function Declaration

```yaoxiang
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
```

Compiled to LLVM IR:

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
| YaoXiang `String` → C `char*` | Extract `.ptr` field for passing |
| YaoXiang `Int32` → C `int` | Pass directly (`i32`) |
| YaoXiang `*Void` → C `void*` | Pass directly (`i8*`) |
| YaoXiang `T` (transparent type) → C `struct T*` | Take address for passing |
| YaoXiang `T` (opaque type) → C `struct T*` | Extract pointer from `{ i8* }` for passing |

#### 6.3 LLVM Layout of Opaque Types

Opaque types defined in [RFC-026](./026-ffi-core-mechanism.md) §4.1:

```yaoxiang
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}
```

LLVM layout: `{ i8* }` — a struct containing a C pointer.

**Layout optimization**: When an opaque type has only one `handle: *Void` field, it can be optimized to directly use `i8*` (omitting the outer struct). The optimized ABI is fully consistent with C pointers, with zero marshalling overhead. The compiler enables this optimization by default, transparent to the user.

#### 6.4 LLVM Representation of ?T Nullable Return Values

FFI nullable return values defined in [RFC-026](./026-ffi-core-mechanism.md) §7.6:

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

General LLVM representation: `{ i1, { i8* } }` — some flag + data.

**Optimization for FFI null pointers**: If the `T` in `?T` is an opaque type (internally a pointer), the compiler uses the **null pointer = None** optimization:

```llvm
; Optimized LLVM representation: directly use a nullable pointer
define i8* @__yx_sqlite3_open(...) {
    %raw = call i8* @sqlite3_open(...)
    ; null → None, non-null → Some (wrapped as opaque type)
    ret i8* %raw
}
```

Caller:
```llvm
%raw = call i8* @__yx_sqlite3_open(...)
%is_null = icmp eq i8* %raw, null
br i1 %is_null, label %none_branch, label %some_branch
```

This optimization makes FFI calls to `?SqliteDb` **zero additional overhead** — completely equivalent to C's null check.

#### 6.5 yx-bindgen Integration

The binding files automatically generated by [yx-bindgen](./026-ffi-core-mechanism.md) §6 are processed as ordinary YaoXiang source code at compile time. The compiler does not need to know that the code comes from bindgen — the handling of `native()` declarations and `unsafe {}` type definitions is completely consistent.

### 7. Destructor Code Generation

Aligned with the RAII semantics of [RFC-009](../accepted/009-ownership-model.md) and the `.drop` convention of [RFC-026](./026-ffi-core-mechanism.md) §7.

#### 7.1 .drop Binding Recognition

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

The compiler recognizes `.drop` bindings and marks the destructor function pointer in the type metadata.

#### 7.2 Cleanup Insertion at Scope End

```
User code:
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT ...")
    stmt.step()
    // ← Scope end
}

Cleanup inserted by the compiler (reverse order):
    call @sqlite3_finalize(%stmt)    // stmt.drop()
    call @sqlite3_close(%db)          // db.drop()
```

**Insertion locations**:
- Normal scope end (`}`)
- Early return (before `return`)
- `?` error propagation path (before `?`)
- spawn block end (destructor of variables within task)

#### 7.3 Move and Destructor

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: ownership transferred to db2
// db is invalidated, no drop inserted for db here
// ← Scope end: only drop inserted for db2
```

The compiler tracks Move semantics ([RFC-009](../accepted/009-ownership-model.md) §1), and only inserts destructor calls at the final holder of the variable.

#### 7.4 Destructor Failure Handling

```llvm
; debug mode: check destructor return value
%ret = call i32 @sqlite3_close(i8* %handle)
%ok = icmp eq i32 %ret, 0
br i1 %ok, label %done, label %panic
panic:
  call @__yx_panic("destructor failed")
  unreachable
done:
  ret void

; release mode: ignore return value
call i32 @sqlite3_close(i8* %handle)
ret void
```

### 8. Compilation Artifact Structure

The compilation artifact consists of the following components (specific struct definitions determined during implementation phase):

- **Machine code**: Object file (`.o`) compiled by LLVM, containing all function translation results
- **spawn metadata**: Task function pointers, dependencies, resource conflict serialization pairs for each spawn block
- **FFI symbol table**: External C symbol references (symbol name + whether weak reference)
- **Entry point table**: List of entry functions for the executable
- **Type information**: Reflection metadata, written to the `.reflect` section, mmap'd on demand by the runtime

### 9. Runtime Library

Aligned with [RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md), the runtime is linked into the final exe as a **static library**.

```
Final exe internal structure:

┌────────────────────────────────────────────┐
│  User Code (Native Machine Code)                       │
│  ├── Ordinary Functions (Sequential Execution)                    │
│  ├── spawn Block Expansion (Task Functions + Scheduling Calls)     │
│  ├── FFI Marshalling Wrapper Functions               │
│  └── RAII Destructor Code                          │
├────────────────────────────────────────────┤
│  Runtime Static Library (approx. 500KB-1MB, depending on platform and feature selection)  │
│  ├── Thread Pool (num_workers)                  │
│  ├── Event Loop (libuv / io_uring)           │
│  ├── Work-Stealing Queue (Full Runtime only)       │
│  ├── Memory Allocator (jemalloc / mimalloc)      │
│  └── Reflection Metadata (.reflect section, mmap on demand)    │
│                                              │
│  None of:                                      │
│  ❌ Bytecode Interpreter                             │
│  ❌ JIT Compiler                               │
│  ❌ GC                                      │
│  ❌ Virtual Machine                                    │
└────────────────────────────────────────────┘
```

**Key design**: Task identification and dependency analysis of spawn blocks are completed at compile time, and the runtime only does "create task → distribute to thread pool → wait for completion" — the data structures are fixed and the behavior is predictable.

> **Difference in size estimate from RFC-008**: RFC-008 §4 estimates the scheduler to be approximately 200-500KB, only including the task scheduling core. The 500KB-1MB estimate in this document additionally includes the memory allocator (jemalloc/mimalloc), event loop (libuv/io_uring), and reflection metadata section. The actual size depends on the platform and feature selection, and precise figures will be provided during the implementation phase.

**Relationship between the three-tier runtime and LLVM** (aligned with RFC-008 §1):

| Runtime | LLVM AOT Behavior |
|--------|---------------|
| **Embedded** | No spawn support, directly generates sequential machine code |
| **Standard** | Supports spawn blocks, DAG within spawn block + single-threaded scheduling (num_workers=1) |
| **Full** | Supports spawn blocks, DAG within spawn block + multi-threaded scheduling (num_workers>1), supports WorkStealing |

---

## Detailed Design

### Module Directory Structure

Aligned with the directory layout of [RFC-008](../accepted/008-runtime-concurrency-model.md) §6. The `[! Planned]` marker indicates that the file/directory has not yet been created and will be introduced during the implementation phase of this RFC.

```
src/
├── frontend/                          # Compilation frontend (shared by all backends)
│   ├── core/
│   │   ├── spawn/                     # spawn module (concurrency analysis shared by VM and LLVM backends)
│   │   │   ├── mod.rs                 # spawn module entry
│   │   │   ├── placement.rs           # spawn occurrence location legality check
│   │   │   └── analysis.rs            # [! Planned] Task identification, dependency analysis, resource conflict detection
│   │   └── typecheck/
│   │       └── ...
│
├── middle/
│   ├── core/
│   │   ├── ir.rs                      # IR definition (shared by VM and LLVM)
│   │   └── ir_gen.rs                  # IR generation
│   └── passes/
│       ├── codegen/
│       │   ├── mod.rs                 # Orchestration layer (currently outputs BytecodeFile)
│       │   ├── translator.rs          # IR → bytecode translation (for VM backend)
│       │   ├── emitter.rs             # Bytecode emission + jump backfilling (for VM backend)
│       │   ├── buffer.rs              # Constant pool + bytecode buffer (for VM backend)
│       │   ├── bytecode.rs            # Bytecode format definition + serialization (for VM backend)
│       │   ├── flow.rs                # Register allocation + label generation + symbol table (for VM backend)
│       │   └── operand.rs             # Operand parsing (for VM backend)
│       ├── lifetime/                  # Lifetime/token liveness analysis
│       └── mono/                      # Monomorphization
│
├── backends/
│   ├── common/                        # Shared value/heap/opcode
│   ├── interpreter/                   # Tree-walking interpreter (VM backend)
│   ├── llvm/                          # [! Planned] LLVM backend code generation (see file list below)
│   │   ├── mod.rs                     # [! Planned] LLVM backend entry
│   │   ├── context.rs                 # [! Planned] LLVM context management
│   │   ├── types.rs                   # [! Planned] Type mapping (YaoXiang → LLVM IR)
│   │   ├── values.rs                  # [! Planned] Value mapping
│   │   ├── func.rs                    # [! Planned] Function translation
│   │   ├── spawn.rs                   # [! Planned] spawn block expansion
│   │   ├── ffi.rs                     # [! Planned] FFI call code generation
│   │   └── drop.rs                    # [! Planned] Destructor function insertion
│   └── runtime/                       # Compiled runtime (static library linked into exe)
│       ├── engine.rs                  # Task scheduling engine
│       ├── facade.rs                  # External interface
│       └── task.rs                    # Task representation
│
└── util/
    └── diagnostic/                    # Error diagnostics (shared)
```

> **Key change**: spawn block analysis (task identification, dependency analysis, resource conflict detection) will be implemented in `frontend/core/spawn/` (shared frontend). The existing `frontend/core/typecheck/passes/spawn_placement.rs` (spawn occurrence location check) will be migrated to `frontend/core/spawn/placement.rs`, see RFC-024 for details. The LLVM backend only consumes the analysis results and generates the corresponding scheduling code.
>
> **Current status**: The current `middle/passes/codegen/` `buffer.rs`, `emitter.rs`, `bytecode.rs`, `flow.rs`, `operand.rs` serve the VM backend's bytecode generation (`CodegenContext::generate()` → `BytecodeFile`). The LLVM backend will be implemented in `backends/llvm/`, at the same level as the interpreter backend and runtime — both share the same `ModuleIR` input and output different target formats (bytecode vs native code).

### Platform ABI Support

| Platform | Target Triple | Output Format | Calling Convention (FFI default) |
|------|-----------|----------|---------------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | ELF | System V AMD64 |
| macOS x86_64 | `x86_64-apple-darwin` | Mach-O | System V AMD64 |
| macOS ARM64 | `aarch64-apple-darwin` | Mach-O | ARM64 AAPCS |
| Windows x86_64 | `x86_64-pc-windows-msvc` | COFF | Microsoft x64 |

FFI calls default to the platform's C calling convention. Users can override with `native("symbol", cc = "stdcall")` and other options (aligned with future extensions of [RFC-026](./026-ffi-core-mechanism.md)).

### Floating-Point Semantics Consistency (VM ↔ LLVM)

The core promise of the dual-backend architecture is that the behavior of VM (development and debugging) and LLVM (production release) is consistent. Floating-point operations have potential inconsistencies between the two execution modes:

| Scenario | Risk | Strategy |
|------|------|------|
| NaN propagation | VM and LLVM may handle NaN's sign bit and payload differently | Compiler normalizes NaN representation at the IR level; NaN comparisons uniformly use `fcmp uno` |
| Rounding mode | LLVM defaults to round-to-nearest-even, VM depends on host CPU | Non-default rounding modes not exposed; VM and LLVM uniformly use RTNE |
| Division by zero | IEEE 754 defines ±Inf, but some platforms may trap | debug mode checks division by zero and reports diagnostic; release mode follows IEEE 754 |
| `-0.0` vs `+0.0` | Comparison operations may not be equivalent | Uniformly use IEEE 754 rules: `+0.0 == -0.0` |
| Denormalized numbers | Some platforms flush-to-zero | LLVM does not enable `denormal-fp-math` attribute, preserving full IEEE 754 semantics |

> **Testing strategy**: Implement a set of cross-backend floating-point consistency test suites — the same YaoXiang source code is executed on the VM and LLVM backends respectively, and outputs are compared value by value. This set of tests is a mandatory gate for CI.

---

## Trade-offs

### Advantages

1. **Performance**: AOT compilation is 10-100x faster than interpreted execution
2. **Unified frontend**: VM and LLVM share the same frontend, with completely consistent behavior
3. **Zero scheduling overhead**: Ordinary code is directly generated as sequential machine code, no DAG overhead outside spawn blocks
4. **Static linking**: No external runtime dependencies, single exe is deployable
5. **Zero GC**: RAII deterministic destructor, no pauses
6. **Zero FFI overhead**: `?T` null pointer optimization, opaque type layout optimization, FFI call cost equivalent to C
7. **Compile-time analysis**: Task identification and dependency analysis of spawn blocks are completed at compile time, runtime only executes

### Disadvantages

1. **LLVM integration complexity**: Requires deep understanding of inkwell API and LLVM IR
2. **Compilation time**: AOT compilation is slower than interpreter (one-time cost)
3. **Debugging experience**: Native code debugging requires DWARF/PDB symbol support (compiler needs to generate debug information)
4. **Incremental compilation**: Additional design needed for incremental compilation of large projects
5. **Floating-point semantics consistency**: VM and LLVM may have differences in edge behaviors such as NaN propagation, rounding mode, and division by zero; dual-backend behavior consistency needs to be guaranteed through normalization strategies (see §10)

### Consistency with Related RFCs

| RFC | Consistency |
|-----|--------|
| RFC-024 spawn block concurrency model | ✅ spawn block direct subexpression → task dispatch |
| RFC-008 runtime architecture | ✅ Dual backend + scheduler static library + module directory structure |
| RFC-009 ownership model v9 | ✅ `&T`/`&mut T` tokens (zero-sized), `ref T` (fat pointer), `?T` (Option) |
| RFC-026 FFI core mechanism | ✅ `native()` → declare + marshalling, `.drop` → RAII cleanup |

---

## Alternatives

| Plan | Description | Why not chosen |
|------|------|-----------|
| Interpreter only | No AOT needed | Insufficient performance |
| Pure static compilation (no runtime) | No scheduler linked | spawn blocks require runtime task scheduling |
| Cranelift backend | Faster compilation speed | Runtime performance not as good as LLVM, as a future optional backend |
| Link external LLVM runtime | Use LLVM's built-in runtime | Introduces unnecessary dependencies |

---

## Implementation Strategy

### Phase Division

#### Phase 1: Basic Framework
- [ ] Add inkwell dependency
- [ ] Implement LLVM context initialization (`context.rs`)
- [ ] Implement basic type mapping (`types.rs`)

#### Phase 2: Function Translation
- [ ] Implement function declaration translation (`func.rs`)
- [ ] Implement basic instruction translation (arithmetic, control flow, calls) (`translator.rs`)
- [ ] Implement value mapping (`values.rs`)

#### Phase 3: Ownership Type Translation
- [ ] Implement `&T`/`&mut T` tokens (zero-sized, disappear after compilation)
- [ ] Implement `ref T` (fat pointer `{ i64*, T* }`)
- [ ] Implement `?T` (`{ i1, T }` tagged union)
- [ ] Implement `List(T)` (`{ T*, i64, i64 }`)
- [ ] Implement Move semantics tracking (used for destructor insertion judgment)

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
- [ ] `?T` null pointer optimization (FFI specific)

#### Phase 6: Destructor Code Generation
- [ ] `.drop` binding recognition
- [ ] Scope end cleanup insertion (reverse order) (`drop.rs`)
- [ ] Early return path cleanup
- [ ] `?` error propagation path cleanup

#### Phase 7: Runtime Library Linking
- [ ] Implement `runtime_task_spawn` / `runtime_task_wait_all` and other runtime functions
- [ ] Link runtime static library
- [ ] End-to-end integration testing

### Dependencies

- RFC-024 (spawn block concurrency) → Phase 4 input
- RFC-009 v9 (ownership) → Phase 3, 6 input
- RFC-008 (runtime architecture) → Phase 7 input
- RFC-026 (FFI mechanism) → Phase 5 input

---

## Related Work

### Lazy Task Creation (1990)[^1]

| Attribute | Description |
|------|------|
| Institution | MIT |
| Authors | James R. Larus, Robert H. Halstead Jr. |
| Core | Lazy creation of child tasks, created on demand |
| Reference value | Theoretical basis for on-demand scheduling of tasks within spawn blocks |

**Core idea**: Instead of creating tasks immediately, creation is delayed. When the parent task needs the child's value, the child task is created. This solves the performance overhead problem of fine-grained parallel tasks[^1]. YaoXiang's spawn block scheduling draws on this idea — tasks are identified at compile time, but distributed to the thread pool on demand at runtime.

### Lazy Scheduling (2014)[^2]

| Attribute | Description |
|------|------|
| Institution | University of Maryland |
| Authors | Tzannes, Caragea |
| Core | Runtime adaptive scheduling, no extra state |
| Reference value | Reference for Full Runtime WorkStealing scheduler design |

### SISAL Language[^3]

| Attribute | Description |
|------|------|
| Institution | Lawrence Livermore National Laboratory (LLNL) |
| Core | Single-assignment language, Dataflow graph, implicit parallelism |
| Reference value | Proof of feasibility of Dataflow model in industrial-grade applications |

**Key difference**: SISAL's parallelism is **implicit** — the language has single-assignment semantics, and the compiler automatically analyzes the data dependency graph of the entire program to determine parallelism. YaoXiang's parallelism is **explicit** — users use `spawn {}` blocks to mark parallel regions, and the compiler only analyzes dependencies within spawn blocks. This avoids the complexity of whole-program analysis in SISAL while preserving user control over parallel behavior.

### Mul-T Parallel Scheme[^4]

| Attribute | Description |
|------|------|
| Institution | MIT |
| Core | Future construct, Lazy Task Creation implementation |
| Reference value | Specific implementation reference |

### Comparison Summary

| Technology | Lazy Creation | Parallel Marker | Analysis Scope | Ownership |
|------|----------|----------|----------|--------|
| Lazy Task Creation[^1] | ✅ | Implicit | Whole program | N/A |
| Lazy Scheduling[^2] | ✅ | Implicit | Whole program | N/A |
| SISAL[^3] | ✅ | Implicit (single-assignment) | Whole program | N/A |
| Mul-T[^4] | ✅ | Explicit (future) | Call site | N/A |
| **YaoXiang** | ✅ | **Explicit (spawn block)** | **Within spawn block** | **✅ (Move + Token + ref)** |

**YaoXiang's innovation**: Elevates the parallel marker from "every function call" (future) to "structured block" (spawn), users write ordinary code and place spawn blocks where parallelism is needed. The analysis scope is constrained within the spawn block, compilation is efficient and behavior is controllable.

---

## Appendix

### Appendix A: Comparison with Rust async

| Feature | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| Compilation artifact | State machine + machine code | Machine code + spawn task metadata |
| Runtime | tokio | Statically linked scheduler (approx. 500KB-1MB) |
| Concurrency marker | async/await keywords | `spawn { }` block |
| Task creation | State machine generated at compile time | Direct subexpression identified at compile time → task function |
| Color function | async infection | **No function coloring** |
| Synchronous wait | `.await` | spawn block automatically synchronously blocks |
| Memory management | GC (runtime) | **RAII (deterministic)** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler automatically selects Rc/Arc)** |

### Appendix B: Design Decision Record

| Decision | Resolution | Date |
|------|------|------|
| Adopt LLVM AOT | Direct Codegen, no over-abstraction | 2026-02-15 |
| Concurrency model alignment | Aligned with RFC-024 spawn block direct subexpression model | 2026-06-10 |
| DAG analysis scope | Within spawn block, not across spawn blocks (aligned with RFC-024) | 2026-06-05 |
| Ownership model alignment | Aligned with RFC-009 v9: `&T`/`&mut T` tokens + `ref` keyword | 2026-06-10 |
| Dual backend model | VM (development) + LLVM (production), aligned with RFC-008 | 2026-05-11 |
| Scheduler form | Static library linked into exe, approx. 500KB-1MB (depending on platform and features), no GC | 2026-05-11 |
| FFI code generation | Integrate RFC-026: `native()` declare + marshalling | 2026-06-10 |
| Destructor | `.drop` → RAII cleanup insertion, aligned with RFC-026 §7 | 2026-06-10 |
| Side effect handling | Remove `@IO`/`@Pure` inference, switch to RFC-024 resource type | 2026-06-10 |
| Reflection metadata | Compile into exe .reflect section, mmap on demand | 2026-05-11 |
| Paper citations | Retain Lazy Task Creation etc., clarify differences from YaoXiang | 2026-02-16 |

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
| **Draft** | `docs/design/rfc/` | Author's draft, waiting to be submitted for review |
| **Under Review** | `docs/design/rfc/review/` | Open community discussion and feedback |
| **Accepted** | `docs/design/rfc/accepted/` | Becomes a formal design document |
| **Rejected** | `docs/design/rfc/` | Remains in RFC directory |

> Current status: **Accepted** — Aligned with RFC-024 spawn block concurrency model, RFC-009 v9 ownership model, and RFC-026 FFI mechanism