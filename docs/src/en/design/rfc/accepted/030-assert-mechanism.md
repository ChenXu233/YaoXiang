---
title: 'RFC-030: assert Mechanism'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-15'
updated: '2026-07-14'
decision:
  'assert and Assert are two sides of the same coin, automatically dispatched by dispatch. All 6
  Phases implemented (#157-#162 closed). Unified registration in std.assert module (#169 closed),
  assert native function + Assert/IsTrue type family on the same path.'
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

# RFC-030: assert Mechanism

## Summary

Introduce the `assert` mechanism for YaoXiang, used for testing, precondition checks, and runtime
panic. `assert` and the compile-time refined type `Assert(C)` (see RFC-011 §4.3) are **two sides of
the same refinement primitive**—automatically dispatched by dispatch to either compile-time proof or
runtime check based on "whether the predicate's free variables are available at compile time."
`assert(false, "msg")` is equivalent to `raise`, and no separate `throw`/`raise` keyword is needed.

## Motivation

### Why is this feature needed?

Currently, YaoXiang's E2E tests can only simulate assertions through `if` + `io.println` + `return`:

```yaoxiang
val = some_func()
if val != 42 {
    io.println("FAIL: expected 42")
    return
}
```

This approach has three problems:

1. **Boilerplate-heavy**: Each assertion requires 4 lines, bloating test files
2. **Weak error messages**: Manual string concatenation, missing source location
3. **Not composable**: Cannot register assertions in bulk, cannot pass as arguments to test
   frameworks

### Current problems

- No unified assertion mechanism
- Test code is flooded with `if` + print + `return` patterns
- The `Throw` instruction already exists at the bytecode level but is not exposed at the language
  level
- RFC-011 defines the compile-time `Assert(C)` conditional type, but runtime `assert()` is not yet
  implemented

### Design principles

`assert` is YaoXiang's only user-facing panic mechanism. `assert(false, "msg")` is equivalent to
`raise`, and no separate `throw`/`raise` keyword is needed. The `assert` function itself is the best
encapsulation of `if raise`.

**No new keywords, no new syntax. Everything is a function call.**

## Approach A: native function

Implement `assert` as a native function, introducing no new keywords.

```yaoxiang
use std.assert.assert

main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### Overload signatures

`assert` has two overloads:

```
// Core signature: assert is the value-universe introducer of Assert
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))
//                                       ^^^^^^^^^^^^^^^^^^^^^^^^
//                                       Returns a refined type, not ()
//
// IsTrue: Bool -> Type is the bridge from truth value to type:
//   IsTrue(true)  = Void   (⊤, program continues)
//   IsTrue(false) = Never  (⊥, divergent/compile error)
```

The actual behavior of `assert` is determined by dispatch:

- All free variables known at compile time → **CompileTime**: compiler evaluates cond, true → erased
  to Void, false → compile error (Never uninhabitable)
- Runtime free variables exist → **Runtime**: insert check, inject refined facts into the
  flow-sensitive assumption set Γ

The optional message `?msg` and the Result overload (see below) are preserved as runtime raise
payloads.

#### Overload 1: Conditional assertion `(Bool, ?String | Error)`

`Bool` + optional message. The message can be a `String` or an `Error` value:

```yaoxiang
assert(1 + 1 == 2)                    // No message, default panic info
assert(1 + 1 == 2, "math is broken")   // String message
assert(x > 0, my_error)                // Throw an Error value directly
```

`assert(false, "msg")` is YaoXiang's `raise`/`throw` equivalent—no separate keyword needed.

#### Overload 2: Result assertion `(Result)`

A single `Result` parameter, automatically checking for `Err`:

### Advantages

- **Zero syntax change**: Pure function, no new keywords needed
- **Zero new concepts**: Reuses existing native function registration mechanism
- **High extensibility**: Function overloading naturally supports multiple signatures
- **Self-documenting**: The `std.assert` namespace is documentation in itself

### Disadvantages

- None. When assert's type signature is correct, the compiler can infer dead code through function
  reachability analysis. No additional pass needed.

### Runtime behavior

1. Evaluate the first argument `condition: Bool`
2. If `true`, return `Unit`
3. If `false`, trigger a runtime panic:
   - Output the `message` content (if any)
   - Output the call stack (in debug mode)
   - Terminate the current execution

#### Failure behavior for each overload

| Signature                  | Behavior on failure              |
| -------------------------- | -------------------------------- |
| `assert(false)`            | Default panic message            |
| `assert(false, "msg")`     | Output string message then panic |
| `assert(false, error_val)` | Throw the Error value            |
| `assert(Err(x))`           | Extract Err contents and panic   |

### Relationship with compile-time Assert

`assert` and `Assert` are **two sides of the same refinement primitive**—automatically selected by
the dispatch pipeline based on "whether the predicate's free variables are available at compile
time":

| Condition                                | Dispatch                     | Behavior                                                               |
| ---------------------------------------- | ---------------------------- | ---------------------------------------------------------------------- |
| All free variables known at compile time | CompileTime → proof pipeline | Proved → erased, Disproved → compile error, Unknown → proof required   |
| Runtime free variables exist             | Runtime → insert check       | Bool check + inject refined facts into flow-sensitive assumption set Γ |

```yaoxiang
use std.assert

