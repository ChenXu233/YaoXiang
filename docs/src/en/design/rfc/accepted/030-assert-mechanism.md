---
title: "RFC-030: assert Assertion Mechanism"
status: "Accepted"
author: "Chen Xu"
created: "2026-06-15"
updated: "2026-07-14"
decision: "assert and Assert are two sides of the same coin, with dispatch automatically routing. All 6 Phases are implemented (#157-#162 closed). std.assert module unified registration (#169 closed), assert native function and Assert/IsTrue type family share the same path."
issue: "#97"
issues_impl:
  - "#155"
  - "#157"
  - "#158"
  - "#159"
  - "#160"
  - "#161"
  - "#162"
  - "#169"
---

# RFC-030: assert Assertion Mechanism

## Summary

Introduce an `assert` assertion mechanism for YaoXiang, used for testing, precondition checks, and runtime panics. `assert` and the compile-time refinement type `Assert(C)` (see RFC-011 §4.3) are **two sides of the same refinement primitive**—dispatch automatically routes between compile-time proof and runtime check based on "whether the predicate's free variables are reachable at compile time." `assert(false, "msg")` is equivalent to `raise`; no separate `throw`/`raise` keyword is required.

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

This style has three problems:

1. **Boilerplate-heavy**: Each assertion requires 4 lines, bloating test files
2. **Weak error messages**: Manual string concatenation, lacking source location
3. **Not composable**: Cannot register assertions in bulk, cannot pass as arguments to test frameworks

### Current problems

- No unified assertion mechanism
- Test code is filled with the `if` + print + `return` pattern
- The `Throw` instruction already exists at the bytecode level but is not exposed at the language level
- RFC-011 defines the compile-time `Assert(C)` conditional type, but the runtime `assert()` is not yet implemented

### Design Principles

`assert` is YaoXiang's only user-level panic mechanism. `assert(false, "msg")` is equivalent to `raise`; no separate `throw`/`raise` keyword is required. The `assert` function itself is the best encapsulation of `if raise`.

**No new keywords, no new syntax. Everything is a function call.**

## Option A: Native Function

Implement `assert` as a native function, introducing no new keywords.

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
// Core signature: assert is the value-universe introducer for Assert
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))
//                                       ^^^^^^^^^^^^^^^^^^^^^^^^
//                                       Returns a refinement type, not ()
//
// IsTrue: Bool -> Type bridges truth values to types:
//   IsTrue(true)  = Void   (⊤, program continues)
//   IsTrue(false) = Never  (⊥, diverges/compile error)
```

The actual behavior of `assert` is determined by dispatch routing:
- All free variables are compile-time known → **CompileTime**: the compiler evaluates `cond`; `true` → erased to `Void`; `false` → compile error (Never cannot be inhabited)
- Runtime free variables exist → **Runtime**: insert a check, inject refinement facts into the flow-sensitive assumption set Γ

The optional message `?msg` and Result overload (see below) are retained as runtime raise payloads.

#### Overload 1: Condition Assertion `(Bool, ?String | Error)`

`Bool` + optional message. The message can be a `String` or an `Error` value:

```yaoxiang
assert(1 + 1 == 2)                    // No message, default panic info
assert(1 + 1 == 2, "math is broken")   // String message
assert(x > 0, my_error)                // Throw Error value directly
```

`assert(false, "msg")` is YaoXiang's equivalent of `raise`/`throw`—no separate keyword needed.

#### Overload 2: Result Assertion `(Result)`

A single `Result` argument, automatically checking if it is `Err`:

### Advantages

- **Zero syntax changes**: Pure function, no new keywords needed
- **Zero new concepts**: Reuses existing native function registration mechanism
- **High extensibility**: Function overloading naturally supports multiple signatures
- **Self-documenting**: The `std.assert` namespace itself is documentation

### Disadvantages

- None. When `assert`'s type signature is correct, the compiler can infer dead code through function reachability analysis. No additional pass needed.

### Runtime Behavior

1. Evaluate the first argument `condition: Bool`
2. If `true`, return `Unit`
3. If `false`, trigger a runtime panic:
   - Output the `message` content (if any)
   - Output the call stack (in debug mode)
   - Terminate the current execution

#### Failure Behavior of Each Overload

| Signature | Behavior on failure |
|------|-----------|
| `assert(false)` | Default panic message |
| `assert(false, "msg")` | Output string message then panic |
| `assert(false, error_val)` | Throw Error value |
| `assert(Err(x))` | Extract Err content and panic |

### Relationship with Compile-time Assert

`assert` and `Assert` are **two sides of the same refinement primitive**—the dispatch routing pipeline automatically chooses based on "whether the predicate's free variables are reachable at compile time":

| Condition | Routing | Behavior |
|------|------|------|
| All free variables are compile-time known | CompileTime → proof pipeline | Proved → erase, Disproved → compile error, Unknown → require proof |
| Runtime free variables exist | Runtime → insert check | Bool check + inject refinement facts into the flow-sensitive assumption set Γ |

```yaoxiang
use std.assert

