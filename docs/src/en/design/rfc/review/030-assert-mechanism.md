---
title: "RFC-030: assert Mechanism"
status: "Under Review"
author: "Chenxu"
created: "2026-06-15"
updated: "2026-07-11"
decision: "Assert and assert: compile-time and runtime assertion."
issue: "#97"
issues_impl:
  - "#155"
---

# RFC-030: assert Mechanism

## Summary

Introduce the `assert` mechanism to YaoXiang, for testing, precondition checking, and runtime panic. `assert` and the compile-time refinement type `Assert(C)` (see RFC-011 §4.3) are **two sides of the same refinement primitive**—dispatched automatically by whether the predicate's free variables are reachable at compile time, routing the call either to compile-time proof or runtime check. `assert(false, "msg")` is equivalent to `raise`; no separate `throw`/`raise` keyword is needed.

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

This pattern has three problems:

1. **Heavy boilerplate**: each assertion takes 4 lines, bloating test files
2. **Weak error messages**: manual string concatenation, no source location
3. **Not composable**: cannot register assertions in bulk, cannot pass them to test frameworks as arguments

### Current Problems

- No unified assertion mechanism
- Test code is cluttered with the `if` + print + `return` pattern
- The `Throw` bytecode instruction already exists but is not exposed at the language level
- RFC-011 defines the compile-time `Assert(C)` conditional type, but the runtime `assert()` is not yet implemented

### Design Principles

`assert` is YaoXiang's only user-mode panic mechanism. `assert(false, "msg")` is equivalent to `raise`; no separate `throw`/`raise` keyword is needed. The `assert` function itself is the best encapsulation of `if raise`.

**No new keywords, no new syntax. Everything is a function call.**

## Approach A: native function

Implement `assert` as a native function, with no new keyword.

```yaoxiang
use std.assert.assert

main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### Overload Signatures

`assert` has two overloads:

```
// Core signature: assert is the value-universe introducer of Assert
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))
//                                       ^^^^^^^^^^^^^^^^^^^^^^^^
//                                       Returns a refinement type, not ()
//
// IsTrue: Bool -> Type is the truth-to-type bridge:
//   IsTrue(true)  = Void   (⊤, program continues)
//   IsTrue(false) = Never  (⊥, diverges / compile error)
```

The actual behavior of `assert` is determined by dispatch:
- All free variables known at compile time → **CompileTime**: the compiler evaluates `cond`; `true` → erased to Void; `false` → compile error (Never cannot be inhabited)
- Some free variables are runtime values → **Runtime**: insert a check, inject a refinement fact into the flow-sensitive assumption set Γ

The optional message `?msg` and the Result overload (see below) are preserved as the runtime raise payload.

#### Overload 1: Conditional assertion `(Bool, ?String | Error)`

`Bool` + optional message. The message can be a `String` or an `Error` value:

```yaoxiang
assert(1 + 1 == 2)                    // no message, default panic info
assert(1 + 1 == 2, "math is broken")   // string message
assert(x > 0, my_error)                // directly throw an Error value
```

`assert(false, "msg")` is the YaoXiang equivalent of `raise`/`throw`—no separate keyword needed.

#### Overload 2: Result assertion `(Result)`

A single `Result` argument, automatically checking whether it is `Err`:

### Advantages

- **Zero syntax change**: pure function, no new keyword
- **Zero new concepts**: reuses the existing native function registration mechanism
- **High extensibility**: function overloading naturally supports multiple signatures
- **Self-documenting**: the `std.assert` namespace itself serves as documentation

### Disadvantages

- None. When the type signature of `assert` is correct, the compiler can infer dead code through function reachability analysis. No additional pass is required.

### Runtime Behavior

1. Evaluate the first argument `condition: Bool`
2. If `true`, return `Unit`
3. If `false`, trigger a runtime panic:
   - Output the `message` content (if any)
   - Output the call stack (in debug mode)
   - Terminate the current execution

#### Failure behavior per overload

| Signature | Behavior on failure |
|------|-----------|
| `assert(false)` | Default panic message |
| `assert(false, "msg")` | Output the string message, then panic |
| `assert(false, error_val)` | Throw the Error value |
| `assert(Err(x))` | Extract the Err content and panic |

### Relationship with Compile-time Assert

`assert` and `Assert` are **two sides of the same refinement primitive**—automatically selected by the dispatch pipeline based on whether the predicate's free variables are reachable at compile time:

| Condition | Dispatch | Behavior |
|------|------|------|
| All free variables known at compile time | CompileTime → proof pipeline | Proved → erase, Disproved → compile error, Unknown → require proof |
| Some free variables are runtime values | Runtime → insert check | Bool check + inject a refinement fact into the flow-sensitive assumption set Γ |

```yaoxiang
use std.assert

