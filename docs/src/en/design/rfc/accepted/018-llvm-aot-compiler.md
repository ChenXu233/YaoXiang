---
title: 'RFC-018: LLVM AOT Compiler Design'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-02-15'
updated: '2026-07-05 (synced with GitHub Issue #14, #134; added implementation status analysis)'
issue: '#14'
tracking_issue: 'https://github.com/ChenXu233/YaoXiang/issues/134'
---

# RFC-018: LLVM AOT Compiler Design

> **References**:
>
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
> - [RFC-026: FFI Core Mechanism](./026-ffi-core-mechanism.md)
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)

> **Superseded**:
>
> - Old "bottom-up automatic DAG analysis" model — replaced by RFC-024 spawn block direct
>   subexpression model
> - `@IO`/`@Pure` implicit side-effect inference — replaced by RFC-024 resource type mechanism
> - `Arc(T)` type mapping — replaced by RFC-009 v9 `ref` keyword

## Abstract

This document designs the LLVM AOT (Ahead-of-Time) compiler for the YaoXiang language. The LLVM
backend shares the same compilation frontend with the VM backend (interpreter), forming the
dual-backend architecture defined in [RFC-008](../accepted/008-runtime-concurrency-model.md): VM for
development/debugging, LLVM for production/release.

**Core responsibilities**:

```
Source Code → Frontend (shared) → IR → LLVM Codegen → .o → Link scheduler static library → exe
```

The compiler translates YaoXiang source code into native machine code, where:

| Language Feature      | Compilation Strategy                                                                                                 |
| --------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Normal code           | Sequential machine code, zero scheduling overhead                                                                    |
| `spawn { }` block     | Direct subexpressions → task distribution + sync wait (aligned with [RFC-024](../accepted/024-concurrency-model.md)) |
| `native("symbol")`    | LLVM `declare external` + parameter marshalling (aligned with [RFC-026](./026-ffi-core-mechanism.md))                |
| `.drop` destructor    | RAII cleanup code insertion (aligned with [RFC-009](../accepted/009-ownership-model.md))                             |
| `&T` / `&mut T` token | Zero-size type, disappears after compilation                                                                         |
| `ref T` shared        | `{ refcount_ptr, data_ptr }` fat pointer, compiler automatically selects Rc/Arc                                      |

**Relationship with RFC-024**: RFC-024 defines the **user semantics** of spawn blocks (direct
subexpressions create tasks, synchronous blocking wait). This document defines how these semantics
**compile to machine code**.

**Relationship with RFC-026**: RFC-026 defines the **user syntax** for FFI (`native()`, `[0]` method
binding, `.drop`). This document defines how FFI calls **generate LLVM IR**.

---

## Motivation

### Why Do We Need an LLVM AOT Compiler?

Currently, YaoXiang only has an interpreter as the execution backend:

| Problem     | Impact                                              |
| ----------- | --------------------------------------------------- |
| Performance | Interpretation is 10-100x slower                    |
| Deployment  | Requires bundled interpreter/runtime                |
| Production  | Interpreter unsuitable for perf-sensitive scenarios |

### LLVM in the Dual-Backend Model

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 defines the dual-backend architecture:

```
                    ┌─────────────────────┐
                    │   Compilation Frontend (Unified)  │
                    │   Lexer → Parser     │
                    │   → TypeCheck        │
                    │   → spawn analysis   │
                    │   → Escape analysis  │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │   VM Backend (Dev)│     │  LLVM Backend (Prod)│
      │   IR → Interpret  │     │  IR → Native Code   │
      │   Step debugging  │     │  Link scheduler lib │
      │   Fast iteration  │     │  Output .exe        │
      └───────────────────┘     └───────────────────┘
```

Both backends have **identical behavior** — the only difference is in execution method. The same
source code, same type checking, same spawn analysis result.

---

## Proposal

### 1. Compiler Architecture

The LLVM backend is at the final stage of the compilation pipeline, receiving IR from the frontend
and generating native code:

```
Source Code
  → Lexer / Parser (frontend/core/)
  → TypeCheck + spawn analysis (frontend/core/typecheck/)
  → IR Generation (middle/core/ir_gen.rs)
  → LLVM Codegen (backends/llvm/)
      ├── Type mapping: YaoXiang type → LLVM IR type
      ├── Function translation: IR instruction → LLVM IR instruction
      ├── spawn expansion: direct subexpression → task function + scheduler call
      ├── FFI expansion: native() call → declare + marshalling
      └── Destructor insertion: scope end → .drop() call
  → LLVM optimization + target code generation
  → Link runtime static library → executable file
```

### 2. Compilation Flow

