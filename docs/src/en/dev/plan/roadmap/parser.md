---
title: "Parser State"
---

# Parser

> **Module Status**: Stable (5 items pending improvement)
> **Location**: `src/frontend/core/parser/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The parser is responsible for converting a Token stream into an AST (Abstract Syntax Tree). It uses the classic Pratt Parsing (Top-down Operator Precedence) algorithm and supports the complete YaoXiang language syntax specification.

**Code Size**: ~5000 lines (31 source files, of which 14 are test files)

---

## Feature List

### Expression Parsing (Pratt Parser)

**Prefix (nud)**:
- ✅ All literals: Int, Float, String, Char, Bool, FString
- ✅ Identifiers/variable references
- ✅ Unary operators: `-`, `+`, `not`, `*` (dereference)
- ✅ Borrow expressions: `&expr`, `&mut expr`
- ✅ Grouping/tuple: `(expr)`, `(a, b, c)`
- ✅ List literals and list comprehensions: `[1,2,3]`, `[x*x for x in items]`
- ✅ Block expressions: `{ stmts; expr }`
- ✅ Control flow: `if/elif/else`, `match`, `while`, `for`
- ✅ `ref` keyword (creates Arc)
- ✅ `unsafe` blocks
- ✅ `@block/@auto/@eager` evaluation strategy annotations
- ✅ `spawn` concurrent blocks
- ✅ `return`, `break`, `continue` (with optional labels)

**Infix (led)**:
- ✅ All binary operators: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `and`, `or`, `..`
- ✅ Assignment: `=`
- ✅ Function calls: `f(a, b)`, with named arguments `f(x=1, y=2)`
- ✅ Field access: `obj.field` (chained: `a.b.c`)
- ✅ Index access: `arr[0]` (chained: `m[i][j]`)
- ✅ Type casting: `expr as Type`
- ✅ Error propagation: `expr?`
- ✅ Lambda: `x => expr`, `(a, b) => expr`, `(x: Int) => x + 1`

**Precedence Levels (10 levels)**: Lowest(0) < Assign/Range(1) < Or(2) < And(3) < Eq(4) < Cmp(5) < Add(6) < Mul(7) < Unary/Cast(8) < Call(9) < Highest(10)

### Statement Parsing

- ✅ Variable declarations: `x = 42`, `x: Int = 42`, `mut x: Int = 0`, `pub x: Int = 42`
- ✅ Function definitions (RFC-010): `add: (a: Int, b: Int) -> Int = a + b`
- ✅ Type definitions (RFC-010): `Name: Type = { ... }`
- ✅ Method definitions (RFC-010): `Point.draw: (self: Point, s: Surface) -> Void = ...`
- ✅ External bindings (RFC-004): `Point.distance = distance[0]`
- ✅ Control flow: `if/elif/else`, `while`, `for [mut] item in iter`, `return`, `break [label]`, `continue [label]`
- ✅ Imports: `use path`, `use path.{a, b}`, `use path as alias`
- ✅ Evaluation strategy annotations (RFC-001/008): `@block`, `@auto`, `@eager`
- ✅ `pub` visibility modifier

### Type System Parsing

- ✅ Named types: `Int`, `String`, `Bool`, `Float`
- ✅ Meta types (MetaType): `Type` (RFC-010 core)
- ✅ Function types: `(Int, Float) -> Bool`
- ✅ Tuple types: `(Int, String, Bool)`
- ✅ Struct types: `{ x: Float, y: Float }`
- ✅ Enum/variant types: `{ red | green | blue }`, `{ ok(Int) | err(String) }`
- ✅ Generic types: `List(Int)`, `Map(String, Int)`
- ✅ Raw pointers: `*Int`
- ✅ Reference types: `&T`, `&mut T`
- ✅ Associated types: `T::Item`
- ✅ Literal types (const generics): `n: n`

### Error Recovery

- ✅ `parse()`: Returns `Err` on first error
- ✅ `parse_with_recovery()`: Always returns `ParseResult`, inserting `StmtKind::Error` / `Expr::Error` placeholder nodes at error positions
- ✅ `synchronize()` method: Jumps to the next statement boundary for recovery

---

## Test Coverage

**All 285 tests passed**, distributed across 14 test files:

| Test File | Test Count | Coverage Area |
|-----------|------------|---------------|
| `tests/ast.rs` | ~55 | Construction and matching of all AST node variants |
| `tests/expressions.rs` | ~28 | Literals, unary/binary operators, function calls, Lambda, control flow, etc. |
| `tests/integration.rs` | 5 | Full program parsing (mixed statements) |
| `tests/parser_state.rs` | 15 | State machine operations (bump, skip, save/restore, error tracking) |
| `tests/error_recovery.rs` | 6 | Error recovery (empty input, single/multiple errors, continue parsing after recovery) |
| `pratt/tests/nud.rs` | ~30 | Prefix parser routing and functionality |
| `pratt/tests/led.rs` | ~30 | Infix parser routing and functionality |
| `pratt/tests/precedence.rs` | 1 | Precedence order validation |
| `statements/tests/declarations.rs` | ~16 | Variable, function, type definitions, method definitions |
| `statements/tests/control_flow.rs` | ~10 | if/while/for/return/break/continue |
| `statements/tests/functions.rs` | 5 | Function definition variations |
| `statements/tests/imports.rs` | 4 | use statement variations |
| `statements/tests/types.rs` | ~20 | Type annotation parsing |
| `statements/tests/bindings.rs` | ~18 | Binding syntax (RFC-004/010) |

---

## RFC Comparison

| RFC | Implementation Status | Description |
|-----|----------------------|-------------|
| RFC-001 Concurrency Model | ✅ Implemented | `EvalMode` (Block/Auto/Eager) annotations |
| RFC-004 Curry Multi-position Binding | ✅ Implemented | `Type.method = func[0,1]` external binding syntax |
| RFC-007 Unified Function Syntax | ✅ Implemented | Lambda `(a, b) => body`, HM inference |
| RFC-008 Runtime Concurrency Model | ✅ Implemented | `spawn { ... }` blocks |
| RFC-010 Unified Type Syntax | ✅ Implemented | `name: type = value` unified model, `Type` meta type |
| RFC-011 Generic Type System | ✅ Implemented | `(T: Type, N: Int) -> Type` generic syntax |
| RFC-012 F-string Template Strings | ✅ Implemented | `f"Hello {name}"` parsed as FString nodes |
| RFC-017 LSP Support | ✅ Implemented | `parse_with_recovery()` + Error placeholder nodes |

---

## Code Quality Assessment

| Dimension | Score | Description |
|-----------|-------|-------------|
| Incomplete Items | 5 | Supplementary tests, placeholder bindings, Platform parsing |
| Test Coverage | Excellent | All 285 tests passed |
| Documentation Quality | Good | Sufficient file-level and function-level comments, clear RFC associations |
| Code Architecture | Excellent | Standard Pratt Parser implementation, clear modularity |
| RFC Compliance | Highly Compliant | RFC-001/004/007/008/010/011/012/017 all implemented |

---

## Pending Improvements

1. **Add Dict literal parsing tests**
2. **Add FString parsing end-to-end tests**
3. **Add end-to-end tests for `@block/@auto/@eager` and `spawn`**
4. **Implement placeholder `_` position binding** (RFC-004)
5. **Implement Platform parameter parsing** (RFC-011)