# Known at compile time (generic parameter) — goes to CompileTime, zero runtime overhead
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N is a generic parameter, evaluated at compile time
}

# Runtime value — goes to Runtime, inserts Bool check
x = read_int()
assert.assert(x > 0, "expected positive")  # Runtime check
```

> **2026-07-12 Unified plan**: The previous "completely independent" conclusion has been superseded.
> `assert()` is the value introducer of `Assert`, automatically dispatched by dispatch.

### Compiler changes

**No changes to parser, AST, typecheck, or IR gen are needed.**

Only native function registration needs to be added under `src/std/`:

1. Add `src/std/assert.rs`
2. Register `std.assert.assert` and `std.assert.Assert` (the latter is the compile-time conditional
   type)
3. Internally call the existing `BytecodeInstr::Throw` instruction

### Advantages

- **Zero syntax change**: Pure function, no new keywords needed
- **Zero new concepts**: Reuses existing native function registration mechanism
- **High extensibility**: Function signatures can be extended to variants like `assert_eq` (future)
- **Self-documenting**: The `std.assert` namespace is documentation in itself

### Disadvantages

- ~~Not known at compile time: unlike Approach B (keyword), cannot perform dead code elimination at
  compile time~~ → **No longer holds under the unified plan**. CompileTime-mode assert goes through
  the proof pipeline; compile-time-known cond → erased or compile error (`assert(false)` → Never →
  dead code).
- Call stack is only available in debug mode

## Approach B: Built-in keyword (superseded by the unified plan)

> Deprecated. The opposition between Approaches A and B is dissolved by the dispatch pipeline—assert
> is the value introducer of Assert; compile-time-known predicates go through the proof pipeline
> (zero runtime overhead), runtime predicates go through check. There is no need to choose between
> "function" and "keyword." The following is historical record.

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### Type signature

No independent type signature—the keyword is handled by the parser.

### Runtime behavior

Same as Approach A.

### Compiler changes

Parser, AST, typecheck, and IR gen need to be modified:

1. Parser: add new `Expr::Assert` variant
2. AST: add new `Expr::Assert` node
3. Typecheck: validate argument types
4. IR gen: generate `BytecodeInstr::Throw`

### Advantages

- Source location is known at compile time (no dependency on debug info)
- Compile-time constant folding is possible: `assert(true)` → no-op, `assert(false)` → compile error

### Disadvantages

| Disadvantage                          | Impact                                                 |
| ------------------------------------- | ------------------------------------------------------ |
| Requires parser changes               | Introduces new syntax node, increases maintenance cost |
| Keywords are not extensible           | Variants like `assert_eq` still need functions         |
| Compile-time advantage is impractical | See analysis below                                     |

### Comparison

| Dimension           | Approach A (function) | Approach B (keyword)              |
| ------------------- | --------------------- | --------------------------------- |
| Implementation cost | ~20 lines             | parser + AST + typecheck + IR gen |
| Syntax change       | None                  | New keyword                       |
| Extensibility       | Function overloading  | Requires companion macros         |
| Source location     | debug info            | Compile-time available            |
| Constant folding    | Requires pass support | Compile-time available            |
| Runtime overhead    | Function call         | Minimal                           |

### Realistic constraints of compile-time analysis

The core advantage of Approach B—compile-time analysis—requires a **constant folding pass** to take
effect. That is, the compiler needs to evaluate `false` in `assert(false)` at compile time to know
it's dead code.

YaoXiang currently has no constant folding pass. Even with Approach B, common patterns like
`assert(x > 0)` still cannot be analyzed at compile time. Only literals like `assert(true)` /
`assert(false)` can be analyzed.

Therefore, the compile-time advantage of Approach B is **theoretical at the current stage, not
practical**.

---

## Open questions

- [x] ~~Choose Approach A or Approach B?~~ → **Unified plan: assert is the value introducer of
      Assert**. The opposition between A and B is dissolved by the dispatch
      pipeline—compile-time-known predicates go through the proof pipeline, runtime predicates go
      through check. No need to "pick one."
- [x] ~~Does `assert` need to support a simplified form `assert(cond)` without `message`?~~ → **Yes.
      `assert(cond, ?msg)`, message is optional.**
- [x] ~~Are variants like `assert_eq`, `assert_ne` needed?~~ → **No. YAGNI. Wait until the test
      framework takes shape.**
- [x] ~~Does panic output include source location?~~ → Approach A depends on debug info (call
      stack).
- [x] ~~assert / Assert unification issue~~ → **Determined**. Unified plan:
      `assert: (Bool) -> Assert(IsTrue(cond))`, two sides of the same coin, automatically dispatched
      by dispatch. The `Never` type (⊥) is built in as the return type of `assert(false)`.

### 2026-07-05: Choose Approach A (superseded by the unified plan)

The 20-line implementation of Approach A wins in value and cost. After the unified plan was
determined on 2026-07-12, the opposition between A and B is dissolved by the dispatch
pipeline—assert is the value introducer of Assert, no longer a choice between "function" and
"keyword."

### 2026-07-12: Unified plan determined (supersedes the 2026-07-11 "completely independent" conclusion)

**Conclusion**: `assert` and `Assert` are not two independent mechanisms.
`assert: (Bool) -> Assert(IsTrue(cond))`—automatically dispatched by dispatch:

- Compile-time known → enters proof pipeline (Proved erased / Disproved error / Unknown needs proof)
- Runtime input → inserts check + injects Γ assumption

**Module structure**: `std.assert` uniformly hosts runtime assertions (`assert`) and compile-time
refined types (`Assert`, `IsTrue`). No longer "implemented separately," but two sides of the same
primitive.

### 2026-07-11: assert overload design

**Question**: Why does `assert` need two overloads, instead of a unified `(Bool, ?String)`?

**Answer**:

Runtime `assert()` is YaoXiang's only user-facing panic mechanism. `assert(false, "msg")` is
equivalent to `raise`/`throw` in other languages. So it needs to cover three scenarios:

1. Condition + simple message: `assert(cond, "msg")`
2. Condition + custom Error: `assert(cond, my_error)`
3. Result check: `assert(result)` — the most concise `if is_err { panic }`

The justification for the Result overload is: this is the shortest path for error
propagation—"Result should be Ok, or else die." No need to call `.is_ok()` first and then handle the
error separately.

## Appendix B: Design decision record

| Decision                                         | Determination                                                                                                  | Date       | Recorder |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------- | ---------- | -------- |
| Choose Approach A or Approach B                  | **Unified plan**: dispatch pipeline dissolves A/B opposition, assert is the value introducer of Assert         | 2026-07-12 | Chenxu   |
| Whether message is optional                      | **Yes**: `assert(cond, ?msg)`, String or Error                                                                 | 2026-07-11 | Chenxu   |
| Whether variants like assert_eq are needed       | **No**. YAGNI, wait for the test framework                                                                     | 2026-07-11 | Chenxu   |
| Whether a separate raise/throw keyword is needed | **No**. `assert(false, msg)` is equivalent to raise                                                            | 2026-07-11 | Chenxu   |
| Relationship between assert and Assert           | **Two sides of the same coin**. `assert: (Bool) -> Assert(IsTrue(cond))`, automatically dispatched by dispatch | 2026-07-12 | Chenxu   |

## References

- [RFC-007: Unified Function Definition Syntax](007-function-syntax-unification.md) —
  `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — type system foundation
- [RFC-011: Generic Type System Design §4.3](../accepted/011-generic-type-system.md) — compile-time
  verification and `Assert(C)` conditional type
- [RFC-026: FFI Core Mechanism](026-ffi-core-mechanism.md) — native function registration mechanism
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
  — compile-time evaluation system
