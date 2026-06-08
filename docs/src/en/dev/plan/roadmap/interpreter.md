---
title: "Interpreter Status"
---

# Interpreter

> **Module Status**: Stable (5 items pending improvement)
> **Location**: `src/backends/interpreter/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The interpreter is responsible for executing bytecode. It uses a register-based virtual machine architecture and supports all 39 bytecode instructions, fully aligned with RFC-008 (Concurrency Model) and RFC-009 (Ownership Model).

**Code Size**: 3,768 lines (9 source files)

---

## Feature List

### Core Execution Engine (execute.rs - 1,308 lines)

**Control Flow (10 types)**:
- ✅ Nop, Return, ReturnValue, Yield (no-op), EvalPush, EvalPop, Spawn, Jmp, JmpIf, JmpIfNot, Switch (simplified)

**Register Operations (5 types)**:
- ✅ Mov, LoadConst, LoadLocal, StoreLocal, LoadArg

**Arithmetic/Logical Operations (11 BinaryOp + 1 UnaryOp)**:
- ✅ Add/Sub/Mul/Div/Rem/And/Or/Xor/Shl/Sar/Shr, both Int and Float supported
- ⚠️ UnaryOp only implements Int negation

**Comparisons (6 CompareOp types)**:
- ✅ Eq/Ne/Lt/Le/Gt/Ge, both Int and String supported

**Memory Operations (9 types)**:
- ✅ StackAlloc (no-op), HeapAlloc, Drop (no-op), GetField, SetField, LoadElement, StoreElement, NewListWithCap, CreateStruct

**Arc/Weak Operations (5 types)**:
- ✅ ArcNew, ArcClone, ArcDrop (no-op), WeakNew, WeakUpgrade

**Borrow Tokens (2 types)**:
- ✅ Borrow (ZST, runtime equivalent to Mov), Release (ZST, runtime equivalent to Nop)

**Function Calls (7 types)**:
- ✅ CallStatic, CallNative, CallVirt, CallDyn, MakeClosure, LoadUpvalue, StoreUpvalue, CloseUpvalue

**String Operations (6 types)**:
- ✅ StringLength, StringConcat, StringEqual, StringGetChar, StringFromInt, StringFromFloat

**Exception Handling (3 types)**:
- ✅ TryBegin (no-op), TryEnd (no-op), Throw

**Debug/Type (4 types)**:
- ⚠️ BoundsCheck (no-op), TypeCheck (no-op), Cast (passthrough), TypeOf (placeholder)

### Core Architecture Capabilities

- ✅ **Heap**: Dynamic allocation of List/Tuple/Array/Dict/Struct
- ✅ **Call Stack (Frame)**: Register file + local variables + upvalue + eval stack + spawn group
- ✅ **Constant Pool**: Cross-module shared
- ✅ **Function Table**: By name (HashMap) and by index (Vec) dual tables, supports closure calls by ID
- ✅ **FFI Registry**: Preloads `std.io.*` series functions, extensible for custom native functions
- ✅ **DAG Task Scheduling (LocalRuntime)**: Lazy/concurrent evaluation based on RFC-008
- ✅ **Three Evaluation Strategies**: Block (synchronous), Auto (lazy/concurrent), Eager (eager)
- ✅ **Structured Concurrency**: Spawn group tracking, wait for all tasks on scope exit, dependency failure cascading cancellation

---

## Test Coverage

**Approximately 60 tests**:

| Test Type | Count | Coverage Scope |
|-----------|-------|----------------|
| Unit tests (within module) | ~35 | registers, ffi, frames, tests, debug, execute |
| Integration tests | 25 | Full compilation pipeline: hello world, variable declaration, arithmetic, comparison, lambda, function definition, if/elif/else, while, for, match, List/Tuple/Dict, list comprehension, closure higher-order functions, module import, f-string |

---

## RFC Comparison

### RFC-008 (Runtime Concurrency Model)

| Design Requirement | Implementation Status | Notes |
|--------------------|-----------------------|-------|
| Three-layer runtime: Embedded / Standard / Full | ✅ Implemented | Configured via `RuntimeMode` |
| Three evaluation strategies: Block / Auto / Eager | ✅ Implemented | |
| DAG task scheduling (`LocalRuntime`) | ✅ Implemented | |
| Task dependency tracking, cancellation propagation, structured concurrency | ✅ Implemented | |
| Sync = special case of scheduling (Embedded mode) | ✅ Implemented | |

### RFC-009 (Ownership Model)

| Design Requirement | Implementation Status | Notes |
|--------------------|-----------------------|-------|
| Borrow/Release as zero-size tokens (ZST) | ✅ Implemented | Runtime equivalent to Mov/Nop |
| ArcNew/ArcClone/ArcDrop implement `ref` keyword semantics | ✅ Implemented | |
| WeakNew/WeakUpgrade implement weak references | ✅ Implemented | |
| Move semantics (default behavior) | ✅ Implemented | |
| `clone()` handled at compile layer | ✅ Implemented | No special instructions needed at runtime |

---

## Simplified/Placeholder Implementations

| Instruction | Current Behavior | Design Intent |
|-------------|------------------|---------------|
| Switch | Directly advance IP | Should dispatch jumps by value |
| TypeOf | Returns type_table length as placeholder | Should return runtime type information |
| Cast | Passes through value (no actual conversion) | Should convert to target type |
| BoundsCheck / TypeCheck | No-op | Debug mode should perform runtime checks |
| StringGetChar | Takes first character only, ignores index argument | Should take character by index |
| UnaryOp | Int negation only, ignores op type | Should support more unary operations |
| step/step_over/step_out/run | `todo!()` | Debugger stepping functionality not implemented |

---

## Code Quality Assessment

| Dimension | Rating | Notes |
|-----------|--------|-------|
| Outstanding items | 5 | Switch instruction, debugger stepping, instruction completion, debug checks, test coverage |
| Test coverage | Good | Approximately 60 tests, covering main functional paths |
| Documentation quality | Good | Every source file has module-level doc comments, referencing RFC numbers |
| Code architecture | Excellent | Clear layering: executor/frames/registers/ffi/runtime |
| RFC compliance | Fully aligned | RFC-008 and RFC-009 design fully aligned |

---

## Pending Improvements

1. **Implement real dispatch for Switch instruction**
2. **Implement debugger stepping functionality** (step/step_over/step_out/run)
3. **Complete StringGetChar/UnaryOp and other instructions**
4. **Implement BoundsCheck/TypeCheck debug mode checks**
5. **Add boundary conditions and error path tests**