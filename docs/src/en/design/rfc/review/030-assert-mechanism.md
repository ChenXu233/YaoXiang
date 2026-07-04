---
title: "RFC-030: assert Mechanism"
status: "Under Review"
author: "Chenxu"
created: "2026-06-15"
updated: "2026-07-03"
decision: "Plan A — native function. 20 lines of implementation, first-class citizen, zero syntax changes. Plan B's compile-time advantage is theoretical at the current stage."
---

# RFC-030: assert Mechanism

## Summary

Introduce an `assert` mechanism into YaoXiang for testing and precondition checks. This RFC proposes two implementation approaches—**native function** and **keyword**—analyzes their tradeoffs, and presents them for decision.

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

1. **Excessive boilerplate**: Each assertion requires 4 lines, bloating test files
2. **Weak error messages**: Manual string concatenation, missing source location
3. **Not composable**: Cannot register assertions in batch, cannot pass them as arguments to test frameworks

### Current Problems

- No unified assertion mechanism
- Test code is full of `if` + print + `return` patterns
- The bytecode layer already has a `Throw` instruction but it's not exposed at the language level

## Plan A: Native Function

Implement `assert` as a native function, without introducing a new keyword.

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
3. Internally call the existing runtime panic mechanism

### Advantages

- **Zero syntax changes**: No need to modify parser, AST, IR, or codegen
- **Simple implementation**: About 20 lines of native function registration code
- **First-class citizen**: Can be passed as arguments, assigned to variables, used in higher-order functions
- **Progressive enhancement**: Compile-time inlining optimization can be added later without breaking code

### Disadvantages

- **No compile-time analysis**: The compiler cannot perform dead code elimination on code after `assert(false)`
- **Limited error information**: Source location must be obtained through runtime call stack, less precise than keywords
- **Can be shadowed**: Users can define functions with the same name to override `assert`
- **Requires import**: Must write `use std.assert` to use it

---

## Plan B: Keyword

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

`assert` is a keyword, no `use` import needed.

### Compiler Changes

Requires changes across the full pipeline:

| Stage | Changes |
|-------|---------|
| Lexer | Add `KwAssert` token |
| Parser | Add `assert(expr, "msg")` syntax |
| AST | Add `StmtKind::Assert { condition, message }` |
| IR Gen | Compile Assert to conditional jump + Throw |
| Codegen | Generate `JmpIfNot` + `Throw` instruction sequence |

### Advantages

- **Built-in availability**: No import needed, available directly anywhere
- **Precise source location**: The compiler knows the assert's location at the AST layer, with error messages accurate to line and column
- **Cannot be shadowed**: Keywords cannot be overridden by user code
- **Future extensible**: Can perform compile-time analysis (combined with constant folding pass)
- **Clear semantics**: `assert` has clear semantics at the syntax level and won't be misused

### Disadvantages

- **High implementation cost**: Requires changes to parser, AST, IR, and codegen across the full pipeline
- **Not a first-class citizen**: Cannot be passed as arguments, cannot be assigned to variables
- **Keyword bloat**: Each new keyword increases language complexity
- **Poor extensibility**: Adding new signature variants (like `assert_eq`) requires syntax changes

---

## Comparison

| Dimension | Plan A: Function | Plan B: Keyword |
|-----------|------------------|-----------------|
| Implementation cost | Low (~20 lines) | High (~200 lines, 4 modules) |
| Syntax changes | None | Yes (new keyword + syntax) |
| Requires import | Yes | No |
| First-class citizen | Yes | No |
| Source location precision | Runtime call stack | Compile-time precise to line/column |
| Cannot be shadowed | No | Yes |
| Compile-time analysis | Not possible (requires constant folding) | Possible (combined with constant folding) |
| Extensibility | High (overloading, higher-order functions) | Low (requires syntax changes) |

### Realistic Constraints of Compile-Time Analysis

Plan B's core advantage—compile-time analysis—requires a **constant folding pass** to take effect. That is, the compiler must evaluate `false` in `assert(false)` at compile-time to know it's dead code.

YaoXiang currently has no constant folding pass. Even if Plan B is adopted, common patterns like `assert(x > 0)` still cannot be analyzed at compile-time. Only literals like `assert(true)` / `assert(false)` can be analyzed.

Therefore, Plan B's compile-time advantage **is theoretical, not actual, at the current stage**.

---

## Open Questions

- [x] ~~Choose Plan A or Plan B?~~ → **Plan A: native function**
- [ ] Does `assert` need to support a simplified form `assert(cond)` without `message`?
- [ ] Are variants like `assert_eq`, `assert_ne` needed?
- [x] ~~Does panic output include source location?~~ → Plan A depends on debug info (call stack), Plan B is available at compile-time. Adopt Plan A. Inlining capability can naturally be obtained later by adding a constant folding pass.

---

## Appendix B: Design Decision Records

| Decision | Determination | Date | Recorder |
|----------|---------------|------|----------|
| Choose Plan A or Plan B | **Plan A: native function** — 20 lines of implementation, first-class citizen, zero syntax changes. Plan B's compile-time advantage is theoretical at the current stage | 2026-07-03 | Chenxu |

## References

- [RFC-007: Function Definition Syntax Unification](007-function-syntax-unification.md) — `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — Type system foundation
- [RFC-026: FFI Core Mechanism](../review/026-ffi-core-mechanism.md) — Native function registration mechanism