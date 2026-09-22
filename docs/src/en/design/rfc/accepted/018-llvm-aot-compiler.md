---
title: 'RFC-018: LLVM AOT Compiler Design'
status: 'Accepted'
author: 'ChenXu'
created: '2026-02-15'
updated: '2026-07-05 (synced with GitHub Issue #14, #134; added implementation status analysis)'
issue: '#14'
tracking_issue: 'https://github.com/ChenXu233/YaoXiang/issues/134'
---

# RFC-018: LLVM AOT Compiler Design

> **References**:
>
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
> - [RFC-008: Decoupled Design of Runtime Concurrency Model and Scheduler](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
> - [RFC-026: FFI Core Mechanism](./026-ffi-core-mechanism.md)
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)

> **Deprecated**:
>
> - Legacy "bottom-up automatic DAG analysis" model — replaced by the direct subexpression model of
>   spawn blocks in RFC-024
> - `@IO`/`@Pure` implicit side-effect inference — replaced by the resource type mechanism in
>   RFC-024
> - `Arc(T)` type mapping — replaced by the v9 `ref` keyword in RFC-009

## Summary

This document designs the LLVM AOT (Ahead-of-Time) compiler for the YaoXiang language. The LLVM
backend and the VM backend (interpreter) share the same compilation front-end, forming the
dual-backend architecture defined in [RFC-008](../accepted/008-runtime-concurrency-model.md): the VM
is used for development and debugging, and LLVM is used for production releases.

**Core Responsibilities**:

```
Source Code → Front-end (shared) → IR → LLVM Codegen → .o → Link Scheduler Static Library → exe
```

The compiler compiles YaoXiang source code into native machine code, where:

| Language Feature       | Compilation Strategy                                                                                                   |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Normal code            | Sequential machine code, zero scheduling overhead                                                                      |
| `spawn { }` block      | Direct subexpression → task dispatch + synchronous wait (aligned with [RFC-024](../accepted/024-concurrency-model.md)) |
| `native("symbol")`     | LLVM `declare external` + parameter marshalling (aligned with [RFC-026](./026-ffi-core-mechanism.md))                  |
| `.drop` destructor     | RAII cleanup code insertion (aligned with [RFC-009](../accepted/009-ownership-model.md))                               |
| `&T` / `&mut T` tokens | Zero-sized types, disappear after compilation                                                                          |
| `ref T` shared         | `{ refcount_ptr, data_ptr }` fat pointer, compiler auto-selects Rc/Arc                                                 |

**Relationship with RFC-024**: RFC-024 defines the **user semantics** of spawn blocks (direct
subexpressions create tasks, synchronous blocking wait). This document defines **how these semantics
are compiled into machine code**.

**Relationship with RFC-026**: RFC-026 defines the **user syntax** of FFI (`native()`, `[0]` method
binding, `.drop`). This document defines **how FFI calls are lowered into LLVM IR**.

---

## Motivation

### Why an LLVM AOT Compiler?

Currently YaoXiang only has an interpreter as its execution backend:

| Problem                | Impact                                                            |
| ---------------------- | ----------------------------------------------------------------- |
| Performance bottleneck | Interpreted execution is 10-100x slower than machine code         |
| Deployment complexity  | Requires carrying the interpreter and runtime                     |
| Production environment | The interpreter is unsuitable for performance-sensitive scenarios |

### LLVM in the Dual-Backend Model

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 defines the dual-backend architecture:

```
                    ┌─────────────────────┐
                    │   Compilation Front-end (unified) │
                    │   Lexer → Parser     │
                    │   → TypeCheck        │
                    │   → spawn analysis   │
                    │   → Escape Analysis  │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │   VM Backend (development) │     │  LLVM Backend (production)  │
      │   IR → Interpreted Execution │     │  IR → Native Code  │
      │   Step Debugging      │     │  Link Scheduler Static Library   │
      │   Fast Iteration      │     │  Output .exe         │
      └───────────────────┘     └───────────────────┘
```

The **behavior of both backends is completely identical**—the only difference is the execution
method. The same source code, the same type checking, the same spawn analysis results.

---

## Proposal

### 1. Compiler Architecture

The LLVM backend is located at the final stage of the compilation pipeline, receiving IR from the
front-end and generating native code:

```
Source Code
  → Lexer / Parser (frontend/core/)
  → TypeCheck + spawn analysis (frontend/core/typecheck/)
  → IR Generation (middle/core/ir_gen.rs)
  → LLVM Codegen (backends/llvm/)
      ├── Type Mapping: YaoXiang types → LLVM IR types
      ├── Function Translation: IR instructions → LLVM IR instructions
      ├── spawn Expansion: direct subexpressions → task functions + dispatch calls
      ├── FFI Expansion: native() calls → declare + marshalling
      └── Destructor Insertion: scope end → .drop() call
  → LLVM Optimization + Target Code Generation
  → Link Runtime Static Library → Executable
```

### 2. Compilation Flow

