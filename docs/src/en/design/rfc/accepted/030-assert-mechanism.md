---
title: 'RFC-030: assert Mechanism'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-15'
updated: '2026-07-14'
decision:
  'assert and Assert are two sides of the same coin, with dispatch auto-routing. All 6 Phases
  implemented (#157-#162 closed). std.assert module unified registration (#169 closed), assert
  native function + Assert/IsTrue type family on the same path.'
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
panics. `assert` and the compile-time refinement type `Assert(C)` (see RFC-011 §4.3) are **two sides
of the same refinement primitive**—dispatch automatically routes to compile-time proof or runtime
check based on "whether the predicate's free variables are accessible at compile time".
`assert(false, "msg")` is equivalent to `raise`, no separate `throw`/`raise` keyword needed.

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

1. **Verbose boilerplate**: Each assertion requires 4 lines, causing test files to bloat
2. **Weak error messages**: Manual string concatenation, lacking source location
3. **Non-composable**: Cannot batch register assertions, cannot pass as parameters to test
   frameworks

### Current Problems

- No unified assertion mechanism
- Test code flooded with `if` + print + `return` patterns
- `Throw` instruction already exists at bytecode layer but not exposed at language level
- RFC-011 defines compile-time `Assert(C)` conditional type, but runtime `assert()` not yet
  implemented

### Design Principles

`assert` is YaoXiang's sole user-space panic mechanism. `assert(false, "msg")` is equivalent to
`raise`, no separate `throw`/`raise` keyword needed. The `assert` function itself is the best
encapsulation of `if raise`.

**No new keywords, no new syntax. Everything is a function call.**

## Option A: Native Function

Implement `assert` as a native function, without introducing new keywords.

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
// Core signature: assert is the value-universe introduction term for Assert
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))
//                                       ^^^^^^^^^^^^^^^^^^^^^^^^
//                                       Returns refinement type, not ()
//
// IsTrue: Bool -> Type is the bridge from truth values to types:
//   IsTrue(true)  = Void   (⊤, program continues)
//   IsTrue(false) = Never  (⊥, diverges/compile error)
```

The actual behavior of `assert` is determined by dispatch:

- All free variables known at compile time → **CompileTime**: Compiler evaluates cond, true → erased
  to Void, false → compile error (Never is uninhabitable)
- Runtime free variables exist → **Runtime**: Insert check, inject refinement facts into
  flow-sensitive assumption set Γ

Optional message `?msg` and Result overload (see below) are preserved as runtime raise payload.

#### Overload 1: Conditional Assertion `(Bool, ?String | Error)`

`Bool` + optional message. Message can be `String` or `Error` value:

```yaoxiang
assert(1 + 1 == 2)                    // No message, default panic info
assert(1 + 1 == 2, "math is broken")   // String message
assert(x > 0, my_error)                // Directly throw Error value
```

`assert(false, "msg")` is YaoXiang's `raise`/`throw` equivalent—no separate keyword needed.

#### Overload 2: Result Assertion `(Result)`

Single `Result` parameter, automatically checks for `Err`:

### Advantages

- **Zero syntax changes**: Pure function, no new keywords needed
- **Zero new concepts**: Reuses existing native function registration mechanism
- **High extensibility**: Function overloading naturally supports multiple signatures
- **Self-documenting**: `std.assert` namespace is documentation itself

### Disadvantages

- None. When assert's type signature is correct, the compiler can infer dead code through function
  reachability analysis. No additional pass needed.

### Runtime Behavior

1. Evaluate first argument `condition: Bool`
2. If `true`, return `Unit`
3. If `false`, trigger runtime panic:
   - Output `message` content (if present)
   - Output call stack (in debug mode)
   - Terminate current execution

#### Failure Behavior per Overload

| Signature                  | Failure Behavior             |
| -------------------------- | ---------------------------- |
| `assert(false)`            | Default panic message        |
| `assert(false, "msg")`     | Output string message, panic |
| `assert(false, error_val)` | Throw Error value            |
| `assert(Err(x))`           | Extract Err content, panic   |

### Relationship with Compile-Time Assert

`assert` and `Assert` are **two sides of the same refinement primitive**—dispatch routing pipeline
automatically selects based on "whether predicate free variables are accessible at compile time":

| Condition                                | Dispatch                     | Behavior                                                                  |
| ---------------------------------------- | ---------------------------- | ------------------------------------------------------------------------- |
| All free variables known at compile time | CompileTime → proof pipeline | Proved → erase, Disproved → compile error, Unknown → requires proof       |
| Runtime free variables exist             | Runtime → insert check       | Bool check + inject refinement facts into flow-sensitive assumption set Γ |

```yaoxiang
use std.assert

