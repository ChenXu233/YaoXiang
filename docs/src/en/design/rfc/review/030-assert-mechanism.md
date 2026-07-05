---
title: "RFC-030: assert Assertion Mechanism"
status: "Under Review"
author: "Chenxu"
created: "2026-06-15"
updated: "2026-07-05"
decision: "Option A — native function. 20-line implementation, first-class citizen, zero syntax changes. Option B's compile-time advantages are theoretical at the current stage."
---

# RFC-030: assert Assertion Mechanism

## Summary

Introduce an `assert` assertion mechanism into YaoXiang for testing and precondition checks. This RFC proposes two implementation approaches—**native function** and **keyword**—analyzing the trade-offs of each for decision-making.

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

1. **Excessive boilerplate code**: Every assertion requires 4 lines, causing test files to bloat
2. **Weak error messages**: Manual string concatenation lacks source location information
3. **Not composable**: Cannot batch-register assertions or pass them as arguments to test frameworks

### Current Problems

- No unified assertion mechanism
- Test code is saturated with the `if` + print + `return` pattern
- The bytecode layer already has a `Throw` instruction, but the language layer does not expose it

## Option A: native function

Implement `assert` as a native function without introducing a new keyword.

```yaoxiang
use std.assert

main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### Type Signature

```
assert: (Bool, String) -> ()
```

### Runtime Behavior

1. Evaluate the first argument `condition: Bool`
2. If `true`, return `Unit`
3. If `false`, trigger a runtime panic:
   - Output the `message` content
   - Output the call stack (in debug mode)
   - Terminate the current execution

### Compiler Changes

**No changes needed to parser, AST, typecheck, or IR gen.**

Only need to add native function registration under `src/std/`:

1. Add `src/std/assert.rs`
2. Register the `std.assert` native function
3. Internally invoke the existing runtime panic mechanism

### Advantages

- **Zero syntax changes**: No need to modify parser, AST, IR, or codegen
- **Simple implementation**: Approximately 20 lines of native function registration code
- **First-class citizen**: Can be passed as arguments, assigned to variables, used in higher-order functions
- **Progressive enhancement**: Compile-time inlining optimization can be added later without breaking code

### Disadvantages

- **No compile-time analysis**: The compiler cannot perform dead code elimination on code after `assert(false)`
- **Limited error messages**: Source location must be obtained through the runtime call stack, less precise than a keyword
- **Can be shadowed**: Users can define a function with the same name to override `assert`
- **Requires import**: Must write `use std.assert` to use it

---

## Option B: Keyword

Introduce `assert` as a language keyword, with the compiler handling it specially at the IR layer.

```yaoxiang
main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### Syntax

```
assert '(' Expr ',' StringLit ')'
```

`assert` is a keyword and does not require `use` import.

### Compiler Changes

Changes to the full pipeline are required:

| Stage | Changes |
|------|------|
| Lexer | Add `KwAssert` token |
| Parser | Add `assert(expr, "msg")` syntax |
| AST | Add `StmtKind::Assert { condition, message }` |
| IR Gen | Compile Assert as conditional jump + Throw |
| Codegen | Generate `JmpIfNot` + `Throw` instruction sequence |

### Advantages

- **Built-in availability**: No import required, can be used directly anywhere
- **Precise source location**: The compiler knows the position of the assert at the AST layer, with error messages precise to line and column
- **Cannot be shadowed**: Keywords cannot be overridden by user code
- **Future extensible**: Can perform compile-time analysis (in conjunction with a constant folding pass)
- **Clear semantics**: `assert` has clear semantics at the syntax layer and cannot be misused

### Disadvantages

- **High implementation cost**: Requires changes to parser, AST, IR, and codegen across the full pipeline
- **Not a first-class citizen**: Cannot be passed as arguments or assigned to variables
- **Keyword bloat**: Every added keyword increases language complexity
- **Poor extensibility**: Adding new signature variants (such as `assert_eq`) requires syntax changes

---

## Comparison

| Dimension | Option A: Function | Option B: Keyword |
|------|-------------|---------------|
| Implementation cost | Low (~20 lines) | High (~200 lines, 4 modules) |
| Syntax changes | None | Yes (adds keyword + syntax) |
| Requires import | Yes | No |
| First-class citizen | Yes | No |
| Source location precision | Runtime call stack | Compile-time, precise to line and column |
| Cannot be shadowed | No | Yes |
| Compile-time analysis | No (requires constant folding) | Yes (with constant folding) |
| Extensibility | High (overloading, higher-order functions) | Low (requires syntax changes) |

### Practical Constraints of Compile-Time Analysis

Option B's core advantage—compile-time analysis—requires a **constant folding pass** to take effect. That is, the compiler must evaluate `false` in `assert(false)` at compile-time to recognize it as dead code.

YaoXiang currently has no constant folding pass. Even if Option B is adopted, common patterns like `assert(x > 0)` still cannot be analyzed at compile-time. Only literals like `assert(true)` / `assert(false)` can be analyzed.

Therefore, Option B's compile-time advantages are **theoretical at the current stage, not practical**.

---

## Open Questions

- [x] ~~Choose Option A or Option B?~~ → **Option A: native function**
- [ ] Should `assert` support a simplified form `assert(cond)` without a `message`?
- [ ] Are variants such as `assert_eq`, `assert_ne` needed?
- [x] ~~Should panic output include source location?~~ → Option A relies on debug info (call stack), Option B is available at compile-time. Adopt Option A. Inlining capability can be naturally obtained later with the addition of a constant folding pass

---

## Appendix B: Design Decision Record

| Decision | Decision | Date | Recorder |
| Choose Option A or Option B | **Option A: native function** — 20-line implementation, first-class citizen, zero syntax changes. Option B's compile-time advantages are theoretical at the current stage | 2026-07-03 | Chenxu |

## References

- [RFC-007: Function Definition Syntax Unification Plan](007-function-syntax-unification.md) — `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — type system foundation
- [RFC-026: FFI Core Mechanism](../review/026-ffi-core-mechanism.md) — native function registration mechanism