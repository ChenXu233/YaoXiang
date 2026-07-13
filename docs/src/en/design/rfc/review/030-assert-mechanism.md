---
title: "RFC-030: assert Mechanism"
status: "Under Review"
author: "ChenXu"
created: "2026-06-15"
updated: "2026-07-14"
decision: "assert and Assert are two sides of the same coin; dispatch automatically routes. All 6 Phases implemented (#157-#162 closed)."
issue: "#97"
issues_impl:
  - "#155"
  - "#157"
  - "#158"
  - "#159"
  - "#160"
  - "#161"
  - "#162"
---

# RFC-030: assert Mechanism

## Summary

Introduce the `assert` mechanism to YaoXiang, used for testing, precondition checks, and runtime panics. `assert` and the compile-time refinement type `Assert(C)` (see RFC-011 §4.3) are **two sides of the same refinement primitive**—dispatched by the "whether the predicate's free variables are available at compile time" criterion into either a compile-time proof or a runtime check. `assert(false, "msg")` is equivalent to `raise`; no separate `throw`/`raise` keyword is needed.

## Motivation

### Why is this feature needed?

Currently, YaoXiang's E2E tests can only simulate assertions via `if` + `io.println` + `return`:

```yaoxiang
val = some_func()
if val != 42 {
    io.println("FAIL: expected 42")
    return
}
```

This approach has three problems:

1. **Boilerplate-heavy**: Every assertion requires 4 lines, causing test files to bloat.
2. **Weak error messages**: Manual string concatenation, lacking source location.
3. **Not composable**: Cannot batch-register assertions, cannot pass as arguments to test frameworks.

### Current Problems

- No unified assertion mechanism
- Test code is full of the `if` + print + `return` pattern
- The bytecode layer already has a `Throw` instruction, but it is not exposed at the language level
- RFC-011 defines the compile-time `Assert(C)` conditional type, but the runtime `assert()` is not yet implemented

### Design Principles

`assert` is YaoXiang's only user-space panic mechanism. `assert(false, "msg")` is equivalent to `raise`; no separate `throw`/`raise` keyword is needed. The `assert` function itself is the best wrapper for `if raise`.

**No new keywords, no new syntax. Everything is a function call.**

## Option A: native function

Implement `assert` as a native function, without introducing a new keyword.

```yaoxiang
use std.assert.assert

main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### Overloaded Signatures

`assert` has two overloads:

```
// Core signature: assert is the value-universe introducer for Assert
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))
//                                       ^^^^^^^^^^^^^^^^^^^^^^^^
//                                       Returns a refinement type, not ()
//
// IsTrue: Bool -> Type bridges truth values to types:
//   IsTrue(true)  = Void   (⊤, program continues)
//   IsTrue(false) = Never  (⊥, divergent / compile error)
```

The actual behavior of `assert` is determined by dispatch:
- All free variables are known at compile time → **CompileTime**: the compiler evaluates `cond`; `true` → erased to Void, `false` → compile error (Never is uninhabitable)
- Runtime free variables exist → **Runtime**: insert a check, inject refinement facts into the flow-sensitive assumption set Γ

The optional message `?msg` and the Result overload (see below) are retained as runtime raise payloads.

#### Overload 1: Condition Assertion `(Bool, ?String | Error)`

`Bool` + optional message. The message can be a `String` or an `Error` value:

```yaoxiang
assert(1 + 1 == 2)                    // No message; default panic info
assert(1 + 1 == 2, "math is broken")   // String message
assert(x > 0, my_error)                // Throw an Error value directly
```

`assert(false, "msg")` is YaoXiang's `raise`/`throw` equivalent—no separate keyword is needed.

#### Overload 2: Result Assertion `(Result)`

A single `Result` parameter; automatically checks whether it is `Err`:

### Pros

- **Zero syntax change**: Pure function, no new keyword required
- **Zero new concepts**: Reuses the existing native function registration mechanism
- **Highly extensible**: Function overloading naturally supports multiple signatures
- **Self-documenting**: The `std.assert` namespace itself serves as documentation

### Cons

- None. When the type signature of `assert` is correct, the compiler can infer dead code through function reachability analysis. No extra pass is needed.

### Runtime Behavior

1. Evaluate the first argument `condition: Bool`
2. If `true`, return `Unit`
3. If `false`, trigger a runtime panic:
   - Output the contents of `message` (if any)
   - Output the call stack (in debug mode)
   - Terminate the current execution

#### Failure Behavior of Each Overload

| Signature | Behavior on Failure |
|------|-----------|
| `assert(false)` | Default panic info |
| `assert(false, "msg")` | Output the string message, then panic |
| `assert(false, error_val)` | Throw the Error value |
| `assert(Err(x))` | Extract the Err content and panic |

### Relationship with Compile-Time Assert

`assert` and `Assert` are **two sides of the same refinement primitive**—automatically selected by the dispatch pipeline based on "whether the predicate's free variables are available at compile time":

| Condition | Dispatch | Behavior |
|------|------|------|
| All free variables known at compile time | CompileTime → proof pipeline | Proved → erase; Disproved → compile error; Unknown → proof required |
| Runtime free variables exist | Runtime → insert check | Bool check + inject refinement facts into the flow-sensitive assumption set Γ |

```yaoxiang
use std.assert