```
Phase 1: Front-end (shared with VM backend)
  - Parsing, type checking, spawn block analysis, escape analysis
  - Output: type-annotated IR

Phase 2: LLVM IR Generation
  - Type mapping, function declarations, instruction translation
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

| YaoXiang Type    | LLVM IR Type                  | Description                                                    |
| ---------------- | ----------------------------- | -------------------------------------------------------------- |
| `Int`            | `i64`                         | Default 64-bit signed integer                                  |
| `Int32`          | `i32`                         | Explicit 32-bit integer (mainly for FFI)                       |
| `Float`          | `f64`                         | Default 64-bit float                                           |
| `Float32`        | `f32`                         | Explicit 32-bit float (mainly for FFI)                         |
| `Bool`           | `i1`                          | Boolean value                                                  |
| `Char`           | `i32`                         | Unicode code point                                             |
| `String`         | `{ i8*, i64 }`                | Pointer + byte length                                          |
| `Void`           | `{}`                          | Zero-sized empty type                                          |
| `&T`             | —                             | Zero-sized token, disappears after compilation, produces no IR |
| `&mut T`         | —                             | Zero-sized token, disappears after compilation, produces no IR |
| `ref T`          | `{ i64*, T* }`                | Fat pointer (reference count pointer + data pointer)           |
| `*T`             | `T*`                          | Raw pointer                                                    |
| `[T; N]`         | `[N x T]`                     | Fixed-length array                                             |
| `List(T)`        | `{ T*, i64, i64 }`            | Data pointer + length + capacity                               |
| Struct           | Corresponding LLVM struct     | Fields laid out in declaration order                           |
| Tagged Enum      | `{ i64, [max_payload_size] }` | Tag + max payload union                                        |
| `?T`             | `{ i1, T }`                   | Some-tagged + data (generic representation)                    |
| FFI Opaque Type  | `{ i8* }`                     | Wrapped C pointer                                              |
| Function Pointer | `T (...)*`                    | Function pointer type                                          |

> **`&T` / `&mut T` zero runtime overhead**: [RFC-009](../accepted/009-ownership-model.md) §2.7
> defines that the compiler internally assigns brand identifiers (compile-time unique integers) to
> tokens; after monomorphization and inlining, the brands completely disappear—no token traces exist
> in the generated machine code.

#### 3.2 FFI Parameter Type Mapping

Aligned with [RFC-026](./026-ffi-core-mechanism.md) §2.2, with an added LLVM IR column:

| C Type               | YaoXiang Type          | LLVM IR        | Description                                 |
| -------------------- | ---------------------- | -------------- | ------------------------------------------- |
| `int`                | `Int32`                | `i32`          |                                             |
| `long`               | `Int64`                | `i64`          |                                             |
| `float`              | `Float32`              | `f32`          |                                             |
| `double`             | `Float64`              | `f64`          |                                             |
| `char`               | `Char`                 | `i32`          | C char → YaoXiang Char (Unicode-compatible) |
| `char*`              | `String`               | `{ i8*, i64 }` | marshalling: C string → YaoXiang String     |
| `bool`               | `Bool`                 | `i1`           |                                             |
| `size_t`             | `Uint`                 | `i64`          |                                             |
| `void*`              | `*Void`                | `i8*`          |                                             |
| `struct T*`          | `T` (transparent type) | `T*`           | Pass pointer                                |
| `typedef struct T T` | `T` (opaque type)      | `{ i8* }`      | Wrapped C pointer                           |

### 4. IR Normalization and Instruction Translation

#### 4.0 IR Normalization (Stack → Registers)

The current IR (`src/middle/core/ir.rs`) contains stack manipulation instructions
(`Push`/`Pop`/`Dup`/`Swap`), which is designed for the bytecode VM. LLVM IR is in SSA form and does
not accept stack manipulation.

**Processing Strategy**: The LLVM path first passes through a lightweight normalization pass before
instruction translation:

| Stack Instruction | Normalization Strategy                              |
| ----------------- | --------------------------------------------------- |
| `Push(r)`         | Record `stack.push(r)`, produce no IR               |
| `Pop(r)`          | `r = stack.pop()`, produce `load` (from stack slot) |
| `Dup`             | `stack.push(stack.top())`, produce no IR            |
| `Swap`            | Swap top two elements of the stack, produce no IR   |

After normalization, all operands become register/local variable references, and all stack
operations are eliminated. This pass executes as the first step of `translator.rs`.

> **Why not eliminate stack instructions at the IR level?** Because the VM backend requires stack
> semantics. Normalizing at the LLVM translation entry point preserves the IR's sharing between the
> two backends—each backend consumes the same IR according to its own needs.
>
> **Prerequisite**: The IR generation phase guarantees stack balance—all control flow paths reach
> the same program point with a consistent stack depth (the VM bytecode backend relies on the same
> prerequisite, otherwise bytecode execution would fail). The normalization pass does not check this
> prerequisite; violation results in undefined behavior in the LLVM backend.

#### 4.1 Instruction Translation Table

The following lists the LLVM IR translation strategy for each variant of the `Instruction` enum.
Instruction names are identical to `src/middle/core/ir.rs`.

**Arithmetic Instructions**:

| IR Instruction          | LLVM IR                                 | Description                       |
| ----------------------- | --------------------------------------- | --------------------------------- |
| `Add { dst, lhs, rhs }` | `add` (integer) / `fadd` (float)        | Integer or float addition by type |
| `Sub { dst, lhs, rhs }` | `sub` / `fsub`                          |                                   |
| `Mul { dst, lhs, rhs }` | `mul` / `fmul`                          |                                   |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv`                | Signed/unsigned/float division    |
| `Mod { dst, lhs, rhs }` | `srem` / `urem`                         | Signed/unsigned modulo            |
| `Neg { dst, src }`      | `sub 0, src` (integer) / `fneg` (float) |                                   |

> **`Mod` Row Revision Notice (note 2026-09-22, with RFC-011b)**: `srem` / `urem` are **truncation
> remainder** (sign follows the dividend); the description "modulo" in this row is a term mix-up.
> [RFC-011b](./011b-operator-overloading.md) has confirmed the established semantics of `%` as
> **mathematical modulo** (result sign follows the divisor, language reference priority table
> "multiplication/division/modulo" takes precedence). Once that semantic is implemented, this row's
> mapping must change to `srem` + sign correction sequence (or `sdiv`+`mul`+`sub` synthesis), and
> the implementation side (interpreter `checked_rem`, constant folding, bytecode `I64_REM`) must be
> modified in sync.

