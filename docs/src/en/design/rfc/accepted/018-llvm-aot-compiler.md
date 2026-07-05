---
title: "RFC-018: LLVM AOT Compiler Design"
status: "Accepted"
author: "Chen Xu"
created: "2026-02-15"
updated: "2026-07-05 (synced with GitHub Issue #14, #134; added implementation status analysis)"
issue: "#14"
tracking_issue: "https://github.com/ChenXu233/YaoXiang/issues/134"
---

# RFC-018: LLVM AOT Compiler Design

> **References**:
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
> - [RFC-008: Decoupling Runtime Concurrency Model from Scheduler](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
> - [RFC-026: FFI Core Mechanism](./026-ffi-core-mechanism.md)
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)

> **Deprecated**:
> - Old "bottom-up automatic DAG analysis" model — replaced by RFC-024's spawn block direct sub-expression model
> - `@IO`/`@Pure` implicit side effect inference — replaced by RFC-024's resource type mechanism
> - `Arc(T)` type mapping — replaced by RFC-009 v9 `ref` keyword

## Summary

This document designs the LLVM AOT (Ahead-of-Time) compiler for the YaoXiang language. The LLVM backend and the VM backend (interpreter) share the same compilation frontend, forming the dual-backend architecture defined in [RFC-008](../accepted/008-runtime-concurrency-model.md): the VM is used for development and debugging, while LLVM is used for production releases.

**Core Responsibilities**:

```
Source → Frontend (shared) → IR → LLVM Codegen → .o → Link scheduler static lib → exe
```

The compiler translates YaoXiang source code into native machine code, where:

| Language Feature | Compilation Strategy |
|----------|----------|
| Normal code | Sequential machine code, zero scheduling overhead |
| `spawn { }` block | Direct sub-expressions → task dispatch + synchronous wait (aligned with [RFC-024](../accepted/024-concurrency-model.md)) |
| `native("symbol")` | LLVM `declare external` + argument marshalling (aligned with [RFC-026](./026-ffi-core-mechanism.md)) |
| `.drop` destructor | RAII cleanup code insertion (aligned with [RFC-009](../accepted/009-ownership-model.md)) |
| `&T` / `&mut T` tokens | Zero-sized types, disappear after compilation |
| `ref T` shared | `{ refcount_ptr, data_ptr }` fat pointer, compiler automatically selects Rc/Arc |

**Relationship with RFC-024**: RFC-024 defines the **user semantics** of spawn blocks (direct sub-expressions create tasks, synchronous blocking wait). This document defines **how these semantics are compiled to machine code**.

**Relationship with RFC-026**: RFC-026 defines the **user syntax** of FFI (`native()`, `[0]` method binding, `.drop`). This document defines **how FFI calls generate LLVM IR**.

---

## Motivation

### Why an LLVM AOT Compiler?

Currently YaoXiang only has an interpreter as the execution backend:

| Problem | Impact |
|------|------|
| Performance bottleneck | Interpretation is 10-100x slower than machine code |
| Deployment complexity | Need to carry the interpreter and runtime |
| Production environment | Interpreter is not suitable for performance-sensitive scenarios |

### LLVM in the Dual-Backend Model

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 defines the dual-backend architecture:

```
                    ┌─────────────────────┐
                    │  Compilation Frontend (unified) │
                    │  Lexer → Parser     │
                    │  → TypeCheck        │
                    │  → spawn analysis   │
                    │  → Escape analysis  │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │   VM Backend (Development) │     │  LLVM Backend (Production)  │
      │   IR → Interpretation │     │  IR → Native Code      │
      │   Step debugging     │     │  Link scheduler static lib   │
      │   Rapid iteration    │     │  Output .exe         │
      └───────────────────┘     └───────────────────┘
```

The **behavior of the two backends is completely identical** — the only difference is the execution method. The same source code, the same type checking, the same spawn analysis results.

---

## Proposal

### 1. Compiler Architecture

The LLVM backend sits at the last stage of the compilation pipeline, receiving IR from the frontend and generating native code:

```
Source code
  → Lexer / Parser (frontend/core/)
  → TypeCheck + spawn analysis (frontend/core/typecheck/)
  → IR generation (middle/core/ir_gen.rs)
  → LLVM Codegen (backends/llvm/)
      ├── Type mapping: YaoXiang types → LLVM IR types
      ├── Function translation: IR instructions → LLVM IR instructions
      ├── spawn expansion: direct sub-expressions → task functions + dispatch calls
      ├── FFI expansion: native() calls → declare + marshalling
      └── Destructor insertion: end of scope → .drop() calls
  → LLVM optimization + target code generation
  → Link runtime static lib → executable
```

### 2. Compilation Flow

