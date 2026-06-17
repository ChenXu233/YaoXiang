---
title: "RFC-030: assert Mechanism"
status: "Draft"
author: "晨煦 (Chenxu)"
created: "2026-06-15"
---

# RFC-030: assert Mechanism

## Summary

Introduce an `assert` mechanism for YaoXiang, used for testing and precondition checks. This RFC proposes two implementation approaches—**native function** and **keyword**—analyzes the trade-offs of each, and presents them for decision.

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

1. **Lots of boilerplate**: Every assertion takes 4 lines, bloating test files.
2. **Weak error messages**: Manually concatenating strings, missing source location.
3. **Not composable**: Cannot batch-register assertions, cannot pass them as arguments to a testing framework.

### Current Problems

- No unified assertion mechanism.
- Test code is full of the `if` + print + `return` pattern.
- The `Throw` instruction already exists in the bytecode layer, but is not exposed at the language level.

## Option A: native function

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

1. Evaluate the first argument `condition: Bool`.
2. If it is `true`, return `Unit`.
3. If it is `false`, trigger a runtime panic:
   - Output the `message` content.
   - Output the call stack (in debug mode).
   - Terminate the current execution.

### Compiler Changes

**No changes are needed to parser, AST, typecheck, or IR gen.**

Only a native function registration needs to be added under `src/std/`:

1. Add `src/std/assert.rs`.
2. Register the `std.assert` native function.
3. Internally invoke the existing runtime panic mechanism.

### Pros

- **Zero syntax changes**: No changes to parser, AST, IR, or codegen.
- **Simple implementation**: About 20 lines of native function registration code.
- **First-class citizen**: Can be passed as an argument, assigned to a variable, or used in higher-order functions.
- **Incrementally extensible**: Future compile-time inlining optimizations can be added without breaking code.

### Cons

- **No compile-time analysis**: The compiler cannot perform dead code elimination after `assert(false)`.
- **Limited error messages**: The source location must be obtained from the runtime call stack, less precise than with a keyword.
- **Can be shadowed**: Users can define a function with the same name to override `assert`.
- **Requires import**: Must write `use std.assert` to use it.

---

## Option B: Keyword

Introduce `assert` as a language keyword, with the compiler handling it specially in the IR layer.

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

`assert` is a keyword and does not require `use` to import.

### Compiler Changes

Changes are needed across the full pipeline:

| Stage | Changes |
|------|---------|
| Lexer | Add `KwAssert` token |
| Parser | Add `assert(expr, "msg")` syntax |
| AST | Add `StmtKind::Assert { condition, message }` |
| IR Gen | Compile Assert into a conditional jump + Throw |
| Codegen | Emit `JmpIfNot` + `Throw` instruction sequence |

### Pros

- **Built-in availability**: No import required; usable directly anywhere.
- **Precise source location**: The compiler knows the location of the assert at the AST layer, so error messages are precise down to line and column.
- **Cannot be shadowed**: Keywords cannot be overridden by user code.
- **Future extensible**: Compile-time analysis can be performed (in conjunction with a constant folding pass).
- **Clear semantics**: `assert` has explicit semantics at the syntactic level, preventing misuse.

### Cons

- **High implementation cost**: Changes are needed in parser, AST, IR, and codegen across the full pipeline.
- **Not a first-class citizen**: Cannot be passed as an argument or assigned to a variable.
- **Keyword bloat**: Every added keyword increases language complexity.
- **Poor extensibility**: Adding new signature variants (e.g., `assert_eq`) requires grammar changes.

---

## Comparison

| Dimension | Option A: Function | Option B: Keyword |
|------|-------------|---------------|
| Implementation cost | Low (~20 lines) | High (~200 lines, 4 modules) |
| Syntax changes | None | Yes (new keyword + syntax) |
| Requires import | Yes | No |
| First-class citizen | Yes | No |
| Source location precision | Runtime call stack | Compile-time, precise to line/column |
| Cannot be shadowed | No | Yes |
| Compile-time analysis | Not possible (requires constant folding) | Possible (with constant folding) |
| Extensibility | High (overloading, higher-order functions) | Low (grammar changes required) |

### Realistic Constraints of Compile-Time Analysis

The core advantage of Option B—compile-time analysis—requires a **constant folding pass** to take effect. That is, the compiler must evaluate `false` in `assert(false)` at compile time in order to recognize it as dead code.

YaoXiang currently has no constant folding pass. Even with Option B, common usages like `assert(x > 0)` still cannot be analyzed at compile time. Only literals like `assert(true)` / `assert(false)` can be analyzed.

Therefore, the compile-time advantage of Option B is **theoretical, not actual, at the current stage**.

---

## Open Questions

- [ ] Choose Option A or Option B?
- [ ] Does `assert` need to support a simplified form `assert(cond)` without a `message`?
- [ ] Are variants such as `assert_eq` and `assert_ne` needed?
- [ ] Should the panic output include source location? (Option A depends on debug info; Option B can obtain it at compile time.)

---

## Appendix B: Design Decision Log

| Decision | Resolution | Date | Recorder |
|------|------|------|--------|
| (TBD) | | | |

## References

- [RFC-007: Function Definition Syntax Unification](007-function-syntax-unification.md) — `name: type = value` model
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — Type system foundations
- [RFC-026: FFI Core Mechanism](../review/026-ffi-core-mechanism.md) — Native function registration mechanism