**Bitwise Instructions**:

| IR Instruction          | LLVM IR | Description            |
| ----------------------- | ------- | ---------------------- |
| `And { dst, lhs, rhs }` | `and`   |                        |
| `Or { dst, lhs, rhs }`  | `or`    |                        |
| `Xor { dst, lhs, rhs }` | `xor`   |                        |
| `Shl { dst, lhs, rhs }` | `shl`   | Left shift             |
| `Shr { dst, lhs, rhs }` | `lshr`  | Logical right shift    |
| `Sar { dst, lhs, rhs }` | `ashr`  | Arithmetic right shift |

**Comparison Instructions**:

| IR Instruction         | LLVM IR                 | Description |
| ---------------------- | ----------------------- | ----------- |
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp oeq`  |             |
| `Ne { dst, lhs, rhs }` | `icmp ne` / `fcmp one`  |             |
| `Lt { dst, lhs, rhs }` | `icmp slt` / `fcmp olt` |             |
| `Le { dst, lhs, rhs }` | `icmp sle` / `fcmp ole` |             |
| `Gt { dst, lhs, rhs }` | `icmp sgt` / `fcmp ogt` |             |
| `Ge { dst, lhs, rhs }` | `icmp sge` / `fcmp oge` |             |

**Control Flow Instructions**:

| IR Instruction          | LLVM IR                                     | Description          |
| ----------------------- | ------------------------------------------- | -------------------- |
| `Jmp(label)`            | `br label %L`                               | Unconditional jump   |
| `JmpIf(cond, label)`    | `br i1 %cond, label %L, label %fallthrough` | Conditional jump     |
| `JmpIfNot(cond, label)` | `br i1 %cond, label %fallthrough, label %L` | Conditional not-jump |
| `Ret(Some(v))`          | `ret T %v`                                  | Return with value    |
| `Ret(None)`             | `ret void`                                  | Return without value |

**Call Instructions**:

| IR Instruction                             | LLVM IR                                | Description                               |
| ------------------------------------------ | -------------------------------------- | ----------------------------------------- |
| `Call { dst, func, args }`                 | `%r = call T @func(...)`               | Static call                               |
| `CallVirt { dst, obj, method_name, args }` | vtable GEP + `call` (function pointer) | Virtual method call, looked up via vtable |
| `CallDyn { dst, func, args }`              | `%r = call T %func(...)`               | Dynamic call (closure/function pointer)   |
| `TailCall { func, args }`                  | `musttail call` / `tail call`          | Tail call optimization                    |

**Memory Instructions**:

| IR Instruction                        | LLVM IR                                                   | Description                                                                 |
| ------------------------------------- | --------------------------------------------------------- | --------------------------------------------------------------------------- |
| `Move { dst, src }`                   | —                                                         | After normalization becomes register copy, SSA construction eliminates most |
| `Load { dst, src }`                   | `%v = load T, T* %src`                                    |                                                                             |
| `Store { dst, src }`                  | `store T %src, T* %dst`                                   |                                                                             |
| `Alloc { dst, size }`                 | `%p = alloca T` (stack) / `call @malloc` (escape to heap) | Escape analysis determines allocation location                              |
| `Free(ptr)`                           | `call @free(%ptr)` (heap) / — (stack, auto-reclaimed)     |                                                                             |
| `AllocArray { dst, size, elem_size }` | `%p = alloca [N x T]` (stack) / `call @malloc` (heap)     |                                                                             |

**Struct/Array Access Instructions**:

| IR Instruction                            | LLVM IR                                                | Description                          |
| ----------------------------------------- | ------------------------------------------------------ | ------------------------------------ |
| `LoadField { dst, src, field }`           | `%ptr = getelementptr T, T* %src, 0, field` + `load`   |                                      |
| `StoreField { dst, field, src }`          | `%ptr = getelementptr T, T* %dst, 0, field` + `store`  |                                      |
| `LoadIndex { dst, src, index }`           | `%ptr = getelementptr T, T* %src, 0, %index` + `load`  |                                      |
| `StoreIndex { dst, index, src }`          | `%ptr = getelementptr T, T* %dst, 0, %index` + `store` |                                      |
| `CreateStruct { dst, type_name, fields }` | `insertvalue` chain                                    | Construct LLVM struct by field order |

**Type Conversion Instructions**:

| IR Instruction                   | LLVM IR                                                                                                     | Description                                                      |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `Cast { dst, src, target_type }` | `bitcast` / `trunc` / `zext` / `sext` / `fptrunc` / `fpext` / `sitofp` / `fptosi` / `inttoptr` / `ptrtoint` | Select appropriate cast by source/target type combination        |
| `TypeTest(val, type)`            | —                                                                                                           | Compile-time type test, generates `icmp eq` to compare type tags |

**Ownership and Borrowing Instructions**:

| IR Instruction                 | LLVM IR                                              | Description                                                                   |
| ------------------------------ | ---------------------------------------------------- | ----------------------------------------------------------------------------- |
| `Borrow { dst, src, mutable }` | —                                                    | **Zero-sized token, completely disappears after compilation**, produces no IR |
| `Release(val)`                 | —                                                    | **Zero-sized token, completely disappears after compilation**                 |
| `Move { dst, src }`            | —                                                    | Ownership transfer, becomes register copy after normalization                 |
| `Drop(val)`                    | `call void @T.drop(T* %val)`                         | Call type's destructor (see §7)                                               |
| `ShareRef { dst, src }`        | `call %T* @Arc_new(%src)` / `call %T* @Rc_new(%src)` | Compiler auto-selects Arc/Rc based on cross-thread usage                      |
| `ArcNew { dst, src }`          | `call %T* @Arc_new(%src)`                            | Atomic reference count = 1                                                    |
| `ArcClone { dst, src }`        | `call %T* @Arc_clone(%src)`                          | Atomic increment reference count                                              |
| `ArcDrop(val)`                 | `call void @Arc_drop(%val)`                          | Atomic decrement + conditional release                                        |

**Concurrency Instructions**:

| IR Instruction                     | LLVM IR                               | Description                                                             |
| ---------------------------------- | ------------------------------------- | ----------------------------------------------------------------------- |
| `Spawn { closures, plan, result }` | Expanded into scheduler call sequence | See §5 in detail, runtime `task_spawn` + `task_wait_all`                |
| `Yield`                            | —                                     | AOT path has synchronous wait in spawn blocks, no yield needed; ignored |

**unsafe Blocks and Raw Pointer Instructions**:

| IR Instruction            | LLVM IR                                                 | Description                             |
| ------------------------- | ------------------------------------------------------- | --------------------------------------- |
| `UnsafeBlockStart`        | —                                                       | **Compile-time marker, produces no IR** |
| `UnsafeBlockEnd`          | —                                                       | **Compile-time marker, produces no IR** |
| `PtrFromRef { dst, src }` | `%p = ptrtoint T* %src to i64` (or direct pointer copy) |                                         |
| `PtrDeref { dst, src }`   | `%v = load T, T* %src`                                  |                                         |
| `PtrStore { dst, src }`   | `store T %src, T* %dst`                                 |                                         |
| `PtrLoad { dst, src }`    | `%v = load T, T* %src`                                  |                                         |

**String Instructions**:

| IR Instruction                      | LLVM IR                                     | Description                                 |
| ----------------------------------- | ------------------------------------------- | ------------------------------------------- |
| `StringLength { dst, src }`         | `%len = extractvalue { i8*, i64 } %src, 1`  | String is `{ ptr, len }`, length in field 1 |
| `StringConcat { dst, lhs, rhs }`    | `call String @yx_string_concat(%lhs, %rhs)` | Runtime helper function                     |
| `StringGetChar { dst, src, index }` | `getelementptr` + `load i32`                | With bounds check                           |
| `StringFromInt { dst, src }`        | `call String @yx_string_from_int(%src)`     | Runtime helper function                     |
| `StringFromFloat { dst, src }`      | `call String @yx_string_from_f64(%src)`     | Runtime helper function                     |

**Closure Instructions**:

| IR Instruction                           | LLVM IR                                                                                   | Description                           |
| ---------------------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------- |
| `MakeClosure { dst, func: String, env }` | Allocate closure struct + fill function pointer (lookup by function name) and environment | `{ fn_ptr, env_fields... }`           |
| `LoadUpvalue { dst, upvalue_idx }`       | `%v = extractvalue %env, upvalue_idx`                                                     | Read upvalue from closure environment |
| `StoreUpvalue { src, upvalue_idx }`      | `%env = insertvalue %env, %src, upvalue_idx`                                              | Write upvalue to closure environment  |
| `CloseUpvalue(val)`                      | Copy upvalue from stack to heap                                                           |                                       |

**Other Instructions**:

| IR Instruction                  | LLVM IR                                       | Description                        |
| ------------------------------- | --------------------------------------------- | ---------------------------------- |
| `HeapAlloc { dst, type_id }`    | `call i8* @malloc(i64 size)` + write type tag | Heap allocation + type information |
| `NewDict { dst, keys, values }` | `call Dict @yx_dict_new(%keys, %values)`      | Runtime helper function            |

> **Note**: `Push`/`Pop`/`Dup`/`Swap` are already eliminated in the §4.0 normalization phase and do
> not appear in the translation table. `Borrow`/`Release` are zero-sized compile-time tokens and
> produce no machine code at all.

### 5. spawn Block Code Generation

Aligned with [RFC-024](../accepted/024-concurrency-model.md), the compilation of spawn blocks is
divided into the following steps.

#### 5.1 Semantic Recap

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // direct subexpression → task 1
    t2 = fetch("url2"),   // direct subexpression → task 2
    return (t1, t2)       // synchronous wait, assemble result
}
```