```
Phase 1: Frontend (shared with VM backend)
  - Parsing, type checking, spawn block analysis, escape analysis
  - Output: IR with type annotations

Phase 2: LLVM IR generation
  - Type mapping, function declarations, instruction translation
  - Output: LLVM Module

Phase 3: LLVM optimization
  - Standard LLVM optimization pipeline (O0/O1/O2/O3)
  - Inlining, constant folding, dead code elimination

Phase 4: Target code generation
  - LLVM TargetMachine → .o files
  - Platforms: Linux (ELF), macOS (Mach-O), Windows (COFF)

Phase 5: Linking
  - Link runtime static lib (scheduler, allocator)
  - Output: executable
```

### 3. Type Mapping

#### 3.1 YaoXiang → LLVM IR Type Mapping

| YaoXiang Type | LLVM IR Type | Notes |
|---------------|-------------|------|
| `Int` | `i64` | Default 64-bit signed integer |
| `Int32` | `i32` | Explicit 32-bit integer (mainly for FFI) |
| `Float` | `f64` | Default 64-bit float |
| `Float32` | `f32` | Explicit 32-bit float (mainly for FFI) |
| `Bool` | `i1` | Boolean value |
| `Char` | `i32` | Unicode code point |
| `String` | `{ i8*, i64 }` | Pointer + byte length |
| `Void` | `{}` | Zero-sized empty type |
| `&T` | — | Zero-sized token, disappears after compilation, generates no IR |
| `&mut T` | — | Zero-sized token, disappears after compilation, generates no IR |
| `ref T` | `{ i64*, T* }` | Fat pointer (reference count pointer + data pointer) |
| `*T` | `T*` | Raw pointer |
| `[T; N]` | `[N x T]` | Fixed-length array |
| `List(T)` | `{ T*, i64, i64 }` | Data pointer + length + capacity |
| Struct | Corresponding LLVM struct | Fields laid out in declaration order |
| Record enum | `{ i64, [max_payload_size] }` | Tag + union of max payload size |
| `?T` | `{ i1, T }` | Has-value tag + data (general representation) |
| FFI opaque type | `{ i8* }` | Wrapped C pointer |
| Function pointer | `T (...)*` | Function pointer type |

> **Zero runtime overhead for `&T` / `&mut T`**: [RFC-009](../accepted/009-ownership-model.md) §2.7 defines that the compiler internally assigns a brand identifier (compile-time unique integer) to tokens. After monomorphization and inlining, the brand completely disappears — no token traces exist in the generated machine code.

#### 3.2 FFI Argument Type Mapping

Aligned with [RFC-026](./026-ffi-core-mechanism.md) §2.2, with an added LLVM IR column:

| C Type | YaoXiang Type | LLVM IR | Notes |
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

### 4. IR Normalization and Instruction Translation

#### 4.0 IR Normalization (Stack → Register)

The current IR (`src/middle/core/ir.rs`) contains stack manipulation instructions (`Push`/`Pop`/`Dup`/`Swap`), which are designed for the bytecode VM. LLVM IR is in SSA form and does not accept stack operations.

**Handling Strategy**: The LLVM path passes through a lightweight normalization pass before instruction translation:

| Stack Instruction | Normalization Strategy |
|--------|-----------|
| `Push(r)` | Record `stack.push(r)`, generates no IR |
| `Pop(r)` | `r = stack.pop()`, generates `load` (from stack slot) |
| `Dup` | `stack.push(stack.top())`, generates no IR |
| `Swap` | Swap the top two elements of the stack, generates no IR |

After normalization, all operands become register/local variable references, and all stack operations are eliminated. This pass executes as the first step in `translator.rs`.

> **Why not eliminate stack instructions at the IR level?** Because the VM backend needs stack semantics. Normalizing at the LLVM translation entry point keeps the IR shared between two backends — each backend consumes the same IR according to its own needs.
>
> **Prerequisite**: The IR generation phase guarantees stack balance — all control flow paths arrive at the same program point with consistent stack depth (the VM bytecode backend relies on the same prerequisite, otherwise bytecode execution will fail). The normalization pass does not check this prerequisite; violations result in undefined behavior in the LLVM backend.

#### 4.1 Instruction Translation Table

The LLVM IR translation strategy for each variant in the `Instruction` enum is listed below. Instruction names match `src/middle/core/ir.rs` exactly.

**Arithmetic Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `Add { dst, lhs, rhs }` | `add` (integer) / `fadd` (float) | Select integer or float addition by type |
| `Sub { dst, lhs, rhs }` | `sub` / `fsub` | |
| `Mul { dst, lhs, rhs }` | `mul` / `fmul` | |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv` | Signed/unsigned/float division |
| `Mod { dst, lhs, rhs }` | `srem` / `urem` | Signed/unsigned modulo |
| `Neg { dst, src }` | `sub 0, src` (integer) / `fneg` (float) | |

**Bitwise Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `And { dst, lhs, rhs }` | `and` | |
| `Or { dst, lhs, rhs }` | `or` | |
| `Xor { dst, lhs, rhs }` | `xor` | |
| `Shl { dst, lhs, rhs }` | `shl` | Left shift |
| `Shr { dst, lhs, rhs }` | `lshr` | Logical right shift |
| `Sar { dst, lhs, rhs }` | `ashr` | Arithmetic right shift |

**Comparison Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp oeq` | |
| `Ne { dst, lhs, rhs }` | `icmp ne` / `fcmp one` | |
| `Lt { dst, lhs, rhs }` | `icmp slt` / `fcmp olt` | |
| `Le { dst, lhs, rhs }` | `icmp sle` / `fcmp ole` | |
| `Gt { dst, lhs, rhs }` | `icmp sgt` / `fcmp ogt` | |
| `Ge { dst, lhs, rhs }` | `icmp sge` / `fcmp oge` | |

