---
title: 'RFC-030: assert Assertion Mechanism'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-15'
updated: '2026-07-14'
decision:
  'assert and Assert are two sides of the same coin, automatically dispatched by dispatch. All 6
  Phases are implemented (#157–#162 are closed). The std.assert module is uniformly registered (#169
  is closed), with the assert native function and Assert/IsTrue type family on the same path.'
issue: '#97'
issues_impl:
  - '#155'
  - '#157'
  - '#158'
  - '#159'
  - '#160'
  - '#161'
  - '#162'
  - '#169'
---

# RFC-030: assert Assertion Mechanism

## Summary

Introduce the `assert` assertion mechanism for YaoXiang, used for testing, precondition checks, and
runtime panic. `assert` and the compile-time refinement type `Assert(C)` (see RFC-011 §4.3) are
**two sides of the same refinement primitive**—dispatched automatically by the dispatch pipeline
based on "whether the predicate's free variables are reachable at compile time" to either a
compile-time proof or a runtime check. `assert(false, "msg")` is equivalent to `raise`; no separate
`throw`/`raise` keyword is needed.

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

1. **Lots of boilerplate**: each assertion needs 4 lines, bloating test files
2. **Weak error messages**: manual string concatenation, no source location
3. **Not composable**: cannot batch-register assertions, cannot pass them as arguments to a test
   framework

### The current problem

- No unified assertion mechanism
- Test code is filled with the `if` + print + `return` pattern
- The `Throw` instruction already exists at the bytecode level, but is not exposed at the language
  level
- RFC-011 defines the compile-time `Assert(C)` conditional type, but the runtime `assert()` is not
  yet implemented

### Design Principles

`assert` is YaoXiang's only user-level panic mechanism. `assert(false, "msg")` is equivalent to
`raise`; no separate `throw`/`raise` keyword is needed. The `assert` function itself is the best
wrapper around `if raise`.

**No new keyword, no new syntax. Everything is a function call.**

## Option A: native function

Implement `assert` as a native function; no new keyword is introduced.

```yaoxiang
use std.assert.assert

main: () -> Void = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### Overloaded Signatures

`assert` has two overloads:

```
// Core signature: assert is a value-universe introducer of Assert
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))
//                                       ^^^^^^^^^^^^^^^^^^^^^^^^
//                                       Returns a refinement type, not ()
//
// IsTrue: Bool -> Type is the bridge from truth value to type:
//   IsTrue(true)  = Void   (⊤, program continues)
//   IsTrue(false) = Never  (⊥, diverges/compile error)
```

The actual behavior of `assert` is determined by dispatch:

- All free variables known at compile time → **CompileTime**: the compiler evaluates `cond`; `true`
  → erased to `Void`; `false` → compile error (Never is uninhabitable)
- Runtime free variables exist → **Runtime**: insert a check, inject the refinement fact into the
  flow-sensitive assumption set Γ

The optional message `?msg` and the Result overload (see below) are retained as the runtime raise
payload.

#### Overload 1: Condition assertion `(Bool, ?String | Error)`

`Bool` + optional message. The message can be a `String` or an `Error` value:

```yaoxiang
assert(1 + 1 == 2)                    // No message, default panic info
assert(1 + 1 == 2, "math is broken")   // String message
assert(x > 0, my_error)                // Throw the Error value directly
```

`assert(false, "msg")` is YaoXiang's `raise`/`throw` equivalent—no separate keyword is needed.

#### Overload 2: Result assertion `(Result)`

A single `Result` argument; automatically checks whether it is `Err`:

### Advantages

- **Zero syntax change**: pure function, no new keyword needed
- **Zero new concepts**: reuses the existing native function registration mechanism
- **High extensibility**: function overloading naturally supports multiple signatures
- **Self-documenting**: the `std.assert` namespace itself is documentation

### Disadvantages

- None. When `assert`'s type signature is correct, the compiler can infer dead code via function
  reachability analysis. No additional pass is needed.

### Runtime Behavior

1. Evaluate the first argument `condition: Bool`
2. If `true`, return `Unit`
3. If `false`, trigger a runtime panic:
   - Output the `message` content (if any)
   - Output the call stack (in debug mode)
   - Terminate the current execution

#### Failure Behavior of Each Overload

| Signature                  | Behavior on Failure               |
| -------------------------- | --------------------------------- |
| `assert(false)`            | Default panic info                |
| `assert(false, "msg")`     | Output string message then panic  |
| `assert(false, error_val)` | Throw the Error value             |
| `assert(Err(x))`           | Extract the Err content and panic |

### Relationship with Compile-Time Assert

`assert` and `Assert` are **two sides of the same refinement primitive**—the dispatch pipeline
automatically chooses based on "whether the predicate's free variables are reachable at compile
time":

| Condition                                | Dispatch                     | Behavior                                                                      |
| ---------------------------------------- | ---------------------------- | ----------------------------------------------------------------------------- |
| All free variables known at compile time | CompileTime → proof pipeline | Proved → erased; Disproved → compile error; Unknown → proof required          |
| Runtime free variables exist             | Runtime → insert check       | Bool check + inject refinement facts into the flow-sensitive assumption set Γ |

```yaoxiang
use std.assert