**Rules** (RFC-024 §2.1):

- The **direct subexpressions** of a spawn block (top-level comma-separated statements) create
  parallel tasks
- Expressions inside nested `{}` are not direct subexpressions and do not become independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning

#### 5.2 Compilation Steps

```
Step 1: Identify direct subexpressions
  Traverse the spawn block body, collect top-level statements

Step 2: Dependency analysis
  For each direct subexpression, analyze which variables it references that were produced by previous tasks
  No dependency → can be scheduled in parallel immediately
  Has dependency → queue waiting for dependency task to complete

Step 3: Resource conflict detection (RFC-024 §2.5)
  Check if the same resource type instance is used by multiple tasks
  Same instance conflict → mark serial execution order

Step 4: Generate task functions
  Each direct subexpression generates an independent LLVM function (closure)

Step 5: Generate dispatch code
  Call runtime scheduler's task_spawn / task_wait

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

The compiler detects that `parse(data)` references the `data` produced by task 1, and marks the
dependency when generating dispatch code:

```llvm
; Task 2 is created with a dependency on task 1
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
;                                                              ↑
;                                                 depends on task 0 (fetch) completing
```

#### 5.5 Automatic Serialization of Resource Types

The resource types defined in [RFC-024 §2.5](../accepted/024-concurrency-model.md) (`FilePath`,
`HttpUrl`, `DBUrl`, `Console`, and user-defined resource types) are automatically serialized in
spawn blocks:

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // uses SqliteDb (resource type)
    r2 = db.exec("INSERT ...")    // same instance → automatically serialized
}
```

The compiler detects that the same resource instance is used by two tasks and generates a serial
dependency:

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

The compiler expands this into N independent tasks (N = length of items), limited by the maximum
concurrency.