```
Phase 1: Frontend (shared with VM backend)
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

| YaoXiang Type    | LLVM IR Type                  | Notes                                                         |
| ---------------- | ----------------------------- | ------------------------------------------------------------- |
| `Int`            | `i64`                         | Default 64-bit signed integer                                 |
| `Int32`          | `i32`                         | Explicit 32-bit integer (mainly for FFI)                      |
| `Float`          | `f64`                         | Default 64-bit floating point                                 |
| `Float32`        | `f32`                         | Explicit 32-bit floating point (mainly for FFI)               |
| `Bool`           | `i1`                          | Boolean value                                                 |
| `Char`           | `i32`                         | Unicode code point                                            |
| `String`         | `{ i8*, i64 }`                | Pointer + byte length                                         |
| `Void`           | `{}`                          | Zero-size empty type                                          |
| `&T`             | —                             | Zero-size token, disappears after compilation, produces no IR |
| `&mut T`         | —                             | Zero-size token, disappears after compilation, produces no IR |
| `ref T`          | `{ i64*, T* }`                | Fat pointer (refcount pointer + data pointer)                 |
| `*T`             | `T*`                          | Raw pointer                                                   |
| `[T; N]`         | `[N x T]`                     | Fixed-size array                                              |
| `List(T)`        | `{ T*, i64, i64 }`            | Data pointer + length + capacity                              |
| Struct           | Corresponding LLVM struct     | Fields laid out in definition order                           |
| Record enum      | `{ i64, [max_payload_size] }` | Tag + union of max payload                                    |
| `?T`             | `{ i1, T }`                   | Has-value tag + data (general representation)                 |
| FFI opaque type  | `{ i8* }`                     | Wrapped C pointer                                             |
| Function pointer | `T (...)*`                    | Function pointer type                                         |

> **`&T` / `&mut T` Zero Runtime Overhead**: [RFC-009](../accepted/009-ownership-model.md) §2.7
> defines that the compiler internally assigns brand identifiers (compile-time unique integers) to
> tokens. After monomorphization and inlining, brands completely disappear — no token traces exist
> in the generated machine code.

#### 3.2 FFI Parameter Type Mapping

Aligned with [RFC-026](./026-ffi-core-mechanism.md) §2.2, adding LLVM IR column:

| C Type               | YaoXiang Type     | LLVM IR        | Notes                                       |
| -------------------- | ----------------- | -------------- | ------------------------------------------- |
| `int`                | `Int32`           | `i32`          |                                             |
| `long`               | `Int64`           | `i64`          |                                             |
| `float`              | `Float32`         | `f32`          |                                             |
| `double`             | `Float64`         | `f64`          |                                             |
| `char`               | `Char`            | `i32`          | C char → YaoXiang Char (Unicode compatible) |
| `char*`              | `String`          | `{ i8*, i64 }` | marshalling: C string → YaoXiang String     |
| `bool`               | `Bool`            | `i1`           |                                             |
| `size_t`             | `Uint`            | `i64`          |                                             |
| `void*`              | `*Void`           | `i8*`          |                                             |
| `struct T*`          | `T` (transparent) | `T*`           | Pass by pointer                             |
| `typedef struct T T` | `T` (opaque)      | `{ i8* }`      | Wrapped C pointer                           |

### 4. IR Normalization and Instruction Translation

#### 4.0 IR Normalization (Stack → Register)

The current IR (`src/middle/core/ir.rs`) contains stack operation instructions
(`Push`/`Pop`/`Dup`/`Swap`), designed for the bytecode VM. LLVM IR is in SSA form and does not
accept stack operations.

**Handling Strategy**: The LLVM path goes through a lightweight normalization pass before
instruction translation:

| Stack Instruction | Normalization Strategy                              |
| ----------------- | --------------------------------------------------- |
| `Push(r)`         | Record `stack.push(r)`, produce no IR               |
| `Pop(r)`          | `r = stack.pop()`, produce `load` (from stack slot) |
| `Dup`             | `stack.push(stack.top())`, produce no IR            |
| `Swap`            | Swap top two elements, produce no IR                |

After normalization, all operands become register/local variable references, and all stack
operations are eliminated. This pass executes as the first step in `translator.rs`.

> **Why not eliminate stack instructions at the IR level?** Because the VM backend needs stack
> semantics. Normalizing at the LLVM translation entry point preserves IR sharing between both
> backends — each backend consumes the same IR according to its own needs.
>
> **Prerequisite**: The IR generation phase guarantees stack balance — all control flow paths
> reaching the same program point have the same stack depth (the VM bytecode backend relies on the
> same prerequisite, otherwise bytecode execution would fail). The normalization pass does not check
> this prerequisite; violations result in undefined behavior in the LLVM backend.

#### 4.1 Instruction Translation Table

Below lists the LLVM IR translation strategy for each variant in the `Instruction` enum. Instruction
names match exactly with `src/middle/core/ir.rs`.

**Arithmetic Instructions**:

| IR Instruction          | LLVM IR                                 | Notes                               |
| ----------------------- | --------------------------------------- | ----------------------------------- |
| `Add { dst, lhs, rhs }` | `add` (integer) / `fadd` (float)        | Select integer or float add by type |
| `Sub { dst, lhs, rhs }` | `sub` / `fsub`                          |                                     |
| `Mul { dst, lhs, rhs }` | `mul` / `fmul`                          |                                     |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv`                | Signed/unsigned/float div           |
| `Mod { dst, lhs, rhs }` | `srem` / `urem`                         | Signed/unsigned modulo              |
| `Neg { dst, src }`      | `sub 0, src` (integer) / `fneg` (float) |                                     |

**Bitwise Instructions**:

| IR Instruction          | LLVM IR | Notes                  |
| ----------------------- | ------- | ---------------------- |
| `And { dst, lhs, rhs }` | `and`   |                        |
| `Or { dst, lhs, rhs }`  | `or`    |                        |
| `Xor { dst, lhs, rhs }` | `xor`   |                        |
| `Shl { dst, lhs, rhs }` | `shl`   | Left shift             |
| `Shr { dst, lhs, rhs }` | `lshr`  | Logical right shift    |
| `Sar { dst, lhs, rhs }` | `ashr`  | Arithmetic right shift |