**Control Flow Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `Jmp(label)` | `br label %L` | Unconditional jump |
| `JmpIf(cond, label)` | `br i1 %cond, label %L, label %fallthrough` | Conditional jump |
| `JmpIfNot(cond, label)` | `br i1 %cond, label %fallthrough, label %L` | Conditional not-jump |
| `Ret(Some(v))` | `ret T %v` | With return value |
| `Ret(None)` | `ret void` | No return value |

**Call Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `Call { dst, func, args }` | `%r = call T @func(...)` | Static call |
| `CallVirt { dst, obj, method_name, args }` | vtable GEP + `call` (function pointer) | Virtual method call, looked up via vtable |
| `CallDyn { dst, func, args }` | `%r = call T %func(...)` | Dynamic call (closure/function pointer) |
| `TailCall { func, args }` | `musttail call` / `tail call` | Tail call optimization |

**Memory Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `Move { dst, src }` | — | After normalization becomes register copy, SSA construction can eliminate most |
| `Load { dst, src }` | `%v = load T, T* %src` | |
| `Store { dst, src }` | `store T %src, T* %dst` | |
| `Alloc { dst, size }` | `%p = alloca T` (stack) / `call @malloc` (escape to heap) | Escape analysis determines allocation location |
| `Free(ptr)` | `call @free(%ptr)` (heap) / — (stack, auto-reclaimed) | |
| `AllocArray { dst, size, elem_size }` | `%p = alloca [N x T]` (stack) / `call @malloc` (heap) | |

**Struct/Array Access Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `LoadField { dst, src, field }` | `%ptr = getelementptr T, T* %src, 0, field` + `load` | |
| `StoreField { dst, field, src }` | `%ptr = getelementptr T, T* %dst, 0, field` + `store` | |
| `LoadIndex { dst, src, index }` | `%ptr = getelementptr T, T* %src, 0, %index` + `load` | |
| `StoreIndex { dst, index, src }` | `%ptr = getelementptr T, T* %dst, 0, %index` + `store` | |
| `CreateStruct { dst, type_name, fields }` | `insertvalue` chain | Construct LLVM struct in field order |

**Type Conversion Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `Cast { dst, src, target_type }` | `bitcast` / `trunc` / `zext` / `sext` / `fptrunc` / `fpext` / `sitofp` / `fptosi` / `inttoptr` / `ptrtoint` | Choose the appropriate cast instruction based on source/target type combination |
| `TypeTest(val, type)` | — | Compile-time type test, generates `icmp eq` to compare type tags |

**Ownership and Borrowing Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `Borrow { dst, src, mutable }` | — | **Zero-sized token, completely disappears after compilation**, generates no IR |
| `Release(val)` | — | **Zero-sized token, completely disappears after compilation** |
| `Move { dst, src }` | — | Ownership transfer, becomes register copy after normalization |
| `Drop(val)` | `call void @T.drop(T* %val)` | Call the type's destructor (see §7) |
| `ShareRef { dst, src }` | `call %T* @Arc_new(%src)` / `call %T* @Rc_new(%src)` | Compiler automatically selects Arc/Rc based on cross-thread usage |
| `ArcNew { dst, src }` | `call %T* @Arc_new(%src)` | Atomic reference count = 1 |
| `ArcClone { dst, src }` | `call %T* @Arc_clone(%src)` | Atomically increment reference count |
| `ArcDrop(val)` | `call void @Arc_drop(%val)` | Atomically decrement + conditional release |

**Concurrency Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `Spawn { closures, plan, result }` | Expands to scheduler call sequence | See §5, runtime `task_spawn` + `task_wait_all` |
| `Yield` | — | On the AOT path, spawn blocks wait synchronously, no yield needed; ignored |

**unsafe Blocks and Raw Pointer Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `UnsafeBlockStart` | — | **Compile-time marker, generates no IR** |
| `UnsafeBlockEnd` | — | **Compile-time marker, generates no IR** |
| `PtrFromRef { dst, src }` | `%p = ptrtoint T* %src to i64` (or copy the pointer directly) | |
| `PtrDeref { dst, src }` | `%v = load T, T* %src` | |
| `PtrStore { dst, src }` | `store T %src, T* %dst` | |
| `PtrLoad { dst, src }` | `%v = load T, T* %src` | |