# Known at compile time (generic parameter) — goes CompileTime, zero runtime cost
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N is a generic parameter, evaluated at compile time
}

# Runtime value — goes Runtime, inserts a Bool check
x = read_int()
assert.assert(x > 0, "expected positive")  # Runtime check
```

> **2026-07-12 Unified Solution**: the previous "completely independent" conclusion has been
> replaced. `assert()` is the value introducer of `Assert`, automatically dispatched.

### Compiler Changes

**No changes to parser, AST, typecheck, or IR gen are required.**

Only need to add native function registration under `src/std/`:

1. Add `src/std/assert.rs`
2. Register `std.assert.assert` and `std.assert.Assert` (the latter is the compile-time conditional
   type)
3. Internally call the existing `BytecodeInstr::Throw` instruction

### Advantages

- **Zero syntax change**: pure function, no new keyword needed
- **Zero new concepts**: reuses the existing native function registration mechanism
- **High extensibility**: function signatures can be extended to variants like `assert_eq` (in the
  future)
- **Self-documenting**: the `std.assert` namespace itself is documentation

### Disadvantages

- ~~Not known at compile time: unlike Option B (keyword), cannot perform dead-code elimination at
  compile time~~ → **No longer holds under the unified solution**. Under CompileTime mode, `assert`
  goes through the proof pipeline; a `cond` known at compile time → erased or compile error
  (`assert(false)` → Never → dead code).
- Call stack can only be obtained in debug mode

## Option B: Built-in keyword (replaced by the unified solution)

> Deprecated. The opposition between Options A and B is dissolved by the dispatch pipeline—`assert`
> is the value introducer of `Assert`; a compile-time-known condition goes through the proof
> pipeline (zero runtime cost), and a runtime condition goes through a check. No need to choose
> between "function" and "keyword". The following is historical record.

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### Type Signature

No independent type signature—the keyword is handled by the parser.

### Runtime Behavior

Same as Option A.

### Compiler Changes

Need to modify parser, AST, typecheck, and IR gen:

1. parser: add the `Expr::Assert` variant
2. AST: add the `Expr::Assert` node
3. typecheck: validate argument types
4. IR gen: generate `BytecodeInstr::Throw`

### Advantages

- Source location known at compile time (not dependent on debug info)
- Constant folding possible at compile time: `assert(true)` → no-op, `assert(false)` → compile error

### Disadvantages

| Disadvantage                          | Impact                                                 |
| ------------------------------------- | ------------------------------------------------------ |
| Requires parser change                | Introduces new syntax node, increases maintenance cost |
| Keyword not extensible                | Variants like `assert_eq` still need functions         |
| Compile-time advantage is impractical | See analysis below                                     |

### Comparison

| Dimension           | Option A (function)   | Option B (keyword)                |
| ------------------- | --------------------- | --------------------------------- |
| Implementation cost | ~20 lines             | parser + AST + typecheck + IR gen |
| Syntax change       | None                  | New keyword                       |
| Extensibility       | Function overloading  | Requires companion macros         |
| Source location     | debug info            | Available at compile time         |
| Constant folding    | Requires pass support | Available at compile time         |
| Runtime overhead    | Function call         | Negligible                        |

### Realistic Constraints of Compile-Time Analysis

Option B's core advantage—compile-time analysis—requires a **constant folding pass** to take effect.
That is, the compiler needs to evaluate `false` in `assert(false)` at compile time to know it is
dead code.

YaoXiang currently has no constant folding pass. Even with Option B, common usages like
`assert(x > 0)` still cannot be analyzed at compile time. Only literals like `assert(true)` /
`assert(false)` can be analyzed.

Therefore, Option B's compile-time advantage **is theoretical, not actual, at the current stage**.

---

## Open Questions

- [x] ~~Choose Option A or Option B?~~ → **Unified solution: `assert` is the value introducer of
      `Assert`**. The A/B opposition is dissolved by the dispatch pipeline—compile-time-known goes
      through the proof pipeline, runtime goes through a check. No need to "pick one".
- [x] ~~Does `assert` need to support a simplified form `assert(cond)` without `message`?~~ → **Yes.
      `assert(cond, ?msg)`, with `message` optional.**
- [x] ~~Are variants like `assert_eq`, `assert_ne` needed?~~ → **No. YAGNI. Wait until the test
      framework takes shape.**
- [x] ~~Does the panic output include the source location?~~ → Option A depends on debug info (call
      stack).
- [x] ~~Unification of assert / Assert~~ → **Determined**. Unified solution:
      `assert: (Bool) -> Assert(IsTrue(cond))`, two sides of the same coin, automatically dispatched
      by dispatch. The `Never` type (⊥) is built into the return type of `assert(false)`.

### 2026-07-05: Chose Option A (replaced by the unified solution)

Option A's 20-line implementation wins on value and cost. After the 2026-07-12 unified solution was
determined, the A/B opposition is dissolved by the dispatch pipeline—`assert` is the value
introducer of `Assert`, no longer a choice between "function" and "keyword".

### 2026-07-12: Unified Solution Determined (replacing the 2026-07-11 "completely independent" conclusion)

**Conclusion**: `assert` and `Assert` are not two independent mechanisms.
`assert: (Bool) -> Assert(IsTrue(cond))`—automatically dispatched:

- Compile-time known → enters proof pipeline (Proved erased / Disproved error / Unknown requires
  proof)
- Runtime input → inserts a check + injects Γ assumption

**Module structure**: `std.assert` uniformly hosts runtime assertions (`assert`) and compile-time
refinement types (`Assert`, `IsTrue`). No longer "implemented separately", but two sides of the same
primitive.

### 2026-07-11: assert Overload Design

**Question**: Why does `assert` need two overloads instead of a unified `(Bool, ?String)`?

**Answer**:

Runtime `assert()` is YaoXiang's only user-level panic mechanism. `assert(false, "msg")` is
equivalent to `raise`/`throw` in other languages. Therefore, it needs to cover three scenarios:

1. Condition + simple message: `assert(cond, "msg")`
2. Condition + custom Error: `assert(cond, my_error)`
3. Result check: `assert(result)` — the most concise `if is_err { panic }`

The rationale for the Result overload is that this is the shortest path for error propagation—"the
Result should be Ok, otherwise die". No need to `.is_ok()` first and then handle the error
separately.

## Appendix B: Design Decision Record

| Decision                                      | Decision                                                                                                       | Date       | Recorder |
| --------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | ---------- | -------- |
| Choose Option A or Option B                   | **Unified solution**: dispatch pipeline dissolves A/B opposition; `assert` is the value introducer of `Assert` | 2026-07-12 | Chenxu   |
| Is the message optional?                      | **Yes**: `assert(cond, ?msg)`, String or Error                                                                 | 2026-07-11 | Chenxu   |
| Are variants like `assert_eq` needed?         | **No**. YAGNI; wait until the test framework is ready                                                          | 2026-07-11 | Chenxu   |
| Is a separate `raise`/`throw` keyword needed? | **No**. `assert(false, msg)` is equivalent to `raise`                                                          | 2026-07-11 | Chenxu   |
| Relationship between assert and Assert        | **Two sides of the same coin**. `assert: (Bool) -> Assert(IsTrue(cond))`, automatically dispatched by dispatch | 2026-07-12 | Chenxu   |

## References

- [RFC-007: Unified Function Definition Syntax](007-function-syntax-unification.md) — the
  `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — type system foundations
- [RFC-011: Generic Type System Design §4.3](011-generic-type-system.md) — compile-time verification
  and the `Assert(C)` conditional type
- [RFC-026: FFI Core Mechanism](026-ffi-core-mechanism.md) — native function registration mechanism
- [RFC-027: Compile-Time Predicates and Unified Static Verification](027-compile-time-evaluation-types.md)
  — compile-time evaluation system