### 6. FFI Code Generation

> ⚠️ **Dependency Note**: The FFI code generation **architecture** defined in this section
> (`native("x")` → `declare external @x` → marshalling wrapper function → `call`) is stable and does
> not change with RFC-026 syntax changes. The specific parameter marshalling rules table (§6.2) and
> opaque type layout (§6.3) reference RFC-026's definitions—if RFC-026's `native()` syntax or
> marshalling rules change, only the corresponding mapping tables in this document need to be
> updated; the architecture layer is unaffected. RFC-026 current status: **Under review**, in the
> same `review/` directory as this document.
>
> **Acceptance Precondition**: Before this RFC is accepted, the parts of RFC-026 related to §6 of
> this document (`native()` declaration syntax, parameter marshalling rules, opaque type `{ i8* }`
> layout, `.drop` binding convention) should be frozen first or accepted together with 026.
> Otherwise, the mapping tables in §6.2/§6.3/§7 may become outdated before implementation.

Aligned with [RFC-026](./026-ffi-core-mechanism.md), this section defines the LLVM IR generation
strategy for FFI calls.

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

**Key Points**:

- `native("sqlite3_open")` → `declare external @sqlite3_open`
- Compiler automatically generates marshalling wrapper function
- The wrapper function's signature uses YaoXiang types, internally converting to C types

#### 6.2 Parameter Marshalling

| Direction                                       | Conversion                              |
| ----------------------------------------------- | --------------------------------------- |
| YaoXiang `String` → C `char*`                   | Extract `.ptr` field and pass           |
| YaoXiang `Int32` → C `int`                      | Pass directly (`i32`)                   |
| YaoXiang `*Void` → C `void*`                    | Pass directly (`i8*`)                   |
| YaoXiang `T` (transparent type) → C `struct T*` | Pass by address                         |
| YaoXiang `T` (opaque type) → C `struct T*`      | Extract pointer from `{ i8* }` and pass |

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

LLVM Layout: `{ i8* }` — a struct containing a C pointer.

**Layout Optimization**: When an opaque type has only one `handle: *Void` field, it can be optimized
to directly use `i8*` (omitting the outer struct). The optimized ABI is fully consistent with C
pointers, with zero marshalling overhead. This optimization is enabled by default and transparent to
users.

#### 6.4 LLVM Representation of ?T Nullable Return Values

The FFI nullable return value defined in [RFC-026](./026-ffi-core-mechanism.md) §7.6:

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

Generic LLVM representation: `{ i1, { i8* } }` — Some-tagged + data.

**Optimization for FFI null pointers**: If the `T` in `?T` is an opaque type (internally a pointer),
the compiler uses the **null pointer = None** optimization:

```llvm
; Optimized LLVM representation: directly use nullable pointers
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

This optimization makes FFI calls to `?SqliteDb` have **zero additional overhead**—completely
equivalent to C's null check.

#### 6.5 yx-bindgen Integration

The binding files automatically generated by [yx-bindgen](./026-ffi-core-mechanism.md) §6 are
treated as ordinary YaoXiang source code during compilation. The compiler does not need to know the
code comes from bindgen—the handling of `native()` declarations and `unsafe {}` type definitions is
completely consistent.

### 7. Destructor Code Generation

Aligned with the RAII semantics in [RFC-009](../accepted/009-ownership-model.md) and the `.drop`
convention in [RFC-026](./026-ffi-core-mechanism.md) §7.

#### 7.1 .drop Binding Identification

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

The compiler identifies the `.drop` binding and marks the destructor function pointer in the type
metadata.

#### 7.2 Cleanup Insertion at Scope End

```
User code:
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT ...")
    stmt.step()
    // ← scope ends
}

Compiler-inserted cleanup (reverse order):
    call @sqlite3_finalize(%stmt)    // stmt.drop()
    call @sqlite3_close(%db)          // db.drop()
```

**Insertion Positions**:

- Normal scope end (`}`)
- Early return (before `return`)
- `?` error propagation path (before `?`)
- spawn block end (destructors of variables within the task)

#### 7.3 Move and Destructors

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: ownership transferred to db2
// db is no longer valid, no drop is inserted for db here
// ← scope end: drop is only inserted for db2
```

The compiler tracks Move semantics ([RFC-009](../accepted/009-ownership-model.md) §1) and inserts
destructor calls only at the final holder of the variable.

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

### 8. Compilation Output Structure

The compilation output consists of the following components (the specific struct definitions will be
determined in the implementation phase):

- **Machine Code**: LLVM-compiled object files (`.o`), containing all function translation results
- **spawn Metadata**: Each spawn block's task function pointers, dependency relationships, resource
  conflict serialization pairs
- **FFI Symbol Table**: External C symbol references (symbol name + whether weakly referenced)
- **Entry Point Table**: List of entry functions for the executable
- **Type Information**: Reflection metadata, written to the `.reflect` section, mmap'd by the
  runtime on demand

### 9. Runtime Library

Aligned with [RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md), the runtime is linked
into the final exe in the form of a **static library**.

```
Final exe internal structure:

┌────────────────────────────────────────────┐
│  User Code (native machine code)            │
│  ├── Normal functions (sequential execution)  │
│  ├── spawn block expansion (task functions + dispatch calls)  │
│  ├── FFI marshalling wrapper functions      │
│  └── RAII destructor code                   │
├────────────────────────────────────────────┤
│  Runtime Static Library (about 500KB-1MB, depending on platform and feature selection)  │
│  ├── Thread pool (num_workers)                │
│  ├── Event loop (libuv / io_uring)             │
│  ├── Work-stealing queue (Full Runtime only)    │
│  ├── Memory allocator (jemalloc / mimalloc)    │
│  └── Reflection metadata (.reflect section, mmap on demand)  │
│                                              │
│  Not included:                              │
│  ❌ Bytecode interpreter                    │
│  ❌ JIT compiler                              │
│  ❌ GC                                      │
│  ❌ Virtual machine                            │
└────────────────────────────────────────────┘
```

