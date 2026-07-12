---
title: "RFC-030: assert Assertion Mechanism"
status: "Under Review"
author: "晨煦"
created: "2026-06-15"
updated: "2026-07-11"
decision: "Option A — native function. 20-line implementation, first-class citizen, zero syntax changes."
issue: "#97"
issues_impl:
  - "#155"
---

# RFC-030: assert Assertion Mechanism

## Summary

Introduce the `assert` assertion mechanism to YaoXiang for testing and precondition checks. This RFC proposes two implementation approaches—**native function** and **keyword**—analyzes the trade-offs of each, and presents them for decision-making.

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

1. **Lots of boilerplate**: Each assertion requires 4 lines, causing test files to bloat
2. **Weak error messages**: Manually concatenating strings, lacking source code location information
3. **Not composable**: Cannot register assertions in batch, cannot be passed as arguments to test frameworks

### Current problems

- No unified assertion mechanism
- Test code is filled with the `if` + print + `return` pattern
- The bytecode layer already has the `Throw` instruction, but the language layer does not expose it

## Option A: native function

Implement `assert` as a native function, without introducing new keywords.

```yaoxiang
use std.assert

main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### Type signature

```
assert: (Bool, String) -> ()
```

### Runtime behavior

1. Evaluate the first argument `condition: Bool`
2. If `true`, return `Unit`
3. If `false`, trigger a runtime panic:
   - Output the contents of `message`
   - Output the call stack (in debug mode)
   - Terminate the current execution

### Compiler changes

**No changes needed to parser, AST, typecheck, or IR gen.**

Only need to add native function registration under `src/std/`:

1. Add a new `src/std/assert.rs`
2. Register the `std.assert` native function
3. Internally call the existing runtime panic mechanism

### Advantages

- **Zero syntax changes**: No need to modify parser, AST, IR, or codegen
- **Simple to implement**: About 20 lines of native function registration code
- **First-class citizen**: Can be passed as arguments, assigned to variables, used in higher-order functions
- **Progressive enhancement**: Future compile-time inlining optimizations can be added without breaking code

### Disadvantages

- **No compile-time analysis**: The compiler cannot perform dead code elimination on code after `assert(false)`
- **Limited error messages**: Source code location must be obtained through the runtime call stack, not as precise as a keyword
- **Can be shadowed**: Users can define a function with the same name to override `assert`
- **Requires import**: Must write `use std.assert` to use it

---

## Option B: keyword

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

### Compiler changes

Need to modify the full pipeline:

| Phase | Change |
|------|------|
| Lexer | Add `KwAssert` token |
| Parser | Add `assert(expr, "msg")` syntax |
| AST | Add `StmtKind::Assert { condition, message }` |
| IR Gen | Compile Assert to conditional jump + Throw |
| Codegen | Generate `JmpIfNot` + `Throw` instruction sequence |

### Advantages

- **Built-in available**: No import needed, usable directly anywhere
- **Precise source location**: The compiler knows the location of assert at the AST layer, with error messages accurate to line and column
- **Cannot be shadowed**: Keywords cannot be overridden by user code
- **Future-extensible**: Compile-time analysis can be done (in conjunction with the constant folding pass)
- **Clear semantics**: `assert` has clear semantics at the syntax layer and cannot be misused

### Disadvantages

- **High implementation cost**: Need to modify parser, AST, IR, and codegen across the full pipeline
- **Not a first-class citizen**: Cannot be passed as arguments, cannot be assigned to variables
- **Keyword bloat**: Each new keyword increases language complexity
- **Poor extensibility**: Adding new signature variants (like `assert_eq`) requires syntax changes

---

## Comparison

| Dimension | Option A: function | Option B: keyword |
|------|-------------|---------------|
| Implementation cost | Low (~20 lines) | High (~200 lines, 4 modules) |
| Syntax changes | None | Yes (new keyword + syntax) |
| Requires import | Yes | No |
| First-class citizen | Yes | No |
| Source location precision | Runtime call stack | Compile-time, accurate to row/column |
| Cannot be shadowed | No | Yes |
| Compile-time analysis | Not possible (requires constant folding) | Possible (in conjunction with constant folding) |
| Extensibility | High (overloading, higher-order functions) | Low (requires syntax changes) |

### Practical constraints of compile-time analysis

Option B's core advantage—compile-time analysis—requires a **constant folding pass** to take effect. That is, the compiler needs to evaluate `false` in `assert(false)` at compile time in order to know it is dead code.

YaoXiang currently does not have a constant folding pass. Even if Option B is adopted, common usages like `assert(x > 0)` still cannot be analyzed at compile time. Only literals like `assert(true)` / `assert(false)` can be analyzed.

Therefore, Option B's compile-time advantage is **theoretical at the current stage, not practical**.

---

## Open questions

- [x] ~~Choose Option A or Option B?~~ → **Option A: native function**
- [ ] Does `assert` need to support a simplified form `assert(cond)` without `message`?
- [ ] Are variants like `assert_eq` and `assert_ne` needed?
- [x] ~~Does the panic output include source location?~~ → Option A depends on debug info (call stack), Option B is obtainable at compile time. Adopt Option A. Inlining capability can be naturally obtained after adding a constant folding pass later

---

## Appendix B: Design Decision Record

| Decision | Resolution | Date | Recorder |
| Choose Option A or Option B | **Option A: native function** — 20-line implementation, first-class citizen, zero syntax changes. Option B's compile-time advantage is theoretical at the current stage | 2026-07-03 | 晨煦 |

## References

- [RFC-007: Unified Function Definition Syntax](007-function-syntax-unification.md) — `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — Type system foundation
- [RFC-026: FFI Core Mechanism](../review/026-ffi-core-mechanism.md) — native function registration mechanism