# Compile-time known (generic parameter) — CompileTime path, zero runtime overhead
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N is generic parameter, evaluated at compile time
}

# Runtime value — Runtime path, insert Bool check
x = read_int()
assert.assert(x > 0, "expected positive")  # runtime check
```

> **2026-07-12 Unification**: The previous "completely independent" conclusion has been superseded.
> `assert()` is the value introduction term for `Assert`, dispatch auto-routes.

### Compiler Changes

**No changes to parser, AST, typecheck, or IR gen needed.**

Only need to add native function registration under `src/std/`:

1. Add new `src/std/assert.rs`
2. Register `std.assert.assert` and `std.assert.Assert` (the latter is compile-time conditional
   type, see #155)
3. Internally call existing `BytecodeInstr::Throw` instruction

### Advantages

- **Zero syntax changes**: Pure function, no new keywords needed
- **Zero new concepts**: Reuses existing native function registration mechanism
- **High extensibility**: Function signatures extensible to `assert_eq` and other variants (future)
- **Self-documenting**: `std.assert` namespace is documentation itself

### Disadvantages

- ~~Compile-time unknowable: Unlike Option B (keyword), cannot do dead code elimination at compile
  time~~ → **No longer applicable under unified approach**. CompileTime mode assert goes through
  proof pipeline, compile-time-known cond → erase or compile error (`assert(false)` → Never → dead
  code).
- Call stack only available in debug mode

## Option B: Built-in Keyword (Superseded by Unified Approach)

> Deprecated. The opposition between Option A and B is resolved by dispatch routing pipeline—assert
> is the value introduction term for Assert, compile-time-known goes through proof pipeline (zero
> runtime overhead), runtime goes through check. No need to choose between "function" and "keyword".

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### Type Signature

No independent type signature—keywords handled by parser.

### Runtime Behavior

Same as Option A.

### Compiler Changes

Parser, AST, typecheck, IR gen changes needed:

1. parser: Add new `Expr::Assert` variant
2. AST: Add new `Expr::Assert` node
3. typecheck: Validate parameter types
4. IR gen: Generate `BytecodeInstr::Throw`

### Advantages

- Source location available at compile time (not dependent on debug info)
- Compile-time constant folding: `assert(true)` → no-op, `assert(false)` → compile error

### Disadvantages

| Disadvantage                         | Impact                                                |
| ------------------------------------ | ----------------------------------------------------- |
| Parser changes needed                | Introduce new syntax nodes, increase maintenance cost |
| Keywords not extensible              | `assert_eq` and other variants still need functions   |
| Compile-time advantage not practical | See analysis below                                    |

### Comparison

| Dimension           | Option A (Function)  | Option B (Keyword)                |
| ------------------- | -------------------- | --------------------------------- |
| Implementation cost | ~20 lines            | parser + AST + typecheck + IR gen |
| Syntax changes      | None                 | New keyword                       |
| Extensibility       | Function overloading | Requires配套 macros               |
| Source location     | debug info           | Available at compile time         |
| Constant folding    | Needs pass support   | Available at compile time         |
| Runtime overhead    | Function call        | Minimal                           |

### Practical Constraints of Compile-Time Analysis

Option B's core advantage—compile-time analysis—requires a **constant folding pass** to take effect.
That is, the compiler needs to evaluate `false` in `assert(false)` at compile time to know this is
dead code.

YaoXiang currently has no constant folding pass. Even with Option B, common patterns like
`assert(x > 0)` cannot be analyzed at compile time. Only literals like `assert(true)` /
`assert(false)` can be analyzed.

Therefore, Option B's compile-time advantage **is theoretical at this stage, not practical**.

---

## Open Issues

- [x] ~~Choose Option A or Option B?~~ → **Unified approach: assert is the value introduction term
      for Assert**. The opposition between Option A and B is resolved by dispatch routing
      pipeline—compile-time-known goes through proof pipeline, runtime goes through check. No need
      to "choose one".
- [x] ~~Does `assert` need to support simplified form without `message` `assert(cond)`?~~ →
      **Supported. `assert(cond, ?msg)`, message optional.**
- [x] ~~Are `assert_eq`, `assert_ne` and other variants needed?~~ → **No. YAGNI. Wait until test
      framework matures.**
- [x] ~~Should panic output include source location?~~ → Option A relies on debug info (call stack).
- [x] ~~assert / Assert unification~~ → **Determined**. Unified approach:
      `assert: (Bool) -> Assert(IsTrue(cond))`, two sides of same coin, dispatch auto-routes. See
      [#156](https://github.com/ChenXu233/YaoXiang/issues/156) (closed). `Never` type (⊥) as the
      built-in return type of `assert(false)`.

### 2026-07-05: Choose Option A (Superseded by Unified Approach)

Option A's 20-line implementation wins on value and cost. After 2026-07-12 unified approach
determination, the Option A/B opposition is resolved by dispatch routing pipeline—assert is the
value introduction term for Assert, no longer choosing between "function" and "keyword".

### 2026-07-12: Unified Approach Determined (Supersedes 2026-07-11's "Completely Independent" Conclusion)

**Conclusion**: `assert` and `Assert` are not two independent mechanisms.
`assert: (Bool) -> Assert(IsTrue(cond))` — dispatch auto-routes:

- Compile-time known → into proof pipeline (Proved erase / Disproved error / Unknown requires proof)
- Runtime input → insert check + inject Γ assumptions

**Module structure**: `std.assert` unified carries runtime assertions (`assert`) and compile-time
refinement types (`Assert`, `IsTrue`). No longer "implement separately", but two sides of the same
primitive.

### 2026-07-11: assert Overload Design

**Question**: Why does `assert` need two overloads instead of a unified `(Bool, ?String)`?

**Answer**:

Runtime `assert()` is YaoXiang's sole user-space panic mechanism. `assert(false, "msg")` is
equivalent to other languages' `raise`/`throw`. Therefore it needs to cover three scenarios:

1. Condition + simple message: `assert(cond, "msg")`
2. Condition + custom Error: `assert(cond, my_error)`
3. Result check: `assert(result)` — most concise `if is_err { panic }`

The reasonableness of the Result overload: This is the shortest path for error propagation—"Result
should be Ok, otherwise die". No need to first `.is_ok()` then handle error separately.

## Appendix B: Design Decision Record

| Decision                                | Conclusion                                                                                                                | Date       | Recorder |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ---------- | -------- |
| Choose Option A or Option B             | **Unified approach**: dispatch routing pipeline resolves A/B opposition, assert is the value introduction term for Assert | 2026-07-12 | Chen Xu  |
| Is message optional                     | **Yes**: `assert(cond, ?msg)`, String or Error                                                                            | 2026-07-11 | Chen Xu  |
| Are assert_eq and other variants needed | **No**. YAGNI, wait until test framework matures                                                                          | 2026-07-11 | Chen Xu  |
| Is separate raise/throw keyword needed  | **No**. `assert(false, msg)` is equivalent to raise                                                                       | 2026-07-11 | Chen Xu  |
| Relationship between assert and Assert  | **Two sides of same coin**. `assert: (Bool) -> Assert(IsTrue(cond))`, dispatch auto-routes                                | 2026-07-12 | Chen Xu  |

## References

- [RFC-007: Function Definition Syntax Unification](007-function-syntax-unification.md) —
  `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — Type system foundation
- [RFC-011: Generic System Design §4.3](../accepted/011-generic-type-system.md) — Compile-time
  verification and `Assert(C)` conditional type
- [RFC-026: FFI Core Mechanism](026-ffi-core-mechanism.md) — Native function registration mechanism
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
  — Compile-time evaluation system