# Compile-time known (generic parameter) — goes through CompileTime, zero runtime overhead
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N is a generic parameter, evaluated at compile time
}

# Runtime value — goes through Runtime, insert Bool check
x = read_int()
assert.assert(x > 0, "expected positive")  # runtime check
```

> **2026-07-12 Unified Approach**: the previous "fully independent" conclusion has been superseded. `assert()` is the value-introducer of `Assert`, dispatched automatically.

### Compiler Changes

**No changes needed to parser, AST, typecheck, or IR gen.**

Only need to add native function registration under `src/std/`:

1. Add a new `src/std/assert.rs`
2. Register `std.assert.assert` and `std.assert.Assert` (the latter is the compile-time conditional type, see #155)
3. Internally call the existing `BytecodeInstr::Throw` instruction

### Advantages

- **Zero syntax change**: pure function, no new keyword
- **Zero new concepts**: reuses the existing native function registration mechanism
- **High extensibility**: the function signature can be extended to variants like `assert_eq` (future)
- **Self-documenting**: the `std.assert` namespace itself serves as documentation

### Disadvantages

- ~~Not compile-time visible: unlike Approach B (keyword), it cannot perform dead-code elimination at compile time~~ → **No longer valid under the unified approach**. CompileTime-mode `assert` goes through the proof pipeline; a cond known at compile time → erase or compile error (`assert(false)` → Never → dead code).
- Call stack only available in debug mode

## Approach B: Built-in Keyword (Superseded by the Unified Approach)

> Deprecated. The A/B opposition is dissolved by the dispatch pipeline—`assert` is the value-introducer of `Assert`; compile-time-known conditions go through the proof pipeline (zero runtime overhead), runtime values go through check. There is no need to choose between "function" and "keyword". The following is kept as historical record.

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### Type Signature

No independent type signature—handled by the parser as a keyword.

### Runtime Behavior

Same as Approach A.

### Compiler Changes

Requires changes to parser, AST, typecheck, and IR gen:
1. parser: add `Expr::Assert` variant
2. AST: add `Expr::Assert` node
3. typecheck: validate argument types
4. IR gen: emit `BytecodeInstr::Throw`

### Advantages

- Source location known at compile time (no dependency on debug info)
- Constant folding at compile time: `assert(true)` → no-op, `assert(false)` → compile error

### Disadvantages

| Disadvantage | Impact |
|------|------|
| Parser changes required | Introduces a new syntax node, increases maintenance cost |
| Keyword is not extensible | variants like `assert_eq` still need to be functions |
| Compile-time advantage is impractical | see analysis below |

### Comparison

| Dimension | Approach A (function) | Approach B (keyword) |
|------|---------------|-----------------|
| Implementation cost | ~20 lines | parser + AST + typecheck + IR gen |
| Syntax change | None | New keyword |
| Extensibility | Function overloading | Requires companion macro |
| Source location | debug info | Compile-time available |
| Constant folding | Requires pass support | Compile-time available |
| Runtime overhead | Function call | Negligible |

### Real-world Constraints of Compile-time Analysis

The core advantage of Approach B—compile-time analysis—requires a **constant folding pass** to take effect. That is, the compiler must evaluate the `false` in `assert(false)` at compile time to know it is dead code.

YaoXiang currently has no constant folding pass. Even with Approach B, common forms like `assert(x > 0)` still cannot be analyzed at compile time. Only literal cases like `assert(true)` / `assert(false)` can be analyzed.

Therefore, the compile-time advantage of Approach B **is theoretical, not practical, at the current stage**.

---

## Open Questions

- [x] ~~Choose Approach A or Approach B?~~ → **Unified approach: `assert` is the value-introducer of `Assert`**. The A/B opposition is dissolved by the dispatch pipeline—compile-time-known conditions go through the proof pipeline, runtime values go through check. No "either/or" is required.
- [x] ~~Does `assert` need a simplified form `assert(cond)` without `message`?~~ → **Supported. `assert(cond, ?msg)`, message is optional.**
- [x] ~~Are variants like `assert_eq`, `assert_ne` needed?~~ → **No. YAGNI. Wait until the test framework takes shape.**
- [x] ~~Does the panic output include source location?~~ → Approach A depends on debug info (call stack).
- [x] ~~Unification of `assert` / `Assert`~~ → **Resolved**. Unified approach: `assert: (Bool) -> Assert(IsTrue(cond))`, two sides of one coin, dispatched automatically. See [#156](https://github.com/ChenXu233/YaoXiang/issues/156) (closed) for details. The `Never` type (⊥) is built in as the return type of `assert(false)`.

### 2026-07-05: Choosing Approach A (Superseded by the Unified Approach)

Approach A's 20-line implementation wins on value vs. cost. After the 2026-07-12 unified approach was finalized, the A/B opposition is dissolved by the dispatch pipeline—`assert` is the value-introducer of `Assert`, no longer an "either function or keyword" choice.

### 2026-07-12: Unified Approach Finalized (Supersedes the 2026-07-11 "Fully Independent" Conclusion)

**Conclusion**: `assert` and `Assert` are not two independent mechanisms. `assert: (Bool) -> Assert(IsTrue(cond))`—dispatched automatically:

- Compile-time known → enters the proof pipeline (Proved erases / Disproved errors / Unknown requires proof)
- Runtime input → inserts a check + injects a Γ assumption

**Module structure**: `std.assert` uniformly hosts the runtime assertion (`assert`) and the compile-time refinement types (`Assert`, `IsTrue`). No more "implement separately"—they are two sides of the same primitive.

### 2026-07-11: assert Overload Design

**Question**: Why does `assert` need two overloads instead of a unified `(Bool, ?String)`?

**Answer**:

The runtime `assert()` is YaoXiang's only user-mode panic mechanism. `assert(false, "msg")` is equivalent to `raise`/`throw` in other languages. Therefore, it must cover three scenarios:
1. Condition + simple message: `assert(cond, "msg")`
2. Condition + custom Error: `assert(cond, my_error)`
3. Result check: `assert(result)` — the most concise form of `if is_err { panic }`

The Result overload is justified because it is the shortest path for error propagation—"a Result should be Ok, or die." It avoids the need to call `.is_ok()` and then handle the error separately.

## Appendix B: Design Decision Log

| Decision | Resolution | Date | Recorder |
|------|------|------|--------|
| Choose Approach A or B | **Unified approach**: dispatch pipeline dissolves the A/B opposition; `assert` is the value-introducer of `Assert` | 2026-07-12 | Chenxu |
| Whether the message is optional | **Yes**: `assert(cond, ?msg)`, `String` or `Error` | 2026-07-11 | Chenxu |
| Are variants like `assert_eq` needed | **No**. YAGNI, wait until the test framework takes shape | 2026-07-11 | Chenxu |
| Is a separate `raise`/`throw` keyword needed | **No**. `assert(false, msg)` is equivalent to `raise` | 2026-07-11 | Chenxu |
| Relationship between `assert` and `Assert` | **Two sides of one coin**. `assert: (Bool) -> Assert(IsTrue(cond))`, dispatched automatically | 2026-07-12 | Chenxu |

## References

- [RFC-007: Function Definition Syntax Unification](007-function-syntax-unification.md) — the `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — type system foundations
- [RFC-011: Generic Type System Design §4.3](../accepted/011-generic-type-system.md) — compile-time verification and the `Assert(C)` conditional type
- [RFC-026: FFI Core Mechanism](../review/026-ffi-core-mechanism.md) — native function registration mechanism
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md) — compile-time evaluation system