# Compile-time known (generic parameter) — goes through CompileTime, zero runtime overhead
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N is a generic parameter, evaluated at compile time
}

# Runtime value — goes through Runtime, inserts a Bool check
x = read_int()
assert.assert(x > 0, "expected positive")  # Runtime check
```

> **2026-07-12 Unified Solution**: The previous "completely independent" conclusion has been superseded. `assert()` is the value introducer of `Assert`, automatically dispatched by dispatch.

### Compiler Changes

**No changes needed to parser, AST, typecheck, IR gen.**

Only need to add native function registration under `src/std/`:

1. Add new `src/std/assert.rs`
2. Register `std.assert.assert` and `std.assert.Assert` (the latter is the compile-time conditional type, see #155)
3. Internally invoke the existing `BytecodeInstr::Throw` instruction

### Advantages

- **Zero syntax changes**: Pure function, no new keywords needed
- **Zero new concepts**: Reuses existing native function registration mechanism
- **High extensibility**: Function signatures can be extended to variants like `assert_eq` (future)
- **Self-documenting**: The `std.assert` namespace itself is documentation

### Disadvantages

- ~~Compile-time unknowable: unlike Option B (keyword), dead code elimination cannot be done at compile time~~ → **No longer holds under the unified solution**. CompileTime-mode assert goes through the proof pipeline; compile-time known cond → erased or compile error (`assert(false)` → Never → dead code).
- Call stack only available in debug mode

## Option B: Built-in Keyword (Superseded by the Unified Solution)

> Deprecated. The opposition between Options A and B is dissolved by the dispatch routing pipeline—`assert` is the value introducer of `Assert`; compile-time known goes through the proof pipeline (zero runtime overhead), runtime goes through check. No need to choose between "function" and "keyword". The following is historical record.

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### Type Signature

No independent type signature—the keyword is handled by the parser.

### Runtime Behavior

Same as Option A.

### Compiler Changes

Parser, AST, typecheck, IR gen need to be modified:
1. parser: add new `Expr::Assert` variant
2. AST: add new `Expr::Assert` node
3. typecheck: validate argument types
4. IR gen: generate `BytecodeInstr::Throw`

### Advantages

- Compile-time source location available (no dependency on debug info)
- Compile-time constant folding possible: `assert(true)` → no-op, `assert(false)` → compile error

### Disadvantages

| Disadvantage | Impact |
|------|------|
| Parser changes required | Introduces new syntax nodes, increases maintenance cost |
| Keyword not extensible | Variants like `assert_eq` still need functions |
| Compile-time advantage not practical | See analysis below |

### Comparison

| Dimension | Option A (Function) | Option B (Keyword) |
|------|---------------|-----------------|
| Implementation cost | ~20 lines | parser + AST + typecheck + IR gen |
| Syntax changes | None | New keyword |
| Extensibility | Function overloading | Requires companion macros |
| Source location | debug info | Compile-time available |
| Constant folding | Requires pass support | Compile-time available |
| Runtime overhead | Function call | Minimal |

### Realistic Constraints of Compile-time Analysis

Option B's core advantage—compile-time analysis—requires a **constant folding pass** to be effective. That is, the compiler needs to evaluate `false` in `assert(false)` at compile time to know it is dead code.

YaoXiang currently has no constant folding pass. Even with Option B, common patterns like `assert(x > 0)` still cannot be analyzed at compile time. Only literals like `assert(true)` / `assert(false)` can be analyzed.

Therefore, Option B's compile-time advantage **is theoretical at the current stage, not practical**.

---

## Open Questions

- [x] ~~Choose Option A or Option B?~~ → **Unified solution: assert is the value introducer of Assert**. The A/B opposition is dissolved by the dispatch routing pipeline—compile-time known goes through the proof pipeline, runtime goes through check. No "either/or" needed.
- [x] ~~Does `assert` need to support the simplified form `assert(cond)` without `message`?~~ → **Yes, supported. `assert(cond, ?msg)`, message is optional.**
- [x] ~~Are variants like `assert_eq`, `assert_ne` needed?~~ → **Not needed. YAGNI. Wait until the test framework matures.**
- [x] ~~Does the panic output include the source location?~~ → Option A depends on debug info (call stack).
- [x] ~~assert / Assert unification~~ → **Determined**. Unified solution: `assert: (Bool) -> Assert(IsTrue(cond))`, two sides of the same coin, dispatch automatically routes. See [#156](https://github.com/ChenXu233/YaoXiang/issues/156) (closed) for details. `Never` type (⊥) is builtin as the return type of `assert(false)`.

### 2026-07-05: Choosing Option A (Superseded by the Unified Solution)

Option A's 20-line implementation wins on value and cost. After the unified solution was determined on 2026-07-12, the A/B opposition is dissolved by the dispatch routing pipeline—`assert` is the value introducer of `Assert`; no longer choosing between "function" and "keyword".

### 2026-07-12: Unified Solution Determined (Supersedes the 2026-07-11 "Completely Independent" Conclusion)

**Conclusion**: `assert` and `Assert` are not two independent mechanisms. `assert: (Bool) -> Assert(IsTrue(cond))`—automatically dispatched by dispatch:

- Compile-time known → enters the proof pipeline (Proved erased / Disproved error / Unknown requires proof)
- Runtime input → inserts check + injects Γ assumption

**Module structure**: `std.assert` uniformly hosts runtime assertion (`assert`) and compile-time refinement types (`Assert`, `IsTrue`). No longer "implemented separately", but two sides of the same primitive.

### 2026-07-11: assert Overload Design

**Question**: Why does `assert` need two overloads, rather than a unified `(Bool, ?String)`?

**Answer**:

Runtime `assert()` is YaoXiang's only user-level panic mechanism. `assert(false, "msg")` is equivalent to `raise`/`throw` in other languages. Therefore it needs to cover three scenarios:
1. Condition + simple message: `assert(cond, "msg")`
2. Condition + custom Error: `assert(cond, my_error)`
3. Result check: `assert(result)` — the most concise `if is_err { panic }`

The rationale for the Result overload is that it is the shortest path for error propagation—"Result should be Ok, otherwise die". No need to call `.is_ok()` first and then handle the error separately.

## Appendix B: Design Decision Records

| Decision | Resolution | Date | Recorder |
|------|------|------|--------|
| Choose Option A or Option B | **Unified solution**: dispatch routing pipeline dissolves the A/B opposition, assert is the value introducer of Assert | 2026-07-12 | Chen Xu |
| Whether message is optional | **Yes**: `assert(cond, ?msg)`, String or Error | 2026-07-11 | Chen Xu |
| Whether variants like assert_eq are needed | **Not needed**. YAGNI, wait until test framework | 2026-07-11 | Chen Xu |
| Whether a separate raise/throw keyword is needed | **Not needed**. `assert(false, msg)` is equivalent to raise | 2026-07-11 | Chen Xu |
| Relationship between assert and Assert | **Two sides of the same coin**. `assert: (Bool) -> Assert(IsTrue(cond))`, dispatch automatically routes | 2026-07-12 | Chen Xu |

## References

- [RFC-007: Function Definition Syntax Unification Scheme](007-function-syntax-unification.md) — `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — type system foundation
- [RFC-011: Generics System Design §4.3](../accepted/011-generic-type-system.md) — compile-time verification and `Assert(C)` conditional type
- [RFC-026: FFI Core Mechanism](026-ffi-core-mechanism.md) — native function registration mechanism
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md) — compile-time evaluation system