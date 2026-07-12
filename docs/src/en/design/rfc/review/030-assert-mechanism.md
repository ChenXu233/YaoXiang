---
title: "RFC-030: assert Assertion Mechanism"
status: "Under Review"
author: "Chen Xu"
created: "2026-06-15"
updated: "2026-07-11"
decision: "Assert and assert — compile-time assertion and runtime assertion."
issue: "#97"
issues_impl:
  - "#155"
---

# RFC-030: assert Assertion Mechanism

## Summary

Introduce the `assert` assertion mechanism into YaoXiang for testing and precondition checks. `assert` is the **only** user-facing runtime panic mechanism in YaoXiang — no separate `raise`/`throw` keyword is required. The compile-time conditional type `Assert(C)` is a separate feature (see RFC-011 §4.3) located under the same `std.assert` module, but its semantics and implementation are completely independent.

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

1. **Excessive boilerplate**: Each assertion requires 4 lines, bloating test files
2. **Weak error messages**: Manual string concatenation, lacking source location
3. **Not composable**: Cannot register assertions in bulk, cannot pass as arguments to test frameworks

### Current Problems

- No unified assertion mechanism
- Test code is filled with `if` + print + `return` patterns
- The bytecode layer already has a `Throw` instruction but it is not exposed at the language level
- RFC-011 defines the compile-time `Assert(C)` conditional type, but the runtime `assert()` has not yet been implemented

### Design Principles

`assert` is the **only** user-state panic mechanism in YaoXiang. `assert(false, "msg")` is equivalent to `raise` — no separate `throw`/`raise` keyword is required. The `assert` function itself is the best encapsulation of `if raise`.

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
// Overload 1: Conditional assertion
assert(cond: Bool, ?msg: String | Error)

// Overload 2: Result assertion
assert(cond: Result)
```

#### Overload 1: Conditional Assertion `(Bool, ?String | Error)`

`Bool` + optional message. The message can be a `String` or an `Error` value:

```yaoxiang
assert(1 + 1 == 2)                    // No message, default panic info
assert(1 + 1 == 2, "math is broken")   // String message
assert(x > 0, my_error)                // Throw an Error value directly
```

`assert(false, "msg")` is YaoXiang's `raise`/`throw` equivalent — no separate keyword needed.

#### Overload 2: Result Assertion `(Result)`

A single `Result` parameter that automatically checks whether it is `Err`:

### Advantages

- **Zero syntax changes**: Pure function, no new keywords
- **Zero new concepts**: Reuses the existing native function registration mechanism
- **Highly extensible**: Function overloading naturally supports multiple signatures
- **Self-documenting**: The `std.assert` namespace itself serves as documentation

### Disadvantages

- None. When the type signature of `assert` is correct, the compiler can infer dead code through function reachability analysis. No additional pass needed.

### Runtime Behavior

1. Evaluate the first argument `condition: Bool`
2. If `true`, return `Unit`
3. If `false`, trigger a runtime panic:
   - Output the contents of `message` (if any)
   - Output the call stack (in debug mode)
   - Terminate the current execution

#### Failure Behavior for Each Overload

| Signature | Behavior on Failure |
|-----------|---------------------|
| `assert(false)` | Default panic info |
| `assert(false, "msg")` | Output the string message then panic |
| `assert(false, error_val)` | Throw the Error value |
| `assert(Err(x))` | Extract the Err content and panic |

### Relationship with the Compile-time Assert

The `std.assert` module contains two completely independent features:

| Feature | Type | Name | Timing | Failure Behavior | Runtime Overhead |
|---------|------|------|--------|------------------|------------------|
| Runtime assertion | Function | `assert()` | Runtime | panic + call stack | Always present |
| Compile-time conditional type | Type | `Assert(C)` | Compile-time | Compilation error | Zero |

```yaoxiang
use std.assert

// Runtime assert: for testing, checks runtime values
assert.assert(result == 42, "expected 42")