**Comparison Instructions**:

| IR Instruction         | LLVM IR                 | Notes |
| ---------------------- | ----------------------- | ----- |
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp oeq`  |       |
| `Ne { dst, lhs, rhs }` | `icmp ne` / `fcmp one`  |       |
| `Lt { dst, lhs, rhs }` | `icmp slt` / `fcmp olt` |       |
| `Le { dst, lhs, rhs }` | `icmp sle` / `fcmp ole` |       |
| `Gt { dst, lhs, rhs }` | `icmp sgt` / `fcmp ogt` |       |
| `Ge { dst, lhs, rhs }` | `icmp sge` / `fcmp oge` |       |

**Control Flow Instructions**:

| IR Instruction          | LLVM IR                                     | Notes                |
| ----------------------- | ------------------------------------------- | -------------------- |
| `Jmp(label)`            | `br label %L`                               | Unconditional jump   |
| `JmpIf(cond, label)`    | `br i1 %cond, label %L, label %fallthrough` | Conditional jump     |
| `JmpIfNot(cond, label)` | `br i1 %cond, label %fallthrough, label %L` | Conditional not jump |
| `Ret(Some(v))`          | `ret T %v`                                  | Has return value     |
| `Ret(None)`             | `ret void`                                  | No return value      |

**Call Instructions**:

| IR Instruction                             | LLVM IR                                | Notes                                   |
| ------------------------------------------ | -------------------------------------- | --------------------------------------- |
| `Call { dst, func, args }`                 | `%r = call T @func(...)`               | Static call                             |
| `CallVirt { dst, obj, method_name, args }` | vtable GEP + `call` (function pointer) | Virtual method call, via vtable lookup  |
| `CallDyn { dst, func, args }`              | `%r = call T %func(...)`               | Dynamic call (closure/function pointer) |
| `TailCall { func, args }`                  | `musttail call` / `tail call`          | Tail call optimization                  |

**Memory Instructions**:

| IR Instruction                        | LLVM IR                                                    | Notes                                                                       |
| ------------------------------------- | ---------------------------------------------------------- | --------------------------------------------------------------------------- |
| `Move { dst, src }`                   | —                                                          | After normalization becomes register copy; SSA construction eliminates most |
| `Load { dst, src }`                   | `%v = load T, T* %src`                                     |                                                                             |
| `Store { dst, src }`                  | `store T %src, T* %dst`                                    |                                                                             |
| `Alloc { dst, size }`                 | `%p = alloca T` (stack) / `call @malloc` (escaped to heap) | Escape analysis decides allocation location                                 |
| `Free(ptr)`                           | `call @free(%ptr)` (heap) / — (stack, auto-reclaimed)      |                                                                             |
| `AllocArray { dst, size, elem_size }` | `%p = alloca [N x T]` (stack) / `call @malloc` (heap)      |                                                                             |

**Struct/Array Access Instructions**:

| IR Instruction                            | LLVM IR                                                | Notes                                |
| ----------------------------------------- | ------------------------------------------------------ | ------------------------------------ |
| `LoadField { dst, src, field }`           | `%ptr = getelementptr T, T* %src, 0, field` + `load`   |                                      |
| `StoreField { dst, field, src }`          | `%ptr = getelementptr T, T* %dst, 0, field` + `store`  |                                      |
| `LoadIndex { dst, src, index }`           | `%ptr = getelementptr T, T* %src, 0, %index` + `load`  |                                      |
| `StoreIndex { dst, index, src }`          | `%ptr = getelementptr T, T* %dst, 0, %index` + `store` |                                      |
| `CreateStruct { dst, type_name, fields }` | `insertvalue` chain                                    | Construct LLVM struct in field order |

**Type Conversion Instructions**:

| IR Instruction                   | LLVM IR                                                                                                     | Notes                                                                 |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `Cast { dst, src, target_type }` | `bitcast` / `trunc` / `zext` / `sext` / `fptrunc` / `fpext` / `sitofp` / `fptosi` / `inttoptr` / `ptrtoint` | Select appropriate cast instruction by source/target type combination |
| `TypeTest(val, type)`            | —                                                                                                           | Compile-time type test, generates `icmp eq` comparing type tag        |

**Ownership and Borrowing Instructions**:

| IR Instruction                 | LLVM IR                                              | Notes                                                                        |
| ------------------------------ | ---------------------------------------------------- | ---------------------------------------------------------------------------- |
| `Borrow { dst, src, mutable }` | —                                                    | **Zero-size token, completely disappears after compilation**, produces no IR |
| `Release(val)`                 | —                                                    | **Zero-size token, completely disappears after compilation**                 |
| `Move { dst, src }`            | —                                                    | Ownership transfer, after normalization becomes register copy                |
| `Drop(val)`                    | `call void @T.drop(T* %val)`                         | Call type's destructor function (see §7)                                     |
| `ShareRef { dst, src }`        | `call %T* @Arc_new(%src)` / `call %T* @Rc_new(%src)` | Compiler automatically selects Arc/Rc based on cross-thread usage            |
| `ArcNew { dst, src }`          | `call %T* @Arc_new(%src)`                            | Atomic refcount = 1                                                          |
| `ArcClone { dst, src }`        | `call %T* @Arc_clone(%src)`                          | Atomic increment refcount                                                    |
| `ArcDrop(val)`                 | `call void @Arc_drop(%val)`                          | Atomic decrement + conditional free                                          |

**Concurrency Instructions**:

| IR Instruction                     | LLVM IR                           | Notes                                                         |
| ---------------------------------- | --------------------------------- | ------------------------------------------------------------- |
| `Spawn { closures, plan, result }` | Expand to scheduler call sequence | See §5, runtime `task_spawn` + `task_wait_all`                |
| `Yield`                            | —                                 | On AOT path, spawn blocks sync-wait, no yield needed; ignored |

**unsafe Blocks and Raw Pointer Instructions**:

| IR Instruction            | LLVM IR                                                 | Notes                                   |
| ------------------------- | ------------------------------------------------------- | --------------------------------------- |
| `UnsafeBlockStart`        | —                                                       | **Compile-time marker, produces no IR** |
| `UnsafeBlockEnd`          | —                                                       | **Compile-time marker, produces no IR** |
| `PtrFromRef { dst, src }` | `%p = ptrtoint T* %src to i64` (or direct pointer copy) |                                         |
| `PtrDeref { dst, src }`   | `%v = load T, T* %src`                                  |                                         |
| `PtrStore { dst, src }`   | `store T %src, T* %dst`                                 |                                         |
| `PtrLoad { dst, src }`    | `%v = load T, T* %src`                                  |                                         |

**String Instructions**:

| IR Instruction                      | LLVM IR                                     | Notes                                          |
| ----------------------------------- | ------------------------------------------- | ---------------------------------------------- |
| `StringLength { dst, src }`         | `%len = extractvalue { i8*, i64 } %src, 1`  | String is `{ ptr, len }`, length is at field 1 |
| `StringConcat { dst, lhs, rhs }`    | `call String @yx_string_concat(%lhs, %rhs)` | Runtime helper function                        |
| `StringGetChar { dst, src, index }` | `getelementptr` + `load i32`                | Includes bounds checking                       |
| `StringFromInt { dst, src }`        | `call String @yx_string_from_int(%src)`     | Runtime helper function                        |
| `StringFromFloat { dst, src }`      | `call String @yx_string_from_f64(%src)`     | Runtime helper function                        |

**Closure Instructions**:

| IR Instruction                           | LLVM IR                                                                  | Notes                         |
| ---------------------------------------- | ------------------------------------------------------------------------ | ----------------------------- |
| `MakeClosure { dst, func: String, env }` | Allocate closure struct + fill function pointer (lookup by name) and env | `{ fn_ptr, env_fields... }`   |
| `LoadUpvalue { dst, upvalue_idx }`       | `%v = extractvalue %env, upvalue_idx`                                    | Read upvalue from closure env |
| `StoreUpvalue { src, upvalue_idx }`      | `%env = insertvalue %env, %src, upvalue_idx`                             | Write to closure env          |
| `CloseUpvalue(val)`                      | Copy upvalue from stack to heap                                          |                               |

**Other Instructions**:

| IR Instruction                  | LLVM IR                                       | Notes                       |
| ------------------------------- | --------------------------------------------- | --------------------------- |
| `HeapAlloc { dst, type_id }`    | `call i8* @malloc(i64 size)` + type tag write | Heap allocation + type info |
| `NewDict { dst, keys, values }` | `call Dict @yx_dict_new(%keys, %values)`      | Runtime helper function     |

> **Note**: `Push`/`Pop`/`Dup`/`Swap` are eliminated during the §4.0 normalization phase and do not
> appear in the translation table. `Borrow`/`Release` are zero-size compile-time tokens and produce
> no machine code.

### 5. spawn Block Code Generation

Aligned with [RFC-024](../accepted/024-concurrency-model.md), spawn block compilation is divided
into the following steps.

#### 5.1 Semantic Review

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // Direct subexpression → task 1
    t2 = fetch("url2"),   // Direct subexpression → task 2
    return (t1, t2)       // Sync wait, assemble results
}
```