**Key Design**: Task identification and dependency analysis for spawn blocks are completed at
compile time; the runtime only does "create task → dispatch to thread pool → wait for
completion"—fixed data structures, predictable behavior.

> **Difference from RFC-008 Size Estimate**: RFC-008 §4 estimates the scheduler at about 200-500KB,
> containing only the task scheduling core. The 500KB-1MB estimate in this document additionally
> includes the memory allocator (jemalloc/mimalloc), event loop (libuv/io_uring), and reflection
> metadata section. The actual size depends on the platform and feature selection; precise numbers
> will be given in the implementation phase.

**Relationship Between Three-Layer Runtime and LLVM** (aligned with RFC-008 §1):

| Runtime      | LLVM AOT Behavior                                                                                                |
| ------------ | ---------------------------------------------------------------------------------------------------------------- |
| **Embedded** | No spawn support, directly generates sequential machine code                                                     |
| **Standard** | Supports spawn blocks, DAG within spawn block + single-threaded scheduling (num_workers=1)                       |
| **Full**     | Supports spawn blocks, DAG within spawn block + multi-threaded scheduling (num_workers>1), supports WorkStealing |

---

## Detailed Design

### Module Directory Structure

Aligned with the directory layout in [RFC-008](../accepted/008-runtime-concurrency-model.md) §6.
`[! Planned]` markers indicate files/directories not yet created, to be introduced in the
implementation phase of this RFC.

```
src/
├── frontend/                          # Compilation front-end (shared by all backends)
│   ├── core/
│   │   ├── spawn/                     # spawn module (concurrency analysis shared by VM and LLVM backends)
│   │   │   ├── mod.rs                 # spawn module entry
│   │   │   ├── placement.rs           # spawn occurrence location legality check
│   │   │   └── analysis.rs            # [! Planned] task identification, dependency analysis, resource conflict detection
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
│       │   ├── translator.rs          # IR → bytecode translation (VM backend)
│       │   ├── emitter.rs             # Bytecode emission + jump back-patching (VM backend)
│       │   ├── buffer.rs              # Constant pool + bytecode buffer (VM backend)
│       │   ├── bytecode.rs            # Bytecode format definition + serialization (VM backend)
│       │   ├── flow.rs                # Register allocation + label generation + symbol table (VM backend)
│       │   └── operand.rs             # Operand parsing (VM backend)
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
│   │   └── drop.rs                    # [! Planned] Destructor insertion
│   └── runtime/                       # Compiled runtime (static library linked into exe)
│       ├── engine.rs                  # Task scheduling engine
│       ├── facade.rs                  # External interface
│       └── task.rs                    # Task representation
│
└── util/
    └── diagnostic/                    # Error diagnostics (shared)
```

> **Key Change**: spawn block analysis (task identification, dependency analysis, resource conflict
> detection) will be implemented in `frontend/core/spawn/` (shared by the front-end). The existing
> `frontend/core/typecheck/passes/spawn_placement.rs` (spawn occurrence location check) will be
> migrated to `frontend/core/spawn/placement.rs`; see RFC-024 for details. The LLVM backend only
> consumes the analysis results and generates the corresponding dispatch code.
>
> **Current Status**: The current `middle/passes/codegen/` files `buffer.rs`, `emitter.rs`,
> `bytecode.rs`, `flow.rs`, `operand.rs` serve the VM backend's bytecode generation
> (`CodegenContext::generate()` → `BytecodeFile`). The LLVM backend will be implemented in
> `backends/llvm/`, parallel to the interpreter backend and runtime—both share the same `ModuleIR`
> input but output different target formats (bytecode vs native code).

### Platform ABI Support

