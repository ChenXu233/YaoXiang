---
title: "Parser State"
---

# Parser

> **Module Status**: Stable (5 items to improve)
> **Location**: `src/frontend/core/parser/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The Parser is responsible for converting the Token stream into an AST (Abstract Syntax Tree). It uses the classic Pratt Parsing (top-down operator precedence) algorithm and supports the complete YaoXiang language syntax specification.

**Code Volume**: approximately 5,000 lines (31 source files, of which 14 are test files)

---

## Feature List

### Expression Parsing (Pratt Parser)

**Prefix (nud)**:
- ✅ All literals: Int, Float, String, Char, Bool, FString
- ✅ Identifier / variable reference
- ✅ Unary operators: `-`, `+`, `not`, `*` (dereference)
- ✅ Borrow expressions: `&expr`, `&mut expr`
- ✅ Grouping / tuple: `(expr)`, `(a, b, c)`
- ✅ List literals and list comprehensions: `[1,2,3]`, `[x*x for x in items]`
- ✅ Block expression: `{ stmts; expr }`
- ✅ Control flow: `if/elif/else`, `match`, `while`, `for`
- ✅ `ref` keyword (create Arc)
- ✅ `unsafe` block
- ✅ `spawn` concurrent block (RFC-024)
- ✅ `return`, `break`, `continue` (with optional label)

**Infix (led)**:
- ✅ All binary operators: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `and`, `or`, `..`
- ✅ Assignment: `=`
- ✅ Function call: `f(a, b)`, including named arguments `f(x=1, y=2)`
- ✅ Field access: `obj.field` (chained: `a.b.c`)
- ✅ Index access: `arr[0]` (chained: `m[i][j]`)
- ✅ Type cast: `expr as Type`
- ✅ Error propagation: `expr?`
- ✅ Lambda: `x => expr`, `(a, b) => expr`, `(x: Int) => x + 1`

**Precedence Hierarchy (10 levels)**: Lowest(0) < Assign/Range(1) < Or(2) < And(3) < Eq(4) < Cmp(5) < Add(6) < Mul(7) < Unary/Cast(8) < Call(9) < Highest(10)

### Statement Parsing

- ✅ Variable declaration: `x = 42`, `x: Int = 42`, `mut x: Int = 0`, `pub x: Int = 42`
- ✅ Function definition (RFC-010): `add: (a: Int, b: Int) -> Int = a + b`
- ✅ Type definition (RFC-010): `Name: Type = { ... }`
- ✅ Method definition (RFC-010): `Point.draw: (self: Point, s: Surface) -> Void = ...`
- ✅ External binding (RFC-004): `Point.distance = distance[0]`
- ✅ Control flow: `if/elif/else`, `while`, `for [mut] item in iter`, `return`, `break [label]`, `continue [label]`
- ✅ Import: `use path`, `use path.{a, b}`, `use path as alias`
- ✅ `pub` visibility modifier

### Type System Parsing

- ✅ Named types: `Int`, `String`, `Bool`, `Float`
- ✅ Meta type (MetaType): `Type` (RFC-010 core)
- ✅ Function type: `(Int, Float) -> Bool`
- ✅ Tuple type: `(Int, String, Bool)`
- ✅ Struct type: `{ x: Float, y: Float }`
- ✅ Enum / value variant type: `{ red | green | blue }`, `{ ok(Int) | err(String) }`
- ✅ Generics type: `List(Int)`, `Map(String, Int)`
- ✅ Raw pointer: `*Int`
- ✅ Reference type: `&T`, `&mut T`
- ✅ Associated type: `T::Item`
- ✅ Literal type (const generics): `n: n`

### Error Recovery

- ✅ `parse()`: returns `Err` on the first error encountered
- ✅ `parse_with_recovery()`: always returns `ParseResult`, inserts `StmtKind::Error` / `Expr::Error` placeholder nodes at error positions
- ✅ `synchronize()` method: jumps to the next statement boundary for recovery

---

## Test Coverage

**All 285 tests pass**, distributed across 14 test files:

| Test File | Test Count | Coverage |
|----------|--------|----------|
| `tests/ast.rs` | ~55 | Construction and matching of all AST node variants |
| `tests/expressions.rs` | ~28 | Literals, unary / binary operators, function calls, Lambda, control flow, etc. |
| `tests/integration.rs` | 5 | Complete program parsing (mixed statements) |
| `tests/parser_state.rs` | 15 | State machine operations (bump, skip, save/restore, error tracking) |
| `tests/error_recovery.rs` | 6 | Error recovery (empty input, single / multiple errors, continuing after recovery) |
| `pratt/tests/nud.rs` | ~30 | Prefix parser routing and functionality |
| `pratt/tests/led.rs` | ~30 | Infix parser routing and functionality |
| `pratt/tests/precedence.rs` | 1 | Precedence order verification |
| `statements/tests/declarations.rs` | ~16 | Variable, function, type, method definitions |
| `statements/tests/control_flow.rs` | ~10 | if/while/for/return/break/continue |
| `statements/tests/functions.rs` | 5 | Various forms of function definitions |
| `statements/tests/imports.rs` | 4 | Various forms of use statements |
| `statements/tests/types.rs` | ~20 | Type annotation parsing |
| `statements/tests/bindings.rs` | ~18 | Binding syntax (RFC-004/010) |

---

## RFC Comparison

| RFC | Implementation Status | Notes |
|-----|----------|------|
| RFC-001 Concurrency Model | ✅ Implemented | `EvalMode` (Block/Auto/Eager) annotation |
| RFC-004 Curry Multi-position Binding | ✅ Implemented | `Type.method = func[0,1]` external binding syntax |
| RFC-007 Unified Function Syntax | ✅ Implemented | Lambda `(a, b) => body`, HM inference |
| RFC-008 Runtime Concurrency Model | ✅ Implemented | `spawn { ... }` block |
| RFC-010 Unified Type Syntax | ✅ Implemented | `name: type = value` unified model, `Type` meta type |
| RFC-011 Generic Type System | ✅ Implemented | `(T: Type, N: Int) -> Type` generic syntax |
| RFC-012 F-string Template String | ✅ Implemented | `f"Hello {name}"` parsed as FString node |
| RFC-017 LSP Support | ✅ Implemented | `parse_with_recovery()` + Error placeholder nodes |

---

## Code Quality Assessment

| Dimension | Rating | Notes |
|------|------|------|
| Outstanding Items | 5 | Supplementary tests, placeholder binding, Platform parsing |
| Test Coverage | Excellent | All 285 tests pass |
| Documentation Quality | Good | Sufficient file-level and function-level comments, clear RFC references |
| Code Architecture | Excellent | Standard Pratt Parser implementation, clearly modularized |
| RFC Compliance | Highly Compliant | RFC-001/004/007/008/010/011/012/017 all implemented |

---

## Items to Improve

1. **Add Dict literal parsing tests**
2. **Add end-to-end FString parsing tests**
3. **Add end-to-end `spawn` parsing tests**
4. **Implement placeholder `_` positional binding** (RFC-004)
5. **Implement Platform parameter parsing** (RFC-011)