**Rules** (RFC-024 §2.1):

- **Direct subexpressions** (top-level comma-separated statements) of a spawn block create parallel
  tasks
- Expressions nested inside `{}` are not direct subexpressions and do not become independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning

#### 5.2 Compilation Steps

```
Step 1: Identify direct subexpressions
  Traverse spawn block body, collect top-level statements

Step 2: Dependency analysis
  For each direct subexpression, analyze which variables it references that are produced by previous tasks
  No dependency → can be immediately parallel scheduled
  Has dependency → queue and wait for dependent task to complete

Step 3: Resource conflict detection (RFC-024 §2.5)
  Check if same resource type instance is used by multiple tasks
  Same instance conflict → mark for serial execution order

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

; Sync wait for all tasks
call @runtime_task_wait_all(%tasks, %task_count)

; Assemble return value
%r1 = call @runtime_task_result(%tasks[0])
%r2 = call @runtime_task_result(%tasks[1])
ret { %r1, %r2 }
```

#### 5.4 Dependent Tasks

```yaoxiang
result = spawn {
    data = fetch("url"),       // Task 1: no dependencies
    processed = parse(data),  // Task 2: depends on task 1's data
    return processed
}
```

The compiler detects that `parse(data)` references the `data` produced by task 1, and marks the
dependency when generating scheduling code:

```llvm
; Task 2 created with dependency on task 1
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
;                                                              ↑
;                                                 dependency on task 0 (fetch) completed
```

#### 5.5 Resource Type Auto-Serialization