// Compile-time Assert: refines types, validates generic parameters
length: assert.Assert(N > 0)
```

They are placed in the same module because they are different manifestations of the "assertion" concept, but the implementations are completely independent. `assert()` does not require `Assert(C)`, and vice versa.

### Compiler Changes

**No changes to parser, AST, typecheck, or IR gen are required.**

Only need to add native function registration under `src/std/`:

1. Add `src/std/assert.rs`
2. Register `std.assert.assert` and `std.assert.Assert` (the latter is the compile-time conditional type, see #155)
3. Internally call the existing `BytecodeInstr::Throw` instruction

### Advantages

- **Zero syntax changes**: Pure function, no new keywords
- **Zero new concepts**: Reuses the existing native function registration mechanism
- **Highly extensible**: Function signatures can be extended to variants like `assert_eq` (future)
- **Self-documenting**: The `std.assert` namespace itself serves as documentation

### Disadvantages

- Not known at compile time: Unlike Option B (keyword), it cannot perform dead code elimination at compile time
- Call stack is only available in debug mode

## Option B: Built-in Keyword

> Deprecated, kept for historical record only.

Implemented with an `assert` keyword, source location and constant folding are available at compile time.

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### Type Signature

No independent type signature — the keyword is handled by the parser.

### Runtime Behavior

Same as Option A.

### Compiler Changes

Requires changes to parser, AST, typecheck, and IR gen:
1. parser: Add `Expr::Assert` variant
2. AST: Add `Expr::Assert` node
3. typecheck: Validate argument types
4. IR gen: Generate `BytecodeInstr::Throw`

### Advantages

- Source location known at compile time (not dependent on debug info)
- Compile-time constant folding: `assert(true)` → no-op, `assert(false)` → compilation error

### Disadvantages

| Disadvantage | Impact |
|--------------|--------|
| Parser changes required | Introduces a new syntax node, increases maintenance cost |
| Keyword is not extensible | Variants like `assert_eq` still require functions |
| Compile-time advantages are impractical | See analysis below |

### Comparison

| Dimension | Option A (Function) | Option B (Keyword) |
|-----------|---------------------|---------------------|
| Implementation cost | ~20 lines | parser + AST + typecheck + IR gen |
| Syntax changes | None | New keyword |
| Extensibility | Function overloading | Requires supporting macros |
| Source location | debug info | Available at compile time |
| Constant folding | Requires pass support | Available at compile time |
| Runtime overhead | Function call | Minimal |

### Practical Constraints of Compile-time Analysis

The core advantage of Option B — compile-time analysis — requires a **constant folding pass** to take effect. That is, the compiler must evaluate `false` in `assert(false)` at compile time to know that this is dead code.

YaoXiang currently has no constant folding pass. Even with Option B, common forms like `assert(x > 0)` still cannot be analyzed at compile time. Only literals like `assert(true)` / `assert(false)` can be analyzed.

Therefore, the compile-time advantages of Option B are **theoretical rather than practical at the current stage**.

---

## Open Questions

- [x] ~~Choose Option A or Option B?~~ → **Option A: native function**
- [x] ~~Does `assert` need to support a simplified form `assert(cond)` without `message`?~~ → **Yes. `assert(cond, ?msg)`, message is optional.**
- [x] ~~Are variants like `assert_eq`, `assert_ne` needed?~~ → **No. YAGNI. Wait until the test framework is established.**
- [x] ~~Does the panic output include source location?~~ → Option A relies on debug info (call stack).
- [ ] ~~Unification of `assert` / `Assert`~~ → **Split out as a separate discussion #156**. The ideal state is `assert: (cond: Bool) -> Assert(cond)` to unify the two, but `Assert(C)` as a refinement type requires `C` to be known at compile time, which conflicts with runtime assertion. See [#156](https://github.com/ChenXu233/YaoXiang/issues/156) for details.
---

## Appendix A: Design Discussion Records

### 2026-07-05: Choosing Option A

Option A's 20-line implementation wins on value and cost. The compile-time advantages of Option B are theoretical at the present stage with no constant folding pass.

### 2026-07-11: assert Overload Design

**Question**: Why does `assert` need two overloads, instead of a unified `(Bool, ?String)`?

**Answer**:

The runtime `assert()` is the **only** user-state panic mechanism in YaoXiang. `assert(false, "msg")` is equivalent to `raise`/`throw` in other languages. Therefore it must cover three scenarios:
1. Condition + simple message: `assert(cond, "msg")`
2. Condition + custom Error: `assert(cond, my_error)`
3. Result check: `assert(result)` — the most concise `if is_err { panic }`

The reasonability of the Result overload lies in: this is the shortest path for error propagation — "Result should be Ok, otherwise die." No need to first `.is_ok()` and then handle the error separately.

### 2026-07-11: std.assert Module Structure

`std.assert` contains two features:

- `assert()`: Runtime assertion function (this RFC)
- `Assert(C)`: Compile-time conditional type (RFC-011 §4.3, #155)

They are placed in the same module because they are different manifestations of the "assertion" concept. As a namespace, `std.assert` is self-documenting — when developers see `use std.assert`, they know this imports assertion-related functionality.

## Appendix B: Design Decision Records

| Decision | Resolution | Date | Recorder |
|----------|------------|------|----------|
| Choose Option A or Option B | **Option A: native function** — 20-line implementation, first-class citizen, zero syntax changes. The compile-time advantages of Option B are theoretical at the current stage | 2026-07-03 | Chen Xu |
| Whether message is optional | **Yes**: `assert(cond, ?msg)`, String or Error | 2026-07-11 | Chen Xu |
| Whether assert_eq variants are needed | **Not needed**. YAGNI, wait until the test framework | 2026-07-11 | Chen Xu |
| Whether a separate raise/throw keyword is needed | **Not needed**. `assert(false, msg)` is equivalent to raise | 2026-07-11 | Chen Xu |
| Relationship between assert and Assert | **Completely independent**. Runtime function vs. compile-time conditional type, same module, different concepts | 2026-07-11 | Chen Xu |

## References

- [RFC-007: Function Definition Syntax Unification](007-function-syntax-unification.md) — `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — Type system foundation
- [RFC-011: Generic Type System Design §4.3](../accepted/011-generic-type-system.md) — Compile-time validation and `Assert(C)` conditional type
- [RFC-026: FFI Core Mechanism](../review/026-ffi-core-mechanism.md) — Native function registration mechanism
- [RFC-027: Compile-time Predicates and Unified Static Validation](../accepted/027-compile-time-evaluation-types.md) — Compile-time evaluation system