# Known at compile time (generic parameter) — goes through CompileTime, zero runtime overhead
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N is a generic parameter, evaluated at compile time
}

# Runtime value — goes through Runtime, inserts a Bool check
x = read_int()
assert.assert(x > 0, "expected positive")  # Runtime check
```

> **2026-07-12 Unified Solution**: The previous "fully independent" conclusion has been superseded. `assert()` is the value introducer of `Assert`; dispatch handles routing automatically.

### Compiler Changes

**No changes needed to parser, AST, typecheck, or IR gen.**

Only native function registration needs to be added under `src/std/`:

1. Add a new `src/std/assert.rs`
2. Register `std.assert.assert` and `std.assert.Assert` (the latter is the compile-time conditional type, see #155)
3. Internally call the existing `BytecodeInstr::Throw` instruction

### Pros

- **Zero syntax change**: Pure function, no new keyword required
- **Zero new concepts**: Reuses the existing native function registration mechanism
- **Highly extensible**: Function signatures can be extended to variants like `assert_eq` (future)
- **Self-documenting**: The `std.assert` namespace itself serves as documentation

### Cons

- ~~Not compile-time aware: unlike Option B (keyword), it cannot perform compile-time dead code elimination~~ → **No longer holds under the unified solution**. CompileTime-mode `assert` goes through the proof pipeline; a compile-time-known `cond` → erase or compile error (`assert(false)` → Never → dead code).
- Source location is only available in debug mode

## Option B: Built-in Keyword (Superseded by the Unified Solution)

> Deprecated. The opposition between Options A and B is dissolved by the dispatch pipeline—`assert` is the value introducer of `Assert`; compile-time-known cases go through the proof pipeline (zero runtime overhead), runtime cases go through check. No need to choose between "function" and "keyword". The following is historical record.

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### Type Signature

No independent type signature—the keyword is handled by the parser.

### Runtime Behavior

Same as Option A.

### Compiler Changes

Parser, AST, typecheck, and IR gen all need changes:
1. Parser: Add `Expr::Assert` variant
2. AST: Add `Expr::Assert` node
3. Typecheck: Validate argument types
4. IR gen: Generate `BytecodeInstr::Throw`

### Pros

- Source location known at compile time (no dependency on debug info)
- Compile-time constant folding: `assert(true)` → no-op, `assert(false)` → compile error

### Cons

| Con | Impact |
|------|------|
| Parser changes required | Introduces new syntax node, increasing maintenance cost |
| Keyword not extensible | Variants like `assert_eq` still require functions |
| Compile-time advantages not practical | See analysis below |

### Comparison

| Dimension | Option A (Function) | Option B (Keyword) |
|------|---------------|-----------------|
| Implementation cost | ~20 lines | parser + AST + typecheck + IR gen |
| Syntax change | None | New keyword |
| Extensibility | Function overloading | Requires companion macros |
| Source location | debug info | Compile-time available |
| Constant folding | Requires pass support | Compile-time available |
| Runtime overhead | Function call | Minimal |

### Realistic Constraints of Compile-Time Analysis

Option B's core advantage—compile-time analysis—requires a **constant folding pass** to take effect. That is, the compiler must evaluate `false` in `assert(false)` at compile time to recognize it as dead code.

YaoXiang currently has no constant folding pass. Even with Option B, common cases like `assert(x > 0)` still cannot be analyzed at compile time. Only literals like `assert(true)` / `assert(false)` can be analyzed.

Therefore, Option B's compile-time advantage **is theoretical at the current stage, not practical**.

---

## Open Questions

- [x] ~~Choose Option A or Option B?~~ → **Unified solution: `assert` is the value introducer of `Assert`**. The A/B opposition is dissolved by the dispatch pipeline—compile-time-known cases go through the proof pipeline, runtime cases go through check. No need to "pick one".
- [x] ~~Does `assert` need to support a simplified form `assert(cond)` without a `message`?~~ → **Supported. `assert(cond, ?msg)`, message is optional.**
- [x] ~~Are variants like `assert_eq`, `assert_ne` needed?~~ → **No. YAGNI. Wait until the test framework matures.**
- [x] ~~Does the panic output include source location?~~ → Option A depends on debug info (call stack).
- [x] ~~assert / Assert unification issue~~ → **Resolved**. Unified solution: `assert: (Bool) -> Assert(IsTrue(cond))`, two sides of the same coin, dispatch handles routing automatically. See [#156](https://github.com/ChenXu233/YaoXiang/issues/156) (closed) for details. The `Never` type (⊥) is built in as the return type of `assert(false)`.

### 2026-07-05: Choose Option A (Superseded by the Unified Solution)

Option A's 20-line implementation wins on value-vs-cost. After the unified solution was determined on 2026-07-12, the A/B opposition is dissolved by the dispatch pipeline—`assert` is the value introducer of `Assert`; no longer choosing between "function" and "keyword".

### 2026-07-12: Unified Solution Determined (Supersedes the 2026-07-11 "Fully Independent" Conclusion)

**Conclusion**: `assert` and `Assert` are not two independent mechanisms. `assert: (Bool) -> Assert(IsTrue(cond))` — dispatched automatically:

- Compile-time known → enters the proof pipeline (Proved → erase / Disproved → error / Unknown → proof required)
- Runtime input → inserts check + injects Γ assumptions

**Module structure**: `std.assert` uniformly hosts the runtime assertion (`assert`) and the compile-time refinement types (`Assert`, `IsTrue`). No longer "implemented separately"—they are two sides of the same primitive.

### 2026-07-11: assert Overload Design

**Question**: Why does `assert` need two overloads rather than a unified `(Bool, ?String)`?

**Answer**:

The runtime `assert()` is YaoXiang's only user-space panic mechanism. `assert(false, "msg")` is equivalent to `raise`/`throw` in other languages. Therefore it needs to cover three scenarios:
1. Condition + simple message: `assert(cond, "msg")`
2. Condition + custom Error: `assert(cond, my_error)`
3. Result check: `assert(result)` — the most concise `if is_err { panic }`

The reasonability of the Result overload lies in: this is the shortest path for error propagation—"a Result should be Ok, otherwise die". No need to call `.is_ok()` and then handle the error separately.

## Appendix B: Design Decision Record

| Decision | Resolution | Date | Recorder |
|------|------|------|--------|
| Choose Option A or Option B | **Unified solution**: dispatch pipeline dissolves the A/B opposition; `assert` is the value introducer of `Assert` | 2026-07-12 | ChenXu |
| Is the message optional | **Yes**: `assert(cond, ?msg)`, String or Error | 2026-07-11 | ChenXu |
| Are variants like `assert_eq` needed | **No**. YAGNI; revisit when test framework matures | 2026-07-11 | ChenXu |
| Is a separate raise/throw keyword needed | **No**. `assert(false, msg)` is equivalent to raise | 2026-07-11 | ChenXu |
| Relationship between assert and Assert | **Two sides of the same coin**. `assert: (Bool) -> Assert(IsTrue(cond))`, dispatch handles routing automatically | 2026-07-12 | ChenXu |

## References

- [RFC-007: Unified Function Definition Syntax](007-function-syntax-unification.md) — `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — Foundations of the type system
- [RFC-011: Generic Type System Design §4.3](../accepted/011-generic-type-system.md) — Compile-time verification and the `Assert(C)` conditional type
- [RFC-026: FFI Core Mechanism](../review/026-ffi-core-mechanism.md) — Native function registration mechanism
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md) — Compile-time evaluation system