Resource types defined in [RFC-024 §2.5](../accepted/024-concurrency-model.md) (`FilePath`,
`HttpUrl`, `DBUrl`, `Console`, and user-defined resource types) are automatically serialized in
spawn blocks:

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // Uses SqliteDb (resource type)
    r2 = db.exec("INSERT ...")    // Same instance → auto serialize
}
```

The compiler detects that the same resource instance is used by two tasks and generates serial
dependencies:

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

The compiler expands to N independent tasks (N = items length), constrained by maximum concurrency
limit.

### 6. FFI Code Generation

> ⚠️ **Dependency Note**: The **architecture** defined in this section (`native("x")` →
> `declare external @x` → marshalling wrapper → call) is stable and does not change when RFC-026
> syntax changes. The specific parameter marshalling rules table (§6.2) and opaque type layout
> (§6.3) reference RFC-026 definitions — if RFC-026's `native()` syntax or marshalling rules change,
> only update the corresponding mapping tables in this document; the architectural layer is
> unaffected. RFC-026 current status: **Under Review**, in the same `review/` directory as this
> document.
>
> **Acceptance Prerequisite**: Before this RFC is accepted, the parts of RFC-026 related to §6 of
> this document (`native()` declaration syntax, parameter marshalling rules, opaque type `{ i8* }`
> layout, `.drop` binding convention) should be frozen first or accepted together with 026.
> Otherwise, the mapping tables in §6.2/§6.3/§7 may be outdated before implementation.

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
- Wrapper function signature uses YaoXiang types internally, converts to C types

#### 6.2 Parameter Marshalling

| Direction                                  | Conversion                              |
| ------------------------------------------ | --------------------------------------- |
| YaoXiang `String` → C `char*`              | Extract `.ptr` field and pass           |
| YaoXiang `Int32` → C `int`                 | Pass directly (`i32`)                   |
| YaoXiang `*Void` → C `void*`               | Pass directly (`i8*`)                   |
| YaoXiang `T` (transparent) → C `struct T*` | Take address and pass                   |
| YaoXiang `T` (opaque) → C `struct T*`      | Extract pointer from `{ i8* }` and pass |

#### 6.3 Opaque Type LLVM Layout

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

**Layout Optimization**: When an opaque type has only one `handle: *Void` field, it can be optimized
to directly use `i8*` (omit outer struct). The optimized ABI is completely consistent with C
pointers, with zero marshalling overhead. This optimization is enabled by default with no user
perception.

#### 6.4 ?T Nullable Return Value LLVM Representation

FFI nullable return values defined in [RFC-026](./026-ffi-core-mechanism.md) §7.6:

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

General LLVM representation: `{ i1, { i8* } }` — has-value tag + data.

**Optimization for FFI null pointers**: If `T` in `?T` is an opaque type (internally a pointer), the
compiler uses **null pointer = None** optimization:

```llvm
; Optimized LLVM representation: directly use nullable pointer
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

This optimization makes `?SqliteDb` FFI calls **zero additional overhead** — completely equivalent
to C's null checking.

#### 6.5 yx-bindgen Integration

Bindings auto-generated by [yx-bindgen](./026-ffi-core-mechanism.md) §6 are treated as regular
YaoXiang source code at compile time. The compiler doesn't need to know the code came from bindgen —
`native()` declarations and `unsafe {}` type definitions are handled identically.

### 7. Destructor Code Generation

Aligned with the RAII semantics in [RFC-009](../accepted/009-ownership-model.md) and the `.drop`
convention in [RFC-026](./026-ffi-core-mechanism.md) §7.

#### 7.1 .drop Binding Recognition

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

The compiler recognizes `.drop` bindings and marks the destructor function pointer in type metadata.

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

**Insertion locations**:

- Normal scope end (`}`)
- Early return (before `return`)
- `?` error propagation path (before `?`)
- spawn block end (destructors for variables inside tasks)

#### 7.3 Move and Destructors

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: ownership transferred to db2
// db is now invalid; no drop inserted for db here
// ← scope ends: only db2 gets drop inserted
```

The compiler tracks Move semantics ([RFC-009](../accepted/009-ownership-model.md) §1) and only
inserts destructor calls at the final holder of each variable.

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

The compilation output contains the following components (specific struct definitions to be
determined during implementation):

- **Machine code**: LLVM-compiled object files (`.o`), containing all function translation results
- **spawn metadata**: Task function pointers, dependency relationships, resource conflict
  serialization pairs for each spawn block
- **FFI symbol table**: External C symbol references (symbol name + whether weak reference)
- **Entry point table**: List of entry functions for the executable
- **Type information**: Reflection metadata, written to `.reflect` section, mmap'd on demand at
  runtime

### 9. Runtime Library

Aligned with [RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md), the runtime is linked as
a **static library** into the final exe.

```
Internal structure of final exe:

┌────────────────────────────────────────────┐
│  User code (native machine code)            │
│  ├── Normal functions (sequential execution)│
│  ├── spawn block expansion (task functions + scheduler calls)│
│  ├── FFI marshalling wrapper functions     │
│  └── RAII destructor code                  │
├────────────────────────────────────────────┤
│  Runtime static library (~500KB-1MB, depending on platform and feature selection)│
│  ├── Thread pool (num_workers)             │
│  ├── Event loop (libuv / io_uring)         │
│  ├── Work-stealing queue (Full Runtime only)│
│  ├── Memory allocator (jemalloc / mimalloc)│
│  └── Reflection metadata (.reflect section, mmap'd on demand)│
│                                              │
│  No:                                         │
│  ❌ Bytecode interpreter                     │
│  ❌ JIT compiler                            │
│  ❌ GC                                       │
│  ❌ Virtual machine                          │
└────────────────────────────────────────────┘
```

**Key Design**: Task identification and dependency analysis for spawn blocks are completed at
compile time. The runtime only does "create task → dispatch to thread pool → wait for completion" —
fixed data structures, predictable behavior.

> **Difference from RFC-008 Size Estimate**: RFC-008 §4 estimated the scheduler at ~200-500KB,
> containing only task scheduling core. This document's 500KB-1MB estimate additionally includes
> memory allocator (jemalloc/mimalloc), event loop (libuv/io_uring), and reflection metadata
> section. Actual size depends on platform and feature selection; precise numbers will be given
> during implementation.

**Three-Layer Runtime and LLVM Relationship** (aligned with RFC-008 §1):

| Runtime      | LLVM AOT Behavior                                                                                                 |
| ------------ | ----------------------------------------------------------------------------------------------------------------- |
| **Embedded** | No spawn support, directly generate sequential machine code                                                       |
| **Standard** | Supports spawn blocks, DAG within spawn blocks + single-threaded scheduling (num_workers=1)                       |
| **Full**     | Supports spawn blocks, DAG within spawn blocks + multi-threaded scheduling (num_workers>1), supports WorkStealing |

---

## Detailed Design

### Module Directory Structure

Aligned with the directory layout in [RFC-008](../accepted/008-runtime-concurrency-model.md) §6.
`[! Planned]` marks indicate files/directories not yet created, to be introduced during
implementation of this RFC.

```
src/
├── frontend/                          # Compilation frontend (shared by all backends)
│   ├── core/
│   │   ├── spawn/                     # spawn module (concurrency analysis shared by VM and LLVM backends)
│   │   │   ├── mod.rs                 # spawn module entry
│   │   │   ├── placement.rs           # spawn position legality check
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
│       │   ├── translator.rs          # IR → bytecode translation (VM backend)
│       │   ├── emitter.rs             # Bytecode emission + jump backpatching (VM backend)
│       │   ├── buffer.rs              # Constant pool + bytecode buffer (VM backend)
│       │   ├── bytecode.rs            # Bytecode format definition + serialization (VM backend)
│       │   ├── flow.rs                # Register allocation + label generation + symbol table (VM backend)
│       │   └── operand.rs             # Operand parsing (VM backend)
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
│   └── runtime/                       # Compiled runtime (static library linked into exe)
│       ├── engine.rs                  # Task scheduling engine
│       ├── facade.rs                  # Public interface
│       └── task.rs                    # Task representation
│
└── util/
    └── diagnostic/                    # Error diagnostics (shared)
```

> **Key Change**: Spawn block analysis (task identification, dependency analysis, resource conflict
> detection) will be implemented in `frontend/core/spawn/` (frontend shared). The existing
> `frontend/core/typecheck/passes/spawn_placement.rs` (spawn position checking) will be migrated to
> `frontend/core/spawn/placement.rs`, see RFC-024 for details. The LLVM backend only consumes
> analysis results and generates corresponding scheduling code.
>
> **Current Status Note**: The current `middle/passes/codegen/` files `buffer.rs`, `emitter.rs`,
> `bytecode.rs`, `flow.rs`, `operand.rs` serve bytecode generation for the VM backend
> (`CodegenContext::generate()` → `BytecodeFile`). The LLVM backend will be implemented in
> `backends/llvm/`, at the same level as the interpreter backend and runtime — both share the same
> `ModuleIR` input, outputting different target formats (bytecode vs native code).

### Platform ABI Support

| Platform       | Target Triple              | Output Format | Calling Convention (FFI Default) |
| -------------- | -------------------------- | ------------- | -------------------------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu` | ELF           | System V AMD64                   |
| macOS x86_64   | `x86_64-apple-darwin`      | Mach-O        | System V AMD64                   |
| macOS ARM64    | `aarch64-apple-darwin`     | Mach-O        | ARM64 AAPCS                      |
| Windows x86_64 | `x86_64-pc-windows-msvc`   | COFF          | Microsoft x64                    |

FFI calls use the platform's C calling convention by default. Users can override with options like
`native("symbol", cc = "stdcall")` (aligned with future extensions in
[RFC-026](./026-ffi-core-mechanism.md)).

### Floating-Point Semantic Consistency (VM ↔ LLVM)

The core promise of the dual-backend architecture is that VM (development/debugging) and LLVM
(production/release) behave identically. Floating-point operations have potential inconsistency
points between the two execution modes:

| Scenario         | Risk                                                        | Strategy                                                                                     |
| ---------------- | ----------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| NaN propagation  | VM and LLVM may handle NaN sign bit and payload differently | Compiler normalizes NaN representation at IR level; NaN comparisons uniformly use `fcmp uno` |
| Rounding mode    | LLVM defaults round-to-nearest-even, VM depends on host CPU | Don't expose non-default rounding modes; VM and LLVM both use RTNE                           |
| Division by zero | IEEE 754 defines ±Inf, but some platforms may trap          | Debug mode checks division by zero and reports diagnostics; release mode follows IEEE 754    |
| `-0.0` vs `+0.0` | Comparison operations may not be equivalent                 | Uniformly use IEEE 754 rules: `+0.0 == -0.0`                                                 |
| Denormal numbers | Some platforms flush-to-zero                                | LLVM doesn't enable `denormal-fp-math` attribute; preserve full IEEE 754 semantics           |

> **Testing Strategy**: Implement a cross-backend floating-point consistency test suite — same
> YaoXiang source code executes on both VM and LLVM backends, with value-by-value output comparison.
> This test suite is a mandatory CI gate.

---

## Trade-offs

### Advantages

1. **Performance**: AOT compilation is 10-100x faster than interpretation
2. **Unified frontend**: VM and LLVM share the same frontend, behavior is completely consistent
3. **Zero scheduling overhead**: Normal code directly generates sequential machine code, no DAG
   overhead outside spawn blocks
4. **Static linking**: No external runtime dependencies, single exe for deployment
5. **Zero GC**: RAII deterministic destruction, no pauses
6. **FFI zero overhead**: `?T` null pointer optimization, opaque type layout optimization — FFI call
   cost is equivalent to C
7. **Compile-time analysis**: Task identification and dependency analysis for spawn blocks completed
   at compile time; runtime only executes

### Disadvantages

1. **LLVM integration complexity**: Requires deep understanding of inkwell API and LLVM IR
2. **Compile time**: AOT compilation is slower than interpreter (one-time cost)
3. **Debugging experience**: Native code debugging requires DWARF/PDB symbol support (compiler must
   generate debug info)
4. **Incremental compilation**: Additional design needed for large projects
5. **Floating-point semantic consistency**: VM and LLVM may have differences in boundary behaviors
   like NaN propagation, rounding mode, division by zero; normalized strategies ensure dual-backend
   consistency (see §10)

### Consistency with Related RFCs

| RFC                                   | Consistency                                                               |
| ------------------------------------- | ------------------------------------------------------------------------- |
| RFC-024 spawn block concurrency model | ✅ spawn block direct subexpressions → task distribution                  |
| RFC-008 Runtime architecture          | ✅ Dual backend + scheduler static library + module directory structure   |
| RFC-009 Ownership model v9            | ✅ `&T`/`&mut T` tokens (zero-size), `ref T` (fat pointer), `?T` (Option) |
| RFC-026 FFI core mechanism            | ✅ `native()` → declare + marshalling, `.drop` → RAII cleanup             |

---

## Alternative Approaches

| Approach                             | Description               | Why Not Chosen                                                            |
| ------------------------------------ | ------------------------- | ------------------------------------------------------------------------- |
| Interpreter only                     | No AOT needed             | Insufficient performance                                                  |
| Pure static compilation (no runtime) | No scheduler linking      | Spawn blocks need runtime task scheduling                                 |
| Cranelift backend                    | Faster compilation        | Inferior runtime performance vs LLVM; consider as future optional backend |
| Link external LLVM runtime           | Use LLVM built-in runtime | Introduces unnecessary dependencies                                       |

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

- [ ] Implement `&T`/`&mut T` tokens (zero-size, disappears after compilation)
- [ ] Implement `ref T` (fat pointer `{ i64*, T* }`)
- [ ] Implement `?T` (`{ i1, T }` tagged union)
- [ ] Implement `List(T)` (`{ T*, i64, i64 }`)
- [ ] Implement Move semantics tracking (for destructor insertion decisions)

#### Phase 4: spawn Block Code Generation

- [ ] Consume analysis results from `spawn_placement.rs`
- [ ] Direct subexpressions → task function generation
- [ ] Dependent task scheduling code generation
- [ ] Resource conflict serialization
- [ ] spawn for expansion

#### Phase 5: FFI Code Generation

- [ ] `native()` → `declare external` (`ffi.rs`)
- [ ] Parameter marshalling / return value unmarshalling
- [ ] Opaque type layout (including single-field optimization)
- [ ] `?T` null pointer optimization (FFI-specific)

#### Phase 6: Destructor Code Generation

- [ ] `.drop` binding recognition
- [ ] Scope end cleanup insertion (reverse order) (`drop.rs`)
- [ ] Early return path cleanup
- [ ] `?` error propagation path cleanup

#### Phase 7: Runtime Library Linking

- [ ] Implement runtime functions like `runtime_task_spawn` / `runtime_task_wait_all`
- [ ] Link runtime static library
- [ ] End-to-end integration tests

### Dependencies

- RFC-024 (spawn block concurrency) → input for Phase 4
- RFC-009 v9 (ownership) → input for Phases 3, 6
- RFC-008 (runtime architecture) → input for Phase 7
- RFC-026 (FFI mechanism) → input for Phase 5

---

## Related Work

### Lazy Task Creation (1990)[^1]

| Attribute       | Description                                                         |
| --------------- | ------------------------------------------------------------------- |
| Institution     | MIT                                                                 |
| Authors         | James R. Larus, Robert H. Halstead Jr.                              |
| Core            | Lazily create subtasks, create on demand                            |
| Reference Value | Theoretical basis for on-demand task scheduling within spawn blocks |

**Core Idea**: Instead of immediately creating tasks, create them lazily. When the parent task needs
a subtask's value, only then is the subtask created. This solves the performance overhead problem of
fine-grained parallel tasks[^1]. YaoXiang's spawn block scheduling draws from this idea — tasks are
identified at compile time but dispatched to the thread pool on demand at runtime.

### Lazy Scheduling (2014)[^2]

| Attribute       | Description                                              |
| --------------- | -------------------------------------------------------- |
| Institution     | University of Maryland                                   |
| Authors         | Tzannes, Caragea                                         |
| Core            | Runtime-adaptive scheduling, no extra state              |
| Reference Value | Reference for Full Runtime WorkStealing scheduler design |

### SISAL Language[^3]

| Attribute       | Description                                                       |
| --------------- | ----------------------------------------------------------------- |
| Institution     | Lawrence Livermore National Laboratory (LLNL)                     |
| Core            | Single-assignment language, Dataflow graph, implicit parallelism  |
| Reference Value | Proof of feasibility of Dataflow model in industrial applications |

**Key Difference**: SISAL's parallelism is **implicit** — the language has single-assignment
semantics, and the compiler automatically analyzes the whole-program data dependency graph to
determine parallelism. YaoXiang's parallelism is **explicit** — users mark parallel regions with
`spawn {}` blocks, and the compiler only analyzes dependencies within spawn blocks. This avoids
SISAL's whole-program analysis complexity while retaining user control over parallel behavior.

### Mul-T Parallel Scheme[^4]

| Attribute       | Description                                         |
| --------------- | --------------------------------------------------- |
| Institution     | MIT                                                 |
| Core            | Future construct, Lazy Task Creation implementation |
| Reference Value | Specific implementation reference                   |

### Comparison Summary

| Technology             | Lazy Creation | Parallelism Marker           | Analysis Scope         | Ownership                   |
| ---------------------- | ------------- | ---------------------------- | ---------------------- | --------------------------- |
| Lazy Task Creation[^1] | ✅            | Implicit                     | Whole program          | N/A                         |
| Lazy Scheduling[^2]    | ✅            | Implicit                     | Whole program          | N/A                         |
| SISAL[^3]              | ✅            | Implicit (single-assignment) | Whole program          | N/A                         |
| Mul-T[^4]              | ✅            | Explicit (future)            | Call site              | N/A                         |
| **YaoXiang**           | ✅            | **Explicit (spawn block)**   | **Within spawn block** | **✅ (Move + token + ref)** |

**YaoXiang's Innovation**: Elevating parallelism markers from "every function call" (future) to
"structured block" (spawn). Users write normal code, placing spawn blocks where parallelism is
needed. Analysis scope is constrained within spawn blocks, compilation is efficient, and behavior is
controllable.

---

## Appendices

### Appendix A: Comparison with Rust async

| Feature            | Rust async                              | YaoXiang LLVM AOT                                                 |
| ------------------ | --------------------------------------- | ----------------------------------------------------------------- |
| Compilation output | State machine + machine code            | Machine code + spawn task metadata                                |
| Runtime            | tokio                                   | Statically linked scheduler (~500KB-1MB)                          |
| Concurrency marker | async/await keywords                    | `spawn { }` block                                                 |
| Task creation      | State machine generated at compile time | Direct subexpressions identified at compile time → task functions |
| Color function     | async infection                         | **No function coloring**                                          |
| Sync wait          | `.await`                                | spawn block auto sync-blocks                                      |
| Memory management  | GC (runtime)                            | **RAII (deterministic)**                                          |
| Sharing mechanism  | `Arc::new()` + manual Weak              | **`ref` keyword (compiler auto-selects Rc/Arc)**                  |

### Appendix B: Design Decision Record

| Decision                    | Determination                                                                      | Date       |
| --------------------------- | ---------------------------------------------------------------------------------- | ---------- |
| Adopt LLVM AOT              | Direct codegen, no excessive abstraction                                           | 2026-02-15 |
| Concurrency model alignment | Align with RFC-024 spawn block direct subexpression model                          | 2026-06-10 |
| DAG analysis scope          | Within spawn block, not across spawn blocks (aligned with RFC-024)                 | 2026-06-05 |
| Ownership model alignment   | Align with RFC-009 v9: `&T`/`&mut T` tokens + `ref` keyword                        | 2026-06-10 |
| Dual backend model          | VM (development) + LLVM (production), aligned with RFC-008                         | 2026-05-11 |
| Scheduler form              | Static library linked into exe, ~500KB-1MB (depending on platform/features), no GC | 2026-05-11 |
| FFI code generation         | Integrate RFC-026: `native()` declare + marshalling                                | 2026-06-10 |
| Destructors                 | `.drop` → RAII cleanup insertion, aligned with RFC-026 §7                          | 2026-06-10 |
| Side effect handling        | Remove `@IO`/`@Pure` inference, use RFC-024 resource types                         | 2026-06-10 |
| Reflection metadata         | Compile into exe .reflect section, mmap loaded on demand                           | 2026-05-11 |
| Paper citations             | Keep Lazy Task Creation etc., clarify YaoXiang's differences                       | 2026-02-16 |

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
- [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
- [RFC-026: FFI Core Mechanism](./026-ffi-core-mechanism.md)

---

## Lifecycle and Disposition

| Status           | Location                    | Description                                |
| ---------------- | --------------------------- | ------------------------------------------ |
| **Draft**        | `docs/design/rfc/`          | Author draft, awaiting review              |
| **Under Review** | `docs/design/rfc/review/`   | Open for community discussion and feedback |
| **Accepted**     | `docs/design/rfc/accepted/` | Becomes official design document           |
| **Rejected**     | `docs/design/rfc/`          | Preserved in RFC directory                 |

> Current status: **Accepted** — aligned with RFC-024 spawn block concurrency model, RFC-009 v9
> ownership model, RFC-026 FFI mechanism