**String Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `StringLength { dst, src }` | `%len = extractvalue { i8*, i64 } %src, 1` | String is `{ ptr, len }`, length is in field 1 |
| `StringConcat { dst, lhs, rhs }` | `call String @yx_string_concat(%lhs, %rhs)` | Runtime helper function |
| `StringGetChar { dst, src, index }` | `getelementptr` + `load i32` | Includes bounds check |
| `StringFromInt { dst, src }` | `call String @yx_string_from_int(%src)` | Runtime helper function |
| `StringFromFloat { dst, src }` | `call String @yx_string_from_f64(%src)` | Runtime helper function |

**Closure Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `MakeClosure { dst, func: String, env }` | Allocate closure struct + populate function pointer (lookup by function name) and environment | `{ fn_ptr, env_fields... }` |
| `LoadUpvalue { dst, upvalue_idx }` | `%v = extractvalue %env, upvalue_idx` | Read upvalue from closure environment |
| `StoreUpvalue { src, upvalue_idx }` | `%env = insertvalue %env, %src, upvalue_idx` | Write upvalue to closure environment |
| `CloseUpvalue(val)` | Copy stack upvalue to heap | |

**Other Instructions**:

| IR Instruction | LLVM IR | Notes |
|---------|---------|------|
| `HeapAlloc { dst, type_id }` | `call i8* @malloc(i64 size)` + type tag write | Heap allocation + type info |
| `NewDict { dst, keys, values }` | `call Dict @yx_dict_new(%keys, %values)` | Runtime helper function |

> **Note**: `Push`/`Pop`/`Dup`/`Swap` have been eliminated during the §4.0 normalization phase and do not appear in the translation table. `Borrow`/`Release` are zero-sized compile-time tokens and generate no machine code.

### 5. spawn Block Code Generation

Aligned with [RFC-024](../accepted/024-concurrency-model.md), compilation of spawn blocks proceeds in the following steps.

#### 5.1 Semantic Recap

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // direct sub-expression → task 1
    t2 = fetch("url2"),   // direct sub-expression → task 2
    return (t1, t2)       // synchronous wait, assemble result
}
```

**Rules** (RFC-024 §2.1):
- The **direct sub-expressions** of a spawn block (top-level comma-separated statements) create parallel tasks
- Expressions inside nested `{}` are not direct sub-expressions and do not become independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning

#### 5.2 Compilation Steps

```
Step 1: Identify direct sub-expressions
  Traverse the spawn block body, collect top-level statements

Step 2: Dependency analysis
  For each direct sub-expression, analyze which variables generated by previous tasks it references
  No dependency → can be scheduled in parallel immediately
  Has dependency → queue and wait for the dependent task

Step 3: Resource conflict detection (RFC-024 §2.5)
  Check whether the same resource type instance is used by multiple tasks
  Same-instance conflict → mark for serial execution order

Step 4: Generate task functions
  Each direct sub-expression generates a separate LLVM function (closure)

Step 5: Generate dispatch code
  Call the runtime scheduler's task_spawn / task_wait

Step 6: Result assembly
  Collect all task outputs, assemble the return tuple
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
    data = fetch("url"),       // task 1: no dependency
    processed = parse(data),   // task 2: depends on task 1's data
    return processed
}
```

The compiler detects that `parse(data)` references `data` produced by task 1, and marks the dependency when generating dispatch code:

```llvm
; Task 2 is created with a dependency on task 1
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
;                                                              ↑
;                                                 depends on task 0 (fetch) completion
```

#### 5.5 Resource Type Automatic Serialization

[RFC-024 §2.5](../accepted/024-concurrency-model.md) defines that resource types (`FilePath`, `HttpUrl`, `DBUrl`, `Console` and user-defined resource types) are automatically serialized within spawn blocks:

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // uses SqliteDb (resource type)
    r2 = db.exec("INSERT ...")    // same instance → auto-serialized
}
```

The compiler detects that the same resource instance is used by two tasks, generating a serial dependency:

```llvm
; Task 2 depends on task 1 (same resource auto-serialized)
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

> ⚠️ **Dependency Note**: The **architecture** of FFI code generation defined in this section (`native("x")` → `declare external @x` → marshalling wrapper function → call) is stable and does not change with RFC-026 syntax changes. The specific argument marshalling rules table (§6.2) and opaque type layout (§6.3) reference RFC-026's definitions — if RFC-026's `native()` syntax or marshalling rules change, only the corresponding mapping tables in this document need to be updated; the architecture layer is unaffected. RFC-026 current status: **Under Review**, in the same `review/` directory as this document.
>
> **Pre-acceptance Condition**: Before this RFC is accepted, the parts of RFC-026 related to §6 of this document (`native()` declaration syntax, argument marshalling rules, opaque type `{ i8* }` layout, `.drop` binding conventions) should be frozen first or accepted together with RFC-026. Otherwise, the mapping tables in §6.2/§6.3/§7 may become outdated before implementation.

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

**Key Points**:
- `native("sqlite3_open")` → `declare external @sqlite3_open`
- Compiler automatically generates the marshalling wrapper function
- The wrapper function's signature uses YaoXiang types, internally converting to C types

#### 6.2 Argument Marshalling

| Direction | Conversion |
|------|------|
| YaoXiang `String` → C `char*` | Extract the `.ptr` field and pass it |
| YaoXiang `Int32` → C `int` | Pass directly (`i32`) |
| YaoXiang `*Void` → C `void*` | Pass directly (`i8*`) |
| YaoXiang `T` (transparent type) → C `struct T*` | Take address and pass |
| YaoXiang `T` (opaque type) → C `struct T*` | Extract the pointer from `{ i8* }` and pass |

#### 6.3 LLVM Layout of Opaque Types

[RFC-026](./026-ffi-core-mechanism.md) §4.1 defines opaque types:

```yaoxiang
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}
```

LLVM layout: `{ i8* }` — a struct containing a C pointer.

**Layout Optimization**: When an opaque type has only a single `handle: *Void` field, it can be optimized to use `i8*` directly (omitting the outer struct). The optimized ABI is fully consistent with a C pointer, with zero marshalling overhead. This optimization is enabled by default, transparent to the user.

#### 6.4 LLVM Representation of ?T Nullable Return Values

[RFC-026](./026-ffi-core-mechanism.md) §7.6 defines FFI nullable return values:

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

General LLVM representation: `{ i1, { i8* } }` — has-value tag + data.

**Optimization for FFI null pointer**: If the `T` in `?T` is an opaque type (internally a pointer), the compiler uses a **null pointer = None** optimization:

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

This optimization makes FFI calls of `?SqliteDb` have **zero additional overhead** — fully equivalent to a C null check.

#### 6.5 yx-bindgen Integration

The bindings file automatically generated by [yx-bindgen](./026-ffi-core-mechanism.md) §6 is processed as ordinary YaoXiang source code during compilation. The compiler does not need to know that the code came from bindgen — `native()` declarations and `unsafe {}` type definitions are handled in exactly the same way.

### 7. Destructor Code Generation

Aligned with the RAII semantics of [RFC-009](../accepted/009-ownership-model.md) and the `.drop` convention in [RFC-026](./026-ffi-core-mechanism.md) §7.

#### 7.1 .drop Binding Identification

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

The compiler identifies `.drop` bindings and marks the destructor function pointer in the type metadata.

#### 7.2 Cleanup Insertion at End of Scope

```
User code:
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT ...")
    stmt.step()
    // ← end of scope
}

Cleanup inserted by the compiler (reverse order):
    call @sqlite3_finalize(%stmt)    // stmt.drop()
    call @sqlite3_close(%db)          // db.drop()
```

**Insertion Points**:
- Normal scope end (`}`)
- Early return (before `return`)
- `?` error propagation path (before `?`)
- spawn block end (destructor of variables within a task)

#### 7.3 Move and Destructor

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: ownership transferred to db2
// db is now invalid, no drop is inserted for db here
// ← end of scope: drop is only inserted for db2
```

The compiler tracks Move semantics ([RFC-009](../accepted/009-ownership-model.md) §1), inserting destructor calls only at the final holder of the variable.

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

The compilation artifact contains the following components (the specific struct definitions will be determined during the implementation phase):

- **Machine code**: LLVM-compiled object files (`.o`), containing all function translation results
- **spawn metadata**: Task function pointers, dependency relationships, resource conflict serialization pairs for each spawn block
- **FFI symbol table**: External C symbol references (symbol name + whether weakly referenced)
- **Entry point table**: List of entry functions for the executable
- **Type information**: Reflection metadata, written to the `.reflect` section, mmap'd on demand at runtime

### 9. Runtime Library

Aligned with [RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md), the runtime is linked into the final exe as a **static library**.

```
Final exe internal structure:

┌────────────────────────────────────────────┐
│  User code (native machine code)                       │
│  ├── Normal functions (sequential execution)                    │
│  ├── spawn block expansion (task functions + dispatch calls)     │
│  ├── FFI marshalling wrapper functions               │
│  └── RAII destructor code                          │
├────────────────────────────────────────────┤
│  Runtime static lib (approx 500KB-1MB, depending on platform and feature selection)  │
│  ├── Thread pool (num_workers)                  │
│  ├── Event loop (libuv / io_uring)           │
│  ├── Work-stealing queue (Full Runtime only)          │
│  ├── Memory allocator (jemalloc / mimalloc)      │
│  └── Reflection metadata (.reflect section, mmap on demand)    │
│                                              │
│  Not included:                                      │
│  ❌ Bytecode interpreter                             │
│  ❌ JIT compiler                               │
│  ❌ GC                                      │
│  ❌ Virtual machine                                    │
└────────────────────────────────────────────┘
```

**Key Design**: Task identification and dependency analysis of spawn blocks are completed at compile time; at runtime, only "create task → dispatch to thread pool → wait for completion" is performed — data structures are fixed, behavior is predictable.

> **Difference from RFC-008 size estimate**: RFC-008 §4 estimates the scheduler at approximately 200-500KB, containing only the task scheduling core. The 500KB-1MB estimate in this document additionally includes the memory allocator (jemalloc/mimalloc), event loop (libuv/io_uring), and reflection metadata section. The actual size depends on the platform and feature selection; precise numbers will be provided during the implementation phase.

