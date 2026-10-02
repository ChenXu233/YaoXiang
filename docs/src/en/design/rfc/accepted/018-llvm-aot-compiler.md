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

> **⚠️ Implementation Status (verified 2026-10-02)**: This RFC is still in "Accepted" status, but
> **implementation has not yet started** — the repository `Cargo.toml` has no `inkwell` / `llvm` /
> `cranelift` dependencies, and `src/backends/` only contains `common` / `interpreter` / `runtime`;
> none of the "Implementation Strategy" items are checked. Schedule TBD.
>
> **References**:
>
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime Concurrency Model and Scheduler Decoupling Design](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
> - [RFC-026: FFI Core Mechanism](./026-ffi-core-mechanism.md)
> - [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)

> **Deprecated**:
>
> - The old "bottom-up automatic DAG analysis" model — replaced by the RFC-024 direct subexpression
>   model for spawn blocks
> - `@IO`/`@Pure` implicit side-effect inference — replaced by the RFC-024 resource type mechanism
> - `Arc(T)` type mapping — replaced by the RFC-009 v9 `ref` keyword

## Summary

This document designs the LLVM AOT (Ahead-of-Time) compiler for the YaoXiang language. The LLVM
backend and the VM backend (interpreter) share the same compilation front-end, forming the
dual-backend architecture defined by [RFC-008](../accepted/008-runtime-concurrency-model.md): the VM
is used for development and debugging, while LLVM is used for production releases.

**Core Responsibilities**:

```
Source Code → Front-End (shared) → IR → LLVM Codegen → .o → Link Scheduler Static Library → exe
```

The compiler translates YaoXiang source code into native machine code, where:

| Language Feature       | Compilation Strategy                                                                                                   |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Regular code           | Sequential machine code, zero scheduling overhead                                                                      |
| `spawn { }` block      | Direct subexpression → task dispatch + synchronous wait (aligned with [RFC-024](../accepted/024-concurrency-model.md)) |
| `native("symbol")`     | LLVM `declare external` + argument marshalling (aligned with [RFC-026](./026-ffi-core-mechanism.md))                   |
| `.drop` destructor     | RAII cleanup code insertion (aligned with [RFC-009](../accepted/009-ownership-model.md))                               |
| `&T` / `&mut T` tokens | Zero-sized types, disappear after compilation                                                                          |
| `ref T` shared         | `{ refcount_ptr, data_ptr }` fat pointer, compiler auto-selects Rc/Arc                                                 |

**Relationship with RFC-024**: RFC-024 defines the **user semantics** of spawn blocks (direct
subexpressions create tasks, synchronously block and wait). This document defines how these
semantics **compile to machine code**.

**Relationship with RFC-026**: RFC-026 defines the **user syntax** for FFI (`native()`, `[0]` method
binding, `.drop`). This document defines how FFI calls **generate LLVM IR**.

---

## Motivation

### Why is an LLVM AOT Compiler Needed?

Currently, YaoXiang only has an interpreter as its execution backend:

| Problem                | Impact                                                        |
| ---------------------- | ------------------------------------------------------------- |
| Performance bottleneck | Interpretation is 10-100x slower than machine code            |
| Deployment complexity  | Needs to ship the interpreter and runtime                     |
| Production environment | Interpreter is unsuitable for performance-sensitive scenarios |

### LLVM in the Dual-Backend Model

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 defines the dual-backend architecture:

```
                    ┌─────────────────────┐
                    │   Compilation Front-End (unified)│
                    │   Lexer → Parser     │
                    │   → TypeCheck        │
                    │   → spawn analysis   │
                    │   → Escape analysis  │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │   VM Backend (Development) │     │  LLVM Backend (Production)  │
      │   IR → Interpretation      │     │  IR → Native Code           │
      │   Step Debugging           │     │  Link scheduler static lib  │
      │   Rapid Iteration          │     │  Output .exe                │
      └───────────────────┘     └───────────────────┘
```

Both backends have **exactly the same behavior** — the difference is only in the execution method.
The same source code, the same type checking, the same spawn analysis result.

---

## Proposal

### 1. Compiler Architecture

The LLVM backend sits at the final stage of the compilation pipeline, receiving IR from the
front-end and generating native code:

```
Source Code
  → Lexer / Parser (frontend/core/)
  → TypeCheck + spawn analysis (frontend/core/typecheck/)
  → IR Generation (middle/core/ir_gen.rs)
  → LLVM Codegen (backends/llvm/)
      ├── Type mapping: YaoXiang types → LLVM IR types
      ├── Function translation: IR instructions → LLVM IR instructions
      ├── spawn expansion: direct subexpressions → task functions + dispatch calls
      ├── FFI expansion: native() calls → declare + marshalling
      └── Destructor insertion: scope end → .drop() call
  → LLVM optimization + target code generation
  → Link runtime static library → executable
```

### 2. Compilation Flow

```
Phase 1: Front-End (shared with VM backend)
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
  - Output: executable file
```

### 3. Type Mapping

#### 3.1 YaoXiang → LLVM IR Type Mapping

| YaoXiang Type      | LLVM IR Type                  | Notes                                                          |
| ------------------ | ----------------------------- | -------------------------------------------------------------- |
| `Int`              | `i64`                         | Default 64-bit signed integer                                  |
| `Int32`            | `i32`                         | Explicit 32-bit integer (mainly for FFI)                       |
| `Float`            | `f64`                         | Default 64-bit float                                           |
| `Float32`          | `f32`                         | Explicit 32-bit float (mainly for FFI)                         |
| `Bool`             | `i1`                          | Boolean                                                        |
| `Char`             | `i32`                         | Unicode code point                                             |
| `String`           | `{ i8*, i64 }`                | Pointer + byte length                                          |
| `Void`             | `{}`                          | Zero-sized empty type                                          |
| `&T`               | —                             | Zero-sized token, disappears after compilation, produces no IR |
| `&mut T`           | —                             | Zero-sized token, disappears after compilation, produces no IR |
| `ref T`            | `{ i64*, T* }`                | Fat pointer (refcount pointer + data pointer)                  |
| `*T`               | `T*`                          | Raw pointer                                                    |
| `[T; N]`           | `[N x T]`                     | Fixed-length array                                             |
| `List(T)`          | `{ T*, i64, i64 }`            | Data pointer + length + capacity                               |
| Struct             | Corresponding LLVM struct     | Fields laid out in definition order                            |
| Discriminated enum | `{ i64, [max_payload_size] }` | Tag + union of max payload                                     |
| `?T`               | `{ i1, T }`                   | Has-value flag + data (general representation)                 |
| FFI opaque type    | `{ i8* }`                     | Wraps a C pointer                                              |
| Function pointer   | `T (...)*`                    | Function pointer type                                          |

> **`&T` / `&mut T` Zero Runtime Overhead**: [RFC-009](../accepted/009-ownership-model.md) §2.7
> defines that the compiler internally assigns brand identifiers (compile-time unique integers) to
> tokens. After monomorphization and inlining, the brands completely disappear — no token traces
> exist in the generated machine code.

#### 3.2 FFI Parameter Type Mapping

Aligned with [RFC-026](./026-ffi-core-mechanism.md) §2.2, with an added LLVM IR column:

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
| `struct T*`          | `T` (transparent) | `T*`           | Pass pointer                                |
| `typedef struct T T` | `T` (opaque)      | `{ i8* }`      | Wrap C pointer                              |

### 4. IR Normalization and Instruction Translation

#### 4.0 IR Normalization (Stack → Registers)

The current IR (`src/middle/core/ir.rs`) includes stack-manipulation instructions
(`Push`/`Pop`/`Dup`/`Swap`), which are designed for the bytecode VM. LLVM IR is in SSA form and does
not accept stack operations.

**Handling Strategy**: The LLVM path first goes through a lightweight normalization pass before
instruction translation:

| Stack Instruction | Normalization Strategy                              |
| ----------------- | --------------------------------------------------- |
| `Push(r)`         | Record `stack.push(r)`, produce no IR               |
| `Pop(r)`          | `r = stack.pop()`, produce `load` (from stack slot) |
| `Dup`             | `stack.push(stack.top())`, produce no IR            |
| `Swap`            | Swap top two stack elements, produce no IR          |

After normalization, all operands become references to registers or local variables, and stack
operations are completely eliminated. This pass runs as the first step of `translator.rs`.

> **Why not eliminate stack instructions at the IR level?** Because the VM backend needs stack
> semantics. Normalizing at the LLVM translation entry point keeps the IR shared between the two
> backends — each backend consumes the same IR according to its own needs.
>
> **Prerequisite**: The IR generation phase guarantees stack balance — at any program point, all
> control flow paths reach the same stack depth (the VM bytecode backend depends on this same
> prerequisite, otherwise bytecode execution will fail). The normalization pass does not verify this
> prerequisite; violating it results in undefined behavior in the LLVM backend.

#### 4.1 Instruction Translation Table

The LLVM IR translation strategy for each variant of the `Instruction` enum is listed below.
Instruction names are identical to those in `src/middle/core/ir.rs`.

**Arithmetic Instructions**:

| IR Instruction          | LLVM IR                                 | Notes                          |
| ----------------------- | --------------------------------------- | ------------------------------ |
| `Add { dst, lhs, rhs }` | `add` (integer) / `fadd` (float)        | Integer or float add by type   |
| `Sub { dst, lhs, rhs }` | `sub` / `fsub`                          |                                |
| `Mul { dst, lhs, rhs }` | `mul` / `fmul`                          |                                |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv`                | Signed/unsigned/float division |
| `Mod { dst, lhs, rhs }` | `srem` / `urem`                         | Signed/unsigned remainder      |
| `Neg { dst, src }`      | `sub 0, src` (integer) / `fneg` (float) |                                |

> **`Mod` row revision notice (noted 2026-09-22, along with RFC-011b)**: `srem` / `urem` are
> **truncated remainders** (sign follows dividend); "remainder" in the description column of this
> row is a term mix-up. [RFC-011b](./011b-operator-overloading.md) has confirmed that the
> established semantics of `%` is **mathematical modulo** (result sign follows divisor; the language
> reference priority table "multiply/divide/modulo" is the first evidence). After that semantics is
> landed, the mapping in this row needs to change to `srem` + sign correction sequence (or
> `sdiv`+`mul`+`sub` synthesis); the implementation side (interpreter `checked_rem`, constant
> folding, bytecode `I64_REM`) should be updated accordingly.

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
| `JmpIfNot(cond, label)` | `br i1 %cond, label %fallthrough, label %L` | Conditional not-jump |
| `Ret(Some(v))`          | `ret T %v`                                  | With return value    |
| `Ret(None)`             | `ret void`                                  | No return value      |

**Call Instructions**:

| IR Instruction                             | LLVM IR                                | Notes                                   |
| ------------------------------------------ | -------------------------------------- | --------------------------------------- |
| `Call { dst, func, args }`                 | `%r = call T @func(...)`               | Static call                             |
| `CallVirt { dst, obj, method_name, args }` | vtable GEP + `call` (function pointer) | Virtual call, looked up via vtable      |
| `CallDyn { dst, func, args }`              | `%r = call T %func(...)`               | Dynamic call (closure/function pointer) |
| `TailCall { func, args }`                  | `musttail call` / `tail call`          | Tail call optimization                  |

**Memory Instructions**:

| IR Instruction                        | LLVM IR                                                    | Notes                                                                           |
| ------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `Move { dst, src }`                   | —                                                          | After normalization, becomes register copy; SSA construction can eliminate most |
| `Load { dst, src }`                   | `%v = load T, T* %src`                                     |                                                                                 |
| `Store { dst, src }`                  | `store T %src, T* %dst`                                    |                                                                                 |
| `Alloc { dst, size }`                 | `%p = alloca T` (stack) / `call @malloc` (escapes to heap) | Escape analysis decides allocation location                                     |
| `Free(ptr)`                           | `call @free(%ptr)` (heap) / — (stack, auto-reclaimed)      |                                                                                 |
| `AllocArray { dst, size, elem_size }` | `%p = alloca [N x T]` (stack) / `call @malloc` (heap)      |                                                                                 |

**Struct/Array Access Instructions**:

| IR Instruction                            | LLVM IR                                                | Notes                                |
| ----------------------------------------- | ------------------------------------------------------ | ------------------------------------ |
| `LoadField { dst, src, field }`           | `%ptr = getelementptr T, T* %src, 0, field` + `load`   |                                      |
| `StoreField { dst, field, src }`          | `%ptr = getelementptr T, T* %dst, 0, field` + `store`  |                                      |
| `LoadIndex { dst, src, index }`           | `%ptr = getelementptr T, T* %src, 0, %index` + `load`  |                                      |
| `StoreIndex { dst, index, src }`          | `%ptr = getelementptr T, T* %dst, 0, %index` + `store` |                                      |
| `CreateStruct { dst, type_name, fields }` | `insertvalue` chain                                    | Construct LLVM struct in field order |

**Type Conversion Instructions**:

| IR Instruction                   | LLVM IR                                                                                                     | Notes                                                           |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `Cast { dst, src, target_type }` | `bitcast` / `trunc` / `zext` / `sext` / `fptrunc` / `fpext` / `sitofp` / `fptosi` / `inttoptr` / `ptrtoint` | Choose appropriate cast based on source/target type combination |
| `TypeTest(val, type)`            | —                                                                                                           | Compile-time type test, generates `icmp eq` comparing type tags |

**Ownership and Borrow Instructions**:

| IR Instruction                 | LLVM IR                                              | Notes                                                                         |
| ------------------------------ | ---------------------------------------------------- | ----------------------------------------------------------------------------- |
| `Borrow { dst, src, mutable }` | —                                                    | **Zero-sized token, completely disappears after compilation**, produces no IR |
| `Release(val)`                 | —                                                    | **Zero-sized token, completely disappears after compilation**                 |
| `Move { dst, src }`            | —                                                    | Ownership transfer, becomes register copy after normalization                 |
| `Drop(val)`                    | `call void @T.drop(T* %val)`                         | Call type's destructor (see §7)                                               |
| `ShareRef { dst, src }`        | `call %T* @Arc_new(%src)` / `call %T* @Rc_new(%src)` | Compiler auto-selects Arc/Rc based on cross-thread usage                      |
| `ArcNew { dst, src }`          | `call %T* @Arc_new(%src)`                            | Atomic refcount = 1                                                           |
| `ArcClone { dst, src }`        | `call %T* @Arc_clone(%src)`                          | Atomically increment refcount                                                 |
| `ArcDrop(val)`                 | `call void @Arc_drop(%val)`                          | Atomically decrement + conditional release                                    |

**Concurrency Instructions**:

| IR Instruction                     | LLVM IR                           | Notes                                                                   |
| ---------------------------------- | --------------------------------- | ----------------------------------------------------------------------- |
| `Spawn { closures, plan, result }` | Expand to scheduler call sequence | See §5, runtime `task_spawn` + `task_wait_all`                          |
| `Yield`                            | —                                 | AOT path uses synchronous wait in spawn block, no yield needed; ignored |

**Unsafe Block and Raw Pointer Instructions**:

| IR Instruction            | LLVM IR                                                   | Notes                                   |
| ------------------------- | --------------------------------------------------------- | --------------------------------------- |
| `UnsafeBlockStart`        | —                                                         | **Compile-time marker, produces no IR** |
| `UnsafeBlockEnd`          | —                                                         | **Compile-time marker, produces no IR** |
| `PtrFromRef { dst, src }` | `%p = ptrtoint T* %src to i64` (or directly copy pointer) |                                         |
| `PtrDeref { dst, src }`   | `%v = load T, T* %src`                                    |                                         |
| `PtrStore { dst, src }`   | `store T %src, T* %dst`                                   |                                         |
| `PtrLoad { dst, src }`    | `%v = load T, T* %src`                                    |                                         |

**String Instructions**:

| IR Instruction                      | LLVM IR                                     | Notes                                       |
| ----------------------------------- | ------------------------------------------- | ------------------------------------------- |
| `StringLength { dst, src }`         | `%len = extractvalue { i8*, i64 } %src, 1`  | String is `{ ptr, len }`, length is field 1 |
| `StringConcat { dst, lhs, rhs }`    | `call String @yx_string_concat(%lhs, %rhs)` | Runtime helper function                     |
| `StringGetChar { dst, src, index }` | `getelementptr` + `load i32`                | Includes bounds checking                    |
| `StringFromInt { dst, src }`        | `call String @yx_string_from_int(%src)`     | Runtime helper function                     |
| `StringFromFloat { dst, src }`      | `call String @yx_string_from_f64(%src)`     | Runtime helper function                     |

**Closure Instructions**:

| IR Instruction                           | LLVM IR                                                                              | Notes                         |
| ---------------------------------------- | ------------------------------------------------------------------------------------ | ----------------------------- |
| `MakeClosure { dst, func: String, env }` | Allocate closure struct + fill function pointer (looked up by function name) and env | `{ fn_ptr, env_fields... }`   |
| `LoadUpvalue { dst, upvalue_idx }`       | `%v = extractvalue %env, upvalue_idx`                                                | Read upvalue from closure env |
| `StoreUpvalue { src, upvalue_idx }`      | `%env = insertvalue %env, %src, upvalue_idx`                                         | Write upvalue to closure env  |
| `CloseUpvalue(val)`                      | Copy stack upvalue to heap                                                           |                               |

**Other Instructions**:

| IR Instruction                  | LLVM IR                                       | Notes                       |
| ------------------------------- | --------------------------------------------- | --------------------------- |
| `HeapAlloc { dst, type_id }`    | `call i8* @malloc(i64 size)` + write type tag | Heap allocation + type info |
| `NewDict { dst, keys, values }` | `call Dict @yx_dict_new(%keys, %values)`      | Runtime helper function     |

> **Note**: `Push`/`Pop`/`Dup`/`Swap` have already been eliminated during the §4.0 normalization
> phase and do not appear in the translation table. `Borrow`/`Release` are zero-sized compile-time
> tokens and produce no machine code.

### 5. spawn Block Code Generation

Aligned with [RFC-024](../accepted/024-concurrency-model.md), the compilation of a spawn block
proceeds in the following steps.

#### 5.1 Semantics Review

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // Direct subexpression → task 1
    t2 = fetch("url2"),   // Direct subexpression → task 2
    return (t1, t2)       // Synchronous wait, assemble result
}
```

**Rules** (RFC-024 §2.1):

- **Direct subexpressions** (top-level comma-separated statements) of the spawn block create
  parallel tasks
- Expressions inside nested `{}` are not direct subexpressions and do not become independent tasks
- The entire spawn block synchronously blocks, waiting for all tasks to complete before returning

#### 5.2 Compilation Steps

```
Step 1: Identify direct subexpressions
  Walk the spawn block body, collect top-level statements

Step 2: Dependency analysis
  For each direct subexpression, analyze which variables produced by previous tasks it references
  No dependency → can be dispatched in parallel immediately
  Has dependency → queue to wait for the dependent task to complete

Step 3: Resource conflict detection (RFC-024 §2.5)
  Check whether the same resource type instance is used by multiple tasks
  Same-instance conflict → mark for serial execution order

Step 4: Generate task functions
  Each direct subexpression generates a separate LLVM function (closure)

Step 5: Generate dispatch code
  Call runtime scheduler's task_spawn / task_wait

Step 6: Result assembly
  Collect outputs from all tasks, assemble the return tuple
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

; Assemble return values
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

The compiler detects that `parse(data)` references `data` produced by task 1, and marks the
dependency when generating the dispatch code:

```llvm
; Task 2 is created with a dependency on task 1
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
;                                                              ↑
;                                                 Dependent on task 0 (fetch) completing
```

#### 5.5 Automatic Serialization on Resource Types

The resource types ([RFC-024 §2.5](../accepted/024-concurrency-model.md)) (`FilePath`, `HttpUrl`,
`DBUrl`, `Console`, and user-defined resource types) are automatically serialized in spawn blocks:

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // Uses SqliteDb (resource type)
    r2 = db.exec("INSERT ...")    // Same instance → automatically serialized
}
```

The compiler detects that the same resource instance is used by two tasks and generates a serial
dependency:

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

The compiler expands this into N independent tasks (N = length of items), limited by the maximum
concurrency.

### 6. FFI Code Generation

> ⚠️ **Dependency Note**: The **architecture** of the FFI code generation defined in this section
> (`native("x")` → `declare external @x` → marshalling wrapper function → call) is stable and does
> not change with RFC-026 syntax changes. The specific parameter marshalling rule table (§6.2) and
> opaque type layout (§6.3) reference the definitions in RFC-026 — if the `native()` syntax or
> marshalling rules in RFC-026 change, only the corresponding mapping tables in this document need
> to be updated; the architecture layer is unaffected. RFC-026 current status: **Under Review**,
> located alongside this document in the `review/` directory.
>
> **Acceptance Prerequisite**: Before this RFC is accepted, the portions of RFC-026 that relate to
> §6 of this document (the `native()` declaration syntax, parameter marshalling rules, opaque type
> `{ i8* }` layout, `.drop` binding convention) should be frozen first or accepted together with
> RFC-026. Otherwise the mapping tables in §6.2/§6.3/§7 may become outdated before implementation
> begins.

Aligned with [RFC-026](./026-ffi-core-mechanism.md), this section defines the LLVM IR generation
strategy for FFI calls.

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
- The compiler automatically generates the marshalling wrapper function
- The wrapper function's signature uses YaoXiang types, internally converting to C types

#### 6.2 Parameter Marshalling

| Direction                                  | Conversion                              |
| ------------------------------------------ | --------------------------------------- |
| YaoXiang `String` → C `char*`              | Extract `.ptr` field to pass            |
| YaoXiang `Int32` → C `int`                 | Direct pass (`i32`)                     |
| YaoXiang `*Void` → C `void*`               | Direct pass (`i8*`)                     |
| YaoXiang `T` (transparent) → C `struct T*` | Pass address                            |
| YaoXiang `T` (opaque) → C `struct T*`      | Extract pointer from `{ i8* }` and pass |

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

**Layout Optimization**: When an opaque type has only a single `handle: *Void` field, it can be
optimized to directly use `i8*` (omitting the outer struct). The optimized ABI is exactly identical
to a C pointer, with zero marshalling overhead. This optimization is enabled by default and is
transparent to the user.

#### 6.4 LLVM Representation of ?T Nullable Return Values

The FFI nullable return value defined in [RFC-026](./026-ffi-core-mechanism.md) §7.6:

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

General LLVM representation: `{ i1, { i8* } }` — has-value flag + data.

**Optimization for FFI null pointer**: If `T` in `?T` is an opaque type (internally a pointer), the
compiler uses the **null pointer = None** optimization:

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

This optimization makes FFI calls of `?SqliteDb` **zero extra overhead** — completely equivalent to
C's null check.

#### 6.5 yx-bindgen Integration

The auto-generated binding file from [yx-bindgen](./026-ffi-core-mechanism.md) §6 is treated as
regular YaoXiang source code at compile time. The compiler does not need to know the code comes from
bindgen — the handling of `native()` declarations and `unsafe {}` type definitions is completely
uniform.

### 7. Destructor Code Generation

Aligned with the RAII semantics of [RFC-009](../accepted/009-ownership-model.md) and the `.drop`
convention from [RFC-026](./026-ffi-core-mechanism.md) §7.

#### 7.1 .drop Binding Recognition

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

The compiler recognizes the `.drop` binding and marks the destructor function pointer in the type
metadata.

#### 7.2 Cleanup Insertion at Scope End

```
User code:
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT ...")
    stmt.step()
    // ← Scope end
}

Cleanup inserted by the compiler (in reverse order):
    call @sqlite3_finalize(%stmt)    // stmt.drop()
    call @sqlite3_close(%db)         // db.drop()
```

**Insertion Locations**:

- Normal scope end (`}`)
- Early return (before `return`)
- `?` error propagation path (before `?`)
- End of spawn block (destructors of task-local variables)

#### 7.3 Move and Destructors

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: ownership transfers to db2
// db is no longer valid; no drop inserted for db here
// ← Scope end: drop is only inserted for db2
```

The compiler tracks Move semantics ([RFC-009](../accepted/009-ownership-model.md) §1) and only
inserts destructor calls at the final holder of a variable.

#### 7.4 Destructor Failure Handling

```llvm
; Debug mode: check destructor return value
%ret = call i32 @sqlite3_close(i8* %handle)
%ok = icmp eq i32 %ret, 0
br i1 %ok, label %done, label %panic
panic:
  call @__yx_panic("destructor failed")
  unreachable
done:
  ret void

; Release mode: ignore return value
call i32 @sqlite3_close(i8* %handle)
ret void
```

### 8. Compilation Artifact Structure

The compilation artifact contains the following components (specific struct definitions will be
determined during the implementation phase):

- **Machine Code**: LLVM-compiled object file (`.o`), containing all function translation results
- **spawn Metadata**: Task function pointer for each spawn block, dependency relationships, resource
  conflict serialization pairs
- **FFI Symbol Table**: External C symbol references (symbol name + whether weakly referenced)
- **Entry Point Table**: List of entry functions for the executable
- **Type Information**: Reflection metadata, written to the `.reflect` section, mmap'd on demand by
  the runtime

### 9. Runtime Library

Aligned with [RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md), the runtime is linked as
a **static library** into the final exe.

```
Internal structure of the final exe:

┌────────────────────────────────────────────┐
│  User code (native machine code)              │
│  ├── Regular functions (sequential execution)  │
│  ├── spawn block expansion (task funcs + dispatch calls) │
│  ├── FFI marshalling wrapper functions         │
│  └── RAII destructor code                      │
├────────────────────────────────────────────┤
│  Runtime static library (about 500KB-1MB, depending on platform and feature selection)  │
│  ├── Thread pool (num_workers)                  │
│  ├── Event loop (libuv / io_uring)              │
│  ├── Work-stealing queue (Full Runtime only)    │
│  ├── Memory allocator (jemalloc / mimalloc)     │
│  └── Reflection metadata (.reflect section, on-demand mmap)    │
│                                              │
│  Not included:                                │
│  ❌ Bytecode interpreter                       │
│  ❌ JIT compiler                               │
│  ❌ GC                                         │
│  ❌ Virtual machine                            │
└────────────────────────────────────────────┘
```

**Key Design**: The compile-time phase completes task identification and dependency analysis for
spawn blocks; the runtime only does "create task → dispatch to thread pool → wait for completion" —
fixed data structures, predictable behavior.

> **Difference from the size estimate in RFC-008**: RFC-008 §4 estimates the scheduler at about
> 200-500KB, containing only the task scheduling core. The 500KB-1MB estimate in this document
> additionally includes the memory allocator (jemalloc/mimalloc), event loop (libuv/io_uring), and
> reflection metadata section. The actual size depends on the platform and feature selection;
> precise numbers will be given during the implementation phase.

**Three-Layer Runtime and LLVM Relationship** (aligned with RFC-008 §1):

| Runtime      | LLVM AOT Behavior                                                                                                |
| ------------ | ---------------------------------------------------------------------------------------------------------------- |
| **Embedded** | No spawn support, directly generates sequential machine code                                                     |
| **Standard** | Supports spawn blocks, DAG within spawn block + single-threaded scheduling (num_workers=1)                       |
| **Full**     | Supports spawn blocks, DAG within spawn block + multi-threaded scheduling (num_workers>1), supports WorkStealing |

---

## Detailed Design

### Module Directory Structure

Aligned with the directory layout in [RFC-008](../accepted/008-runtime-concurrency-model.md) §6.
`[! Planning]` markers indicate files/directories that have not yet been created and will be
introduced during the implementation phase of this RFC.

```
src/
├── frontend/                          # Compilation front-end (shared by all backends)
│   ├── core/
│   │   ├── spawn/                     # spawn module (concurrency analysis shared by VM and LLVM backends)
│   │   │   ├── mod.rs                 # spawn module entry
│   │   │   ├── placement.rs           # spawn occurrence legality check
│   │   │   └── analysis.rs            # [! Planning] Task identification, dependency analysis, resource conflict detection
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
│       │   ├── translator.rs          # IR → bytecode translation (used by VM backend)
│       │   ├── emitter.rs             # Bytecode emission + jump backpatching (used by VM backend)
│       │   ├── buffer.rs              # Constant pool + bytecode buffer (used by VM backend)
│       │   ├── bytecode.rs            # Bytecode format definition + serialization (used by VM backend)
│       │   ├── flow.rs                # Register allocation + label generation + symbol table (used by VM backend)
│       │   └── operand.rs             # Operand parsing (used by VM backend)
│       ├── lifetime/                  # Lifetime/token liveness analysis
│       └── mono/                      # Monomorphization
│
├── backends/
│   ├── common/                        # Shared values/heap/opcodes
│   ├── interpreter/                   # Tree-walking interpreter (VM backend)
│   ├── llvm/                          # [! Planning] LLVM backend code generation (see file list below)
│   │   ├── mod.rs                     # [! Planning] LLVM backend entry
│   │   ├── context.rs                 # [! Planning] LLVM context management
│   │   ├── types.rs                   # [! Planning] Type mapping (YaoXiang → LLVM IR)
│   │   ├── values.rs                  # [! Planning] Value mapping
│   │   ├── func.rs                    # [! Planning] Function translation
│   │   ├── spawn.rs                   # [! Planning] spawn block expansion
│   │   ├── ffi.rs                     # [! Planning] FFI call code generation
│   │   └── drop.rs                    # [! Planning] Destructor insertion
│   └── runtime/                       # Compiled runtime (static library linked into exe)
│       ├── engine.rs                  # Task scheduling engine
│       ├── facade.rs                  # External interface
│       └── task.rs                    # Task representation
│
└── util/
    └── diagnostic/                    # Error diagnostics (shared)
```

> **Key Change**: The spawn block analysis (task identification, dependency analysis, resource
> conflict detection) will be implemented in `frontend/core/spawn/` (shared by the front-end). The
> existing `frontend/core/typecheck/passes/spawn_placement.rs` (spawn occurrence check) will be
> migrated to `frontend/core/spawn/placement.rs`. See RFC-024 for details. The LLVM backend only
> consumes the analysis results and generates the corresponding dispatch code.
>
> **Current State**: The current `middle/passes/codegen/` files `buffer.rs`, `emitter.rs`,
> `bytecode.rs`, `flow.rs`, `operand.rs` serve the bytecode generation for the VM backend
> (`CodegenContext::generate()` → `BytecodeFile`). The LLVM backend will be implemented in
> `backends/llvm/`, at the same level as the interpreter backend and runtime — both share the same
> `ModuleIR` input, but output different target formats (bytecode vs native code).

### Platform ABI Support

| Platform       | Target Triple              | Output Format | Calling Convention (FFI default) |
| -------------- | -------------------------- | ------------- | -------------------------------- |
| Linux x86_64   | `x86_64-unknown-linux-gnu` | ELF           | System V AMD64                   |
| macOS x86_64   | `x86_64-apple-darwin`      | Mach-O        | System V AMD64                   |
| macOS ARM64    | `aarch64-apple-darwin`     | Mach-O        | ARM64 AAPCS                      |
| Windows x86_64 | `x86_64-pc-windows-msvc`   | COFF          | Microsoft x64                    |

FFI calls use the platform's C calling convention by default. Users can override this with options
like `native("symbol", cc = "stdcall")` (aligned with future extensions in
[RFC-026](./026-ffi-core-mechanism.md)).

### Floating-Point Semantic Consistency (VM ↔ LLVM)

The core promise of the dual-backend architecture is that the VM (development/debugging) and LLVM
(production release) behave consistently. Floating-point operations have potential inconsistencies
between the two execution modes:

| Scenario         | Risk                                                           | Strategy                                                                                     |
| ---------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| NaN propagation  | VM and LLVM may handle NaN sign bit and payload differently    | Compiler normalizes NaN representation at IR level; NaN comparisons uniformly use `fcmp uno` |
| Rounding mode    | LLVM defaults to round-to-nearest-even; VM depends on host CPU | Do not expose non-default rounding modes; VM and LLVM uniformly use RTNE                     |
| Division by zero | IEEE 754 defines ±Inf, but some platforms may trap             | Debug mode checks division by zero and reports diagnostic; release mode follows IEEE 754     |
| `-0.0` vs `+0.0` | Comparison operations may not be equivalent                    | Uniformly use IEEE 754 rules: `+0.0 == -0.0`                                                 |
| Denormal numbers | Some platforms flush-to-zero                                   | LLVM does not enable `denormal-fp-math` attribute, preserving full IEEE 754 semantics        |

> **Testing Strategy**: Implement a cross-backend floating-point consistency test suite — the same
> YaoXiang source code is executed on both VM and LLVM backends, with output compared value by
> value. This test set is a mandatory CI gate.

---

## Trade-offs

### Advantages

1. **Performance**: AOT compilation is 10-100x faster than interpretation
2. **Unified Front-End**: VM and LLVM share the same front-end, with completely consistent behavior
3. **Zero Scheduling Overhead**: Regular code directly generates sequential machine code, with no
   DAG overhead outside spawn blocks
4. **Static Linking**: No external runtime dependencies, a single exe is enough to deploy
5. **Zero GC**: RAII deterministic destructors, no pauses
6. **Zero FFI Overhead**: `?T` null pointer optimization, opaque type layout optimization — FFI call
   cost is equivalent to C
7. **Compile-Time Analysis**: spawn block task identification and dependency analysis are done at
   compile time; the runtime only executes

### Disadvantages

1. **LLVM Integration Complexity**: Requires deep understanding of inkwell API and LLVM IR
2. **Compilation Time**: AOT compilation is slower than the interpreter (one-time cost)
3. **Debugging Experience**: Native code debugging requires DWARF/PDB symbol support (compiler needs
   to generate debug info)
4. **Incremental Compilation**: Large projects require additional incremental compilation design
5. **Floating-Point Semantic Consistency**: VM and LLVM may differ in edge-case behaviors such as
   NaN propagation, rounding mode, and division by zero; normalization strategies are needed to
   ensure consistent behavior across both backends (see §10)

### Consistency with Related RFCs

| RFC                                   | Consistency                                                                |
| ------------------------------------- | -------------------------------------------------------------------------- |
| RFC-024 spawn block concurrency model | ✅ spawn block direct subexpressions → task dispatch                       |
| RFC-008 Runtime Architecture          | ✅ Dual backend + scheduler static library + module directory structure    |
| RFC-009 Ownership Model v9            | ✅ `&T`/`&mut T` tokens (zero-sized), `ref T` (fat pointer), `?T` (Option) |
| RFC-026 FFI Core Mechanism            | ✅ `native()` → declare + marshalling, `.drop` → RAII cleanup              |

---

## Alternatives

| Option                               | Description                 | Why Not Chosen                                                              |
| ------------------------------------ | --------------------------- | --------------------------------------------------------------------------- |
| Interpreter only                     | No AOT needed               | Insufficient performance                                                    |
| Pure static compilation (no runtime) | Do not link scheduler       | spawn blocks need runtime task scheduling                                   |
| Cranelift backend                    | Faster compilation speed    | Runtime performance not as good as LLVM; could be a future optional backend |
| Link external LLVM runtime           | Use LLVM's built-in runtime | Introduces unnecessary dependencies                                         |

---

## Implementation Strategy

### Phase Breakdown

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
- [ ] Implement Move semantics tracking (used for destructor insertion decision)

#### Phase 4: spawn Block Code Generation

- [ ] Consume analysis results from `spawn_placement.rs`
- [ ] Direct subexpression → task function generation
- [ ] Dependent task dispatch code generation
- [ ] Resource conflict serialization
- [ ] spawn for expansion

#### Phase 5: FFI Code Generation

- [ ] `native()` → `declare external` (`ffi.rs`)
- [ ] Argument marshalling / return value unmarshalling
- [ ] Opaque type layout (including single-field optimization)
- [ ] `?T` null pointer optimization (FFI-specific)

#### Phase 6: Destructor Code Generation

- [ ] `.drop` binding recognition
- [ ] Scope-end cleanup insertion (reverse order) (`drop.rs`)
- [ ] Early-return path cleanup
- [ ] `?` error propagation path cleanup

#### Phase 7: Runtime Library Linking

- [ ] Implement `runtime_task_spawn` / `runtime_task_wait_all` and other runtime functions
- [ ] Link runtime static library
- [ ] End-to-end integration testing

### Dependencies

- RFC-024 (spawn block concurrency) → input for Phase 4
- RFC-009 v9 (ownership) → input for Phases 3, 6
- RFC-008 (Runtime Architecture) → input for Phase 7
- RFC-026 (FFI Mechanism) → input for Phase 5

---

## Related Work

### Lazy Task Creation (1990)[^1]

| Attribute       | Description                                                          |
| --------------- | -------------------------------------------------------------------- |
| Institution     | MIT                                                                  |
| Authors         | James R. Larus, Robert H. Halstead Jr.                               |
| Core Idea       | Defer creation of child tasks, create on demand                      |
| Reference Value | Theoretical foundation for on-demand task scheduling in spawn blocks |

**Core Idea**: Rather than creating tasks immediately, defer task creation. A child task is only
created when the parent task needs its value. This addresses the performance overhead of
fine-grained parallel tasks[^1]. YaoXiang's spawn block scheduling borrows this idea — tasks are
identified at compile time, but dispatched on demand to the thread pool at runtime.

### Lazy Scheduling (2014)[^2]

| Attribute       | Description                                              |
| --------------- | -------------------------------------------------------- |
| Institution     | University of Maryland                                   |
| Authors         | Tzannes, Caragea                                         |
| Core Idea       | Runtime adaptive scheduling, no extra state              |
| Reference Value | Reference for Full Runtime WorkStealing scheduler design |

### SISAL Language[^3]

| Attribute       | Description                                                       |
| --------------- | ----------------------------------------------------------------- |
| Institution     | Lawrence Livermore National Laboratory (LLNL)                     |
| Core Idea       | Single-assignment language, dataflow graph, implicit parallelism  |
| Reference Value | Proof of feasibility of dataflow model in industrial applications |

**Key Difference**: SISAL's parallelism is **implicit** — the language has single-assignment
semantics, and the compiler automatically analyzes the whole program's data dependency graph to
decide parallelism. YaoXiang's parallelism is **explicit** — users mark parallel regions with
`spawn {}` blocks, and the compiler only analyzes dependencies within spawn blocks. This avoids the
complexity of whole-program analysis as in SISAL, while preserving the user's control over parallel
behavior.

### Mul-T Parallel Scheme[^4]

| Attribute       | Description                                         |
| --------------- | --------------------------------------------------- |
| Institution     | MIT                                                 |
| Core Idea       | Future construct, Lazy Task Creation implementation |
| Reference Value | Specific implementation reference                   |

### Comparison Summary

| Technique              | Lazy Creation | Parallelism Marker           | Analysis Scope         | Ownership                    |
| ---------------------- | ------------- | ---------------------------- | ---------------------- | ---------------------------- |
| Lazy Task Creation[^1] | ✅            | Implicit                     | Whole program          | N/A                          |
| Lazy Scheduling[^2]    | ✅            | Implicit                     | Whole program          | N/A                          |
| SISAL[^3]              | ✅            | Implicit (single-assignment) | Whole program          | N/A                          |
| Mul-T[^4]              | ✅            | Explicit (future)            | Call site              | N/A                          |
| **YaoXiang**           | ✅            | **Explicit (spawn block)**   | **Within spawn block** | **✅ (Move + tokens + ref)** |

**YaoXiang's Innovation**: Elevates the parallelism marker from "every function call" (future) to
"structured block" (spawn). Users write ordinary code and place a spawn block where parallelism is
needed. The analysis scope is constrained within the spawn block, making compilation efficient and
behavior controllable.

---

## Appendix

### Appendix A: Comparison with Rust async

| Feature             | Rust async                           | YaoXiang LLVM AOT                                                     |
| ------------------- | ------------------------------------ | --------------------------------------------------------------------- |
| Compilation product | State machine + machine code         | Machine code + spawn task metadata                                    |
| Runtime             | tokio                                | Statically linked scheduler (about 500KB-1MB)                         |
| Concurrency marker  | async/await keyword                  | `spawn { }` block                                                     |
| Task creation       | Compile-time generated state machine | Compile-time identification of direct subexpressions → task functions |
| Function coloring   | async contagion                      | **No function coloring**                                              |
| Synchronous wait    | `.await`                             | spawn block auto-synchronous block                                    |
| Memory management   | GC (runtime)                         | **RAII (deterministic)**                                              |
| Sharing mechanism   | `Arc::new()` + manual Weak           | **`ref` keyword (compiler auto-selects Rc/Arc)**                      |

### Appendix B: Design Decision Record

| Decision                    | Decision                                                                                    | Date       |
| --------------------------- | ------------------------------------------------------------------------------------------- | ---------- |
| Adopt LLVM AOT              | Direct Codegen, no over-abstraction                                                         | 2026-02-15 |
| Concurrency model alignment | Aligned with RFC-024 spawn block direct subexpression model                                 | 2026-06-10 |
| DAG analysis scope          | Within spawn block, not across spawn blocks (aligned with RFC-024)                          | 2026-06-05 |
| Ownership model alignment   | Aligned with RFC-009 v9: `&T`/`&mut T` tokens + `ref` keyword                               | 2026-06-10 |
| Dual backend model          | VM (development) + LLVM (production), aligned with RFC-008                                  | 2026-05-11 |
| Scheduler form              | Static library linked into exe, about 500KB-1MB (depending on platform and features), no GC | 2026-05-11 |
| FFI code generation         | Integration with RFC-026: `native()` declare + marshalling                                  | 2026-06-10 |
| Destructor                  | `.drop` → RAII cleanup insertion, aligned with RFC-026 §7                                   | 2026-06-10 |
| Side effect handling        | Remove `@IO`/`@Pure` inference, use RFC-024 resource types instead                          | 2026-06-10 |
| Reflection metadata         | Compiled into exe .reflect section, mmap'd on demand                                        | 2026-05-11 |
| Paper citations             | Retain Lazy Task Creation and others, clearly state YaoXiang's differences                  | 2026-02-16 |

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

## Lifecycle and Destination

| Status        | Location                    | Description                                |
| ------------- | --------------------------- | ------------------------------------------ |
| **Draft**     | `docs/design/rfc/`          | Author's draft, awaiting review submission |
| **Reviewing** | `docs/design/rfc/review/`   | Open community discussion and feedback     |
| **Accepted**  | `docs/design/rfc/accepted/` | Becomes a formal design document           |
| **Rejected**  | `docs/design/rfc/`          | Retained in the RFC directory              |

> Current status: **Accepted** — Aligned with the RFC-024 spawn block concurrency model, RFC-009 v9
> ownership model, and RFC-026 FFI mechanism