| Platform       | Target Triple              | Output Format | Calling Convention (FFI default) |
| -------------- | -------------------------- | ------------- | -------------------------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu` | ELF           | System V AMD64                   |
| macOS x86_64   | `x86_64-apple-darwin`      | Mach-O        | System V AMD64                   |
| macOS ARM64    | `aarch64-apple-darwin`     | Mach-O        | ARM64 AAPCS                      |
| Windows x86_64 | `x86_64-pc-windows-msvc`   | COFF          | Microsoft x64                    |

FFI calls use the platform's C calling convention by default. Users can override via options like
`native("symbol", cc = "stdcall")` (aligned with future extensions of
[RFC-026](./026-ffi-core-mechanism.md)).

### Floating-Point Semantic Consistency (VM ↔ LLVM)

The core promise of the dual-backend architecture is that the VM (development and debugging) and
LLVM (production release) behave consistently. Floating-point operations have potential
inconsistencies between the two execution modes:

| Scenario             | Risk                                                           | Strategy                                                                                     |
| -------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| NaN propagation      | VM and LLVM may handle NaN sign bits and payloads differently  | Compiler normalizes NaN representation at IR level; NaN comparisons uniformly use `fcmp uno` |
| Rounding mode        | LLVM defaults to round-to-nearest-even, VM depends on host CPU | Do not expose non-default rounding modes; VM and LLVM uniformly use RTNE                     |
| Division by zero     | IEEE 754 defines ±Inf, but some platforms may trap             | Debug mode checks division by zero and reports diagnostics; release mode follows IEEE 754    |
| `-0.0` vs `+0.0`     | Comparison operations may be non-equivalent                    | Uniformly use IEEE 754 rules: `+0.0 == -0.0`                                                 |
| Denormalized numbers | Some platforms flush-to-zero                                   | LLVM does not enable `denormal-fp-math` attribute, preserving full IEEE 754 semantics        |

> **Testing Strategy**: Implement a cross-backend floating-point consistency test suite—the same
> YaoXiang source code is executed on the VM and LLVM backends respectively, and outputs are
> compared value by value. This set of tests is a mandatory CI gate.

---

## Trade-offs

### Advantages

1. **Performance**: AOT compilation is 10-100x faster than interpreted execution
2. **Unified Front-end**: VM and LLVM share the same front-end, with completely consistent behavior
3. **Zero Scheduling Overhead**: Normal code is directly generated as sequential machine code, with
   no DAG overhead outside spawn blocks
4. **Static Linking**: No external runtime dependencies, a single exe can be deployed
5. **Zero GC**: RAII deterministic destructors, no pauses
6. **FFI Zero Overhead**: `?T` null pointer optimization, opaque type layout optimization, FFI call
   cost is equivalent to C
7. **Compile-time Analysis**: Task identification and dependency analysis for spawn blocks are
   completed at compile time; the runtime only executes

### Disadvantages

1. **LLVM Integration Complexity**: Requires deep understanding of inkwell API and LLVM IR
2. **Compilation Time**: AOT compilation is slower than the interpreter (one-time cost)
3. **Debugging Experience**: Native code debugging requires DWARF/PDB symbol support (compiler must
   generate debug info)
4. **Incremental Compilation**: Incremental compilation for large projects requires additional
   design
5. **Floating-Point Semantic Consistency**: VM and LLVM may differ in edge cases like NaN
   propagation, rounding modes, and division by zero; normalization strategies are needed to ensure
   dual-backend behavior consistency (see §10)

### Consistency with Related RFCs

| RFC                                   | Consistency                                                                |
| ------------------------------------- | -------------------------------------------------------------------------- |
| RFC-024 spawn block concurrency model | ✅ spawn block direct subexpressions → task dispatch                       |
| RFC-008 Runtime Architecture          | ✅ Dual backend + scheduler static library + module directory structure    |
| RFC-009 Ownership Model v9            | ✅ `&T`/`&mut T` tokens (zero-sized), `ref T` (fat pointer), `?T` (Option) |
| RFC-026 FFI Core Mechanism            | ✅ `native()` → declare + marshalling, `.drop` → RAII cleanup              |

---

## Alternatives

| Alternative                          | Description                 | Why Not Chosen                                                                   |
| ------------------------------------ | --------------------------- | -------------------------------------------------------------------------------- |
| Interpreter only                     | No AOT needed               | Insufficient performance                                                         |
| Pure static compilation (no runtime) | No scheduler linked         | spawn blocks require runtime task scheduling                                     |
| Cranelift backend                    | Faster compilation speed    | Runtime performance not as good as LLVM, considered as a future optional backend |
| Link external LLVM runtime           | Use LLVM's built-in runtime | Introduces unnecessary dependencies                                              |

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
- [ ] Implement Move semantics tracking (for destructor insertion decisions)

#### Phase 4: spawn Block Code Generation

- [ ] Consume analysis results from `spawn_placement.rs`
- [ ] Direct subexpression → task function generation
- [ ] Dependent task dispatch code generation
- [ ] Resource conflict serialization
- [ ] spawn for expansion

#### Phase 5: FFI Code Generation

- [ ] `native()` → `declare external` (`ffi.rs`)
- [ ] Parameter marshalling / return value unmarshalling
- [ ] Opaque type layout (with single-field optimization)
- [ ] `?T` null pointer optimization (FFI-specific)

#### Phase 6: Destructor Code Generation

- [ ] `.drop` binding identification
- [ ] Scope-end cleanup insertion (reverse order) (`drop.rs`)
- [ ] Early return path cleanup
- [ ] `?` error propagation path cleanup

#### Phase 7: Runtime Library Linking

- [ ] Implement runtime functions like `runtime_task_spawn` / `runtime_task_wait_all`
- [ ] Link runtime static library
- [ ] End-to-end integration testing

### Dependency Relationships

- RFC-024 (spawn block concurrency) → Phase 4 input
- RFC-009 v9 (ownership) → Phase 3, 6 input
- RFC-008 (runtime architecture) → Phase 7 input
- RFC-026 (FFI mechanism) → Phase 5 input

---

## Related Work

### Lazy Task Creation (1990)[^1]

| Property        | Description                                                       |
| --------------- | ----------------------------------------------------------------- |
| Institution     | MIT                                                               |
| Authors         | James R. Larus, Robert H. Halstead Jr.                            |
| Core            | Deferred creation of subtasks, on-demand creation                 |
| Reference value | Theoretical basis for on-demand task dispatch within spawn blocks |

**Core Idea**: Instead of creating tasks immediately, creation is deferred. When a parent task needs
the value of a child task, the child task is created. This addresses the performance overhead
problem of fine-grained parallel tasks[^1]. YaoXiang's spawn block scheduling draws on this
idea—tasks are identified at compile time, but dispatched on demand to the thread pool at runtime.

### Lazy Scheduling (2014)[^2]

| Property        | Description                                          |
| --------------- | ---------------------------------------------------- |
| Institution     | University of Maryland                               |
| Authors         | Tzannes, Caragea                                     |
| Core            | Runtime adaptive scheduling, no additional state     |
| Reference value | Full Runtime WorkStealing scheduler design reference |

### SISAL Language[^3]

| Property        | Description                                                      |
| --------------- | ---------------------------------------------------------------- |
| Institution     | Lawrence Livermore National Laboratory (LLNL)                    |
| Core            | Single-assignment language, Dataflow graph, implicit parallelism |
| Reference value | Feasibility proof of Dataflow model in industrial applications   |

**Key Difference**: SISAL's parallelism is **implicit**—the language is single-assignment semantics,
and the compiler automatically analyzes the whole program's data dependency graph to determine
parallelism. YaoXiang's parallelism is **explicit**—users mark parallel regions with `spawn {}`
blocks, and the compiler only analyzes dependencies within the spawn block. This avoids the
complexity of SISAL's whole-program analysis while preserving user control over parallel behavior.

### Mul-T Parallel Scheme[^4]

| Property        | Description                                         |
| --------------- | --------------------------------------------------- |
| Institution     | MIT                                                 |
| Core            | Future construct, Lazy Task Creation implementation |
| Reference value | Specific implementation reference                   |

### Comparison Summary

| Technique              | Deferred Creation | Parallelism Marker           | Analysis Scope         | Ownership                    |
| ---------------------- | ----------------- | ---------------------------- | ---------------------- | ---------------------------- |
| Lazy Task Creation[^1] | ✅                | Implicit                     | Whole Program          | N/A                          |
| Lazy Scheduling[^2]    | ✅                | Implicit                     | Whole Program          | N/A                          |
| SISAL[^3]              | ✅                | Implicit (single-assignment) | Whole Program          | N/A                          |
| Mul-T[^4]              | ✅                | Explicit (future)            | Call Site              | N/A                          |
| **YaoXiang**           | ✅                | **Explicit (spawn block)**   | **Within spawn block** | **✅ (Move + Tokens + ref)** |

**YaoXiang's Innovation**: Elevates the parallelism marker from "per function call" (future) to
"structured block" (spawn). Users write ordinary code and place spawn blocks where parallelism is
needed. The analysis scope is constrained within the spawn block, making compilation efficient and
behavior controllable.

---

## Appendix

### Appendix A: Comparison with Rust async

| Feature            | Rust async                            | YaoXiang LLVM AOT                                                 |
| ------------------ | ------------------------------------- | ----------------------------------------------------------------- |
| Compilation Output | State machine + machine code          | Machine code + spawn task metadata                                |
| Runtime            | tokio                                 | Statically linked scheduler (about 500KB-1MB)                     |
| Concurrency Marker | async/await keywords                  | `spawn { }` block                                                 |
| Task Creation      | Compile-time state machine generation | Compile-time direct subexpression identification → task functions |
| Colored Functions  | async contagion                       | **No function coloring**                                          |
| Synchronous Wait   | `.await`                              | spawn block auto-synchronous block                                |
| Memory Management  | GC (runtime)                          | **RAII (deterministic)**                                          |
| Sharing Mechanism  | `Arc::new()` + manual Weak            | **`ref` keyword (compiler auto-selects Rc/Arc)**                  |

### Appendix B: Design Decision Records

| Decision                    | Determination                                                                               | Date       |
| --------------------------- | ------------------------------------------------------------------------------------------- | ---------- |
| Adopt LLVM AOT              | Direct Codegen, no over-abstraction                                                         | 2026-02-15 |
| Concurrency Model Alignment | Aligned with RFC-024 spawn block direct subexpression model                                 | 2026-06-10 |
| DAG Analysis Scope          | Within spawn block, not across spawn blocks (aligned with RFC-024)                          | 2026-06-05 |
| Ownership Model Alignment   | Aligned with RFC-009 v9: `&T`/`&mut T` tokens + `ref` keyword                               | 2026-06-10 |
| Dual-Backend Model          | VM (development) + LLVM (production), aligned with RFC-008                                  | 2026-05-11 |
| Scheduler Form              | Static library linked into exe, about 500KB-1MB (depending on platform and features), no GC | 2026-05-11 |
| FFI Code Generation         | Integrated with RFC-026: `native()` declare + marshalling                                   | 2026-06-10 |
| Destructor                  | `.drop` → RAII cleanup insertion, aligned with RFC-026 §7                                   | 2026-06-10 |
| Side-effect Handling        | Removed `@IO`/`@Pure` inference, switched to RFC-024 resource types                         | 2026-06-10 |
| Reflection Metadata         | Compiled into exe .reflect section, mmap on demand                                          | 2026-05-11 |
| Paper Citations             | Retained Lazy Task Creation etc., clarifying YaoXiang's differences                         | 2026-02-16 |

---

## References

[^1]:
    Larus, J. R., & Halstead, R. H. (1990). _Lazy Task Creation: A Technique for Increasing the
    Granularity of Parallel Programs_. MIT.

[^2]:
    Tzannes, A., & Caragea, G. (2014). _Lazy Scheduling: A Runtime Adaptive Scheduler for
    Declarative Parallelism_. University of Maryland.

[^3]:
    Feo, J. T., et al. (1990). _A report on the SISAL language project_. Lawrence Livermore National
    Laboratory.

[^4]: Mohr, E., et al. (1991). _Mul-T: A high-performance parallel lisp_. MIT.

- [inkwell LLVM bindings](https://github.com/TheDan64/inkwell)
- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-008: Decoupled Design of Runtime Concurrency Model and Scheduler](../accepted/008-runtime-concurrency-model.md)
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
- [RFC-026: FFI Core Mechanism](./026-ffi-core-mechanism.md)

---

## Lifecycle and Fate

| Status           | Location                    | Description                                |
| ---------------- | --------------------------- | ------------------------------------------ |
| **Draft**        | `docs/design/rfc/`          | Author's draft, awaiting submission review |
| **Under Review** | `docs/design/rfc/review/`   | Open community discussion and feedback     |
| **Accepted**     | `docs/design/rfc/accepted/` | Becomes official design document           |
| **Rejected**     | `docs/design/rfc/`          | Retained in RFC directory                  |

> Current Status: **Accepted** — Aligned with RFC-024 spawn block concurrency model, RFC-009 v9
> ownership model, RFC-026 FFI mechanism