**Relationship between Three-Tier Runtime and LLVM** (aligned with RFC-008 §1):

| Runtime | LLVM AOT Behavior |
|--------|---------------|
| **Embedded** | No spawn support, generates sequential machine code directly |
| **Standard** | Supports spawn blocks, DAG within spawn block + single-threaded scheduling (num_workers=1) |
| **Full** | Supports spawn blocks, DAG within spawn block + multi-threaded scheduling (num_workers>1), supports WorkStealing |

---

## Detailed Design

### Module Directory Structure

Aligned with the directory layout in [RFC-008](../accepted/008-runtime-concurrency-model.md) §6. The `[! Planned]` tag indicates that the file/directory has not yet been created and will be introduced during the implementation phase of this RFC.

```
src/
├── frontend/                          # Compilation frontend (shared by all backends)
│   ├── core/
│   │   ├── spawn/                     # spawn module (concurrency analysis shared by VM and LLVM backends)
│   │   │   ├── mod.rs                 # spawn module entry
│   │   │   ├── placement.rs           # spawn occurrence legality check
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
│       │   ├── emitter.rs             # Bytecode emission + jump back-patching (for VM backend)
│       │   ├── buffer.rs              # Constant pool + bytecode buffer (for VM backend)
│       │   ├── bytecode.rs            # Bytecode format definition + serialization (for VM backend)
│       │   ├── flow.rs                # Register allocation + label generation + symbol table (for VM backend)
│       │   └── operand.rs             # Operand parsing (for VM backend)
│       ├── lifetime/                  # Lifetime/token liveness analysis
│       └── mono/                      # Monomorphization
│
├── backends/
│   ├── common/                        # Shared values/heap/opcodes
│   ├── interpreter/                   # Tree-walking interpreter (VM backend)
│   ├── llvm/                          # [! Planned] LLVM backend code generation (see file list below)
│   │   ├── mod.rs                     # [! Planned] LLVM backend entry
│   │   ├── context.rs                 # [! Planned] LLVM context management
│   │   ├── types.rs                   # [! Planned] Type mapping (YaoXiang → LLVM IR)
│   │   ├── values.rs                  # [! Planned] Value mapping
│   │   ├── func.rs                    # [! Planned] Function translation
│   │   ├── spawn.rs                   # [! Planned] spawn block expansion
│   │   ├── ffi.rs                     # [! Planned] FFI call code generation
│   │   └── drop.rs                    # [! Planned] Destructor insertion
│   └── runtime/                       # Compiled runtime (statically linked into exe)
│       ├── engine.rs                  # Task scheduling engine
│       ├── facade.rs                  # External interface
│       └── task.rs                    # Task representation
│
└── util/
    └── diagnostic/                    # Error diagnostics (shared)
```

> **Key Change**: spawn block analysis (task identification, dependency analysis, resource conflict detection) will be implemented in `frontend/core/spawn/` (shared by frontends). The existing `frontend/core/typecheck/passes/spawn_placement.rs` (spawn occurrence check) will be migrated to `frontend/core/spawn/placement.rs`, see RFC-024 for details. The LLVM backend only consumes the analysis results, generating the corresponding dispatch code.
>
> **Current Status**: The current `middle/passes/codegen/` files `buffer.rs`, `emitter.rs`, `bytecode.rs`, `flow.rs`, `operand.rs` serve the VM backend's bytecode generation (`CodegenContext::generate()` → `BytecodeFile`). The LLVM backend will be implemented in `backends/llvm/`, at the same level as the interpreter backend and runtime — both share the same `ModuleIR` input, outputting different target formats (bytecode vs. native code).

### Platform ABI Support

| Platform | Target Triple | Output Format | Calling Convention (FFI Default) |
|------|-----------|----------|---------------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | ELF | System V AMD64 |
| macOS x86_64 | `x86_64-apple-darwin` | Mach-O | System V AMD64 |
| macOS ARM64 | `aarch64-apple-darwin` | Mach-O | ARM64 AAPCS |
| Windows x86_64 | `x86_64-pc-windows-msvc` | COFF | Microsoft x64 |

FFI calls use the platform's C calling convention by default. Users can override via `native("symbol", cc = "stdcall")` and similar options (aligned with future extensions of [RFC-026](./026-ffi-core-mechanism.md)).

### Floating-Point Semantic Consistency (VM ↔ LLVM)

The core promise of the dual-backend architecture is that the VM (development/debugging) and LLVM (production release) behave consistently. Floating-point operations have potential inconsistencies between the two execution modes:

| Scenario | Risk | Strategy |
|------|------|------|
| NaN propagation | VM and LLVM may handle the sign bit and payload of NaN differently | Compiler normalizes NaN representation at the IR level, NaN comparison uniformly uses `fcmp uno` |
| Rounding mode | LLVM defaults to round-to-nearest-even, VM depends on the host CPU | Do not expose non-default rounding modes, both VM and LLVM use RTNE uniformly |
| Division by zero | IEEE 754 defines ±Inf, but some platforms may trap | debug mode checks for division by zero and reports diagnostics; release mode follows IEEE 754 |
| `-0.0` vs `+0.0` | Comparison operations may not be equivalent | Uniformly use IEEE 754 rules: `+0.0 == -0.0` |
| Subnormal numbers | Some platforms flush-to-zero | LLVM does not enable the `denormal-fp-math` attribute, preserving full IEEE 754 semantics |

> **Testing Strategy**: Implement a cross-backend floating-point consistency test suite — the same YaoXiang source code executes in both VM and LLVM backends, comparing outputs value by value. This test set is a mandatory gate in CI.

---

## Trade-offs

### Advantages

1. **Performance**: AOT compilation is 10-100x faster than interpretation
2. **Unified frontend**: VM and LLVM share the same frontend, behavior is completely identical
3. **Zero scheduling overhead**: Normal code is directly generated as sequential machine code, no DAG overhead outside spawn blocks
4. **Static linking**: No external runtime dependencies, a single exe is deployable
5. **Zero GC**: Deterministic RAII destructors, no pauses
6. **Zero FFI overhead**: `?T` null pointer optimization, opaque type layout optimization, FFI call cost equivalent to C
7. **Compile-time analysis**: Task identification and dependency analysis of spawn blocks are completed at compile time, only executed at runtime

### Disadvantages

1. **LLVM integration complexity**: Requires deep understanding of inkwell API and LLVM IR
2. **Compilation time**: AOT compilation is slower than interpreter (one-time cost)
3. **Debugging experience**: Native code debugging requires DWARF/PDB symbol support (compiler needs to generate debug info)
4. **Incremental compilation**: Incremental compilation for large projects requires additional design
5. **Floating-point semantic consistency**: VM and LLVM may differ in edge cases such as NaN propagation, rounding mode, division by zero, etc.; normalization strategies are needed to ensure consistent behavior across both backends (see §10)

### Consistency with Related RFCs

| RFC | Consistency |
|-----|--------|
| RFC-024 spawn block concurrency model | ✅ spawn block direct sub-expressions → task dispatch |
| RFC-008 runtime architecture | ✅ Dual backend + scheduler static lib + module directory structure |
| RFC-009 v9 ownership model | ✅ `&T`/`&mut T` tokens (zero-sized), `ref T` (fat pointer), `?T` (Option) |
| RFC-026 FFI core mechanism | ✅ `native()` → declare + marshalling, `.drop` → RAII cleanup |

---

## Alternatives

| Option | Description | Why Not Chosen |
|------|------|------|
| Interpreter only | No AOT needed | Insufficient performance |
| Pure static compilation (no runtime) | Do not link scheduler | spawn blocks need runtime task scheduling |
| Cranelift backend | Faster compilation speed | Runtime performance inferior to LLVM, as a future optional backend |
| Link external LLVM runtime | Use LLVM's built-in runtime | Introduces unnecessary dependencies |

---

## Implementation Strategy

### Phase Division

#### Phase 1: Foundation Framework
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
- [ ] Implement Move semantics tracking (for destructor insertion judgment)

#### Phase 4: spawn Block Code Generation
- [ ] Consume analysis results from `spawn_placement.rs`
- [ ] Direct sub-expressions → task function generation
- [ ] Dependent task dispatch code generation
- [ ] Resource conflict serialization
- [ ] spawn for expansion

#### Phase 5: FFI Code Generation
- [ ] `native()` → `declare external` (`ffi.rs`)
- [ ] Argument marshalling / return value unmarshalling
- [ ] Opaque type layout (with single-field optimization)
- [ ] `?T` null pointer optimization (FFI-specific)

#### Phase 6: Destructor Code Generation
- [ ] `.drop` binding identification
- [ ] End-of-scope cleanup insertion (reverse order) (`drop.rs`)
- [ ] Early return path cleanup
- [ ] `?` error propagation path cleanup

#### Phase 7: Runtime Library Linking
- [ ] Implement `runtime_task_spawn` / `runtime_task_wait_all` and other runtime functions
- [ ] Link runtime static lib
- [ ] End-to-end integration tests

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
| Core | Lazily create subtasks, create on demand |
| Reference Value | Theoretical foundation for on-demand scheduling of tasks within spawn blocks |

**Core Idea**: Instead of creating tasks immediately, creation is deferred. When a parent task needs a child task's value, the child task is created. This solves the performance overhead problem of fine-grained parallel tasks[^1]. YaoXiang's spawn block scheduling borrows this idea — tasks are identified at compile time, but dispatched to the thread pool on demand at runtime.

### Lazy Scheduling (2014)[^2]

| Attribute | Description |
|------|------|
| Institution | University of Maryland |
| Authors | Tzannes, Caragea |
| Core | Runtime adaptive scheduling, no additional state |
| Reference Value | Design reference for Full Runtime WorkStealing scheduler |

### SISAL Language[^3]

| Attribute | Description |
|------|------|
| Institution | Lawrence Livermore National Laboratory (LLNL) |
| Core | Single-assignment language, dataflow graph, implicit parallelism |
| Reference Value | Proof of feasibility of dataflow model in industrial applications |

**Key Difference**: SISAL's parallelism is **implicit** — the language has single-assignment semantics, and the compiler automatically analyzes the entire program's data dependency graph to determine parallelism. YaoXiang's parallelism is **explicit** — the user marks parallel regions with `spawn {}` blocks, and the compiler only analyzes dependencies within spawn blocks. This avoids SISAL's whole-program analysis complexity while preserving the user's control over parallel behavior.

### Mul-T Parallel Scheme[^4]

| Attribute | Description |
|------|------|
| Institution | MIT |
| Core | Future construct, Lazy Task Creation implementation |
| Reference Value | Concrete implementation reference |

### Comparison Summary

| Technique | Lazy Creation | Parallel Marker | Analysis Scope | Ownership |
|------|----------|----------|----------|--------|
| Lazy Task Creation[^1] | ✅ | Implicit | Whole program | N/A |
| Lazy Scheduling[^2] | ✅ | Implicit | Whole program | N/A |
| SISAL[^3] | ✅ | Implicit (single assignment) | Whole program | N/A |
| Mul-T[^4] | ✅ | Explicit (future) | Call site | N/A |
| **YaoXiang** | ✅ | **Explicit (spawn block)** | **Within spawn block** | **✅ (Move + token + ref)** |

**YaoXiang's Innovation**: Lifting the parallel marker from "each function call" (future) to "structured block" (spawn) — users write normal code and place a spawn block where parallelism is needed. The analysis scope is constrained within the spawn block, making compilation efficient and behavior controllable.

---

## Appendix

### Appendix A: Comparison with Rust async

| Feature | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| Compilation artifact | State machine + machine code | Machine code + spawn task metadata |
| Runtime | tokio | Statically linked scheduler (approx 500KB-1MB) |
| Concurrency marker | async/await keywords | `spawn { }` block |
| Task creation | Compile-time generated state machine | Compile-time identified direct sub-expressions → task functions |
| Color function | async infection | **No function coloring** |
| Synchronous wait | `.await` | spawn block auto-synchronous blocking |
| Memory management | GC (runtime) | **RAII (deterministic)** |
| Sharing mechanism | `Arc::new()` + manual Weak | **`ref` keyword (compiler auto-selects Rc/Arc)** |

### Appendix B: Design Decision Records

| Decision | Resolution | Date |
|------|------|------|
| Adopt LLVM AOT | Direct Codegen, no excessive abstraction | 2026-02-15 |
| Concurrency model alignment | Aligned with RFC-024 spawn block direct sub-expression model | 2026-06-10 |
| DAG analysis scope | Within spawn block, not across spawn blocks (aligned with RFC-024) | 2026-06-05 |
| Ownership model alignment | Aligned with RFC-009 v9: `&T`/`&mut T` tokens + `ref` keyword | 2026-06-10 |
| Dual backend model | VM (development) + LLVM (production), aligned with RFC-008 | 2026-05-11 |
| Scheduler form | Statically linked into exe, approx 500KB-1MB (depending on platform and features), no GC | 2026-05-11 |
| FFI code generation | Integrate RFC-026: `native()` declare + marshalling | 2026-06-10 |
| Destructor | `.drop` → RAII cleanup insertion, aligned with RFC-026 §7 | 2026-06-10 |
| Side effect handling | Remove `@IO`/`@Pure` inference, use RFC-024 resource types | 2026-06-10 |
| Reflection metadata | Compiled into exe's .reflect section, mmap loaded on demand | 2026-05-11 |
| Paper citations | Retain Lazy Task Creation etc., clarify YaoXiang's differences | 2026-02-16 |

---

## References

[^1]: Larus, J. R., & Halstead, R. H. (1990). *Lazy Task Creation: A Technique for Increasing the Granularity of Parallel Programs*. MIT.

[^2]: Tzannes, A., & Caragea, G. (2014). *Lazy Scheduling: A Runtime Adaptive Scheduler for Declarative Parallelism*. University of Maryland.

[^3]: Feo, J. T., et al. (1990). *A report on the SISAL language project*. Lawrence Livermore National Laboratory.

[^4]: Mohr, E., et al. (1991). *Mul-T: A high-performance parallel lisp*. MIT.

- [inkwell LLVM bindings](https://github.com/TheDan64/inkwell)
- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-008: Decoupling Runtime Concurrency Model from Scheduler](../accepted/008-runtime-concurrency-model.md)
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
- [RFC-026: FFI Core Mechanism](./026-ffi-core-mechanism.md)

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting submission for review |
| **Under Review** | `docs/design/rfc/review/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/rfc/accepted/` | Becomes an official design document |
| **Rejected** | `docs/design/rfc/` | Remains in the RFC directory |

> Current status: **Accepted** — Aligned with RFC-024 spawn block concurrency model, RFC-009 v9 ownership model, RFC-026 FFI mechanism