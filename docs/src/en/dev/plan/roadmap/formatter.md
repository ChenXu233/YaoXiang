---
title: "Formatter Status"
---

# Formatter

> **Module Status**: Stable (0 items to improve)
> **Location**: `src/formatter/`
> **Last Updated**: 2026-06-04

---

## Module Overview

The Formatter is responsible for auto-formatting YaoXiang source code. It supports full AST node formatting, including expressions, statements, types, etc.

**Code Volume**: 2,397 lines (19 source files)

---

## Feature List

### Implemented Features

**Expression Formatting** (handlers/expr.rs, covering all Expr variants):
- ✅ Literals: Int / Float / Bool / Char / String (with escape)
- ✅ Variable references
- ✅ Binary operators / Unary operators (with automatic line wrapping when line width is exceeded)
- ✅ Function calls (with named arguments)
- ✅ Function definition (expression form `fn def`)
- ✅ if/elif/else
- ✅ match expressions (pattern alignment, wraps when too long)
- ✅ for / while loops (with labels)
- ✅ Code blocks (with comment preservation, trailing comment handling)
- ✅ return / break / continue
- ✅ Type conversion (`as`)
- ✅ Tuple / List / List comprehension / Dict
- ✅ Index / Field access (chain call wrapping)
- ✅ try operator (`?`)
- ✅ ref / borrow (`&` / `&mut`)
- ✅ unsafe block
- ✅ spawn block
- ✅ lambda expression (single-expression concise form)
- ✅ f-string (with format spec)
- ✅ Error nodes (insert `/* error */` placeholder)

**Statement Formatting** (handlers/stmt.rs):
- ✅ Variable declarations (`mut` / type annotation / initializer)
- ✅ for loop statement
- ✅ Unified binding statement (function / type / method binding)
- ✅ `use` import statement (with items, alias)
- ✅ if statement
- ✅ External binding statement (External / Anonymous / DefaultExternal)

**Type Formatting** (handlers/types.rs, covering all Type variants):
- ✅ Basic types: Int(size) / Float(size) / Char / String / Bytes / Bool / Void
- ✅ Named type / Struct / Named struct
- ✅ Union / Enum / Variant
- ✅ Tuple / Function type / Option / Result
- ✅ Generics / Associated type / Sum type
- ✅ Literal type / Reference type / Pointer type / MetaType

**Other Features**:
- ✅ Delimited list auto-wrap (handlers/delimited.rs)
- ✅ Comment preservation (source_map.rs)
- ✅ Import sorting (rules/sort_imports.rs)
- ✅ CLI commands: check / write / stdout modes (command.rs)
- ✅ Configuration options: line_width / indent_width / use_tabs / single_quote / sort_imports (options.rs)

---

## Not Fully Implemented Specifications

| Specification | Status | Difference Description |
|---------------|--------|------------------------|
| §2.2 Wrap Strategy Priority | ✅ Implemented | Full priority chain implemented |
| §4.2 Parameter List Wrap (Trailing Comma) | ✅ Implemented | Parameter list auto-wraps when exceeding line width, uses trailing comma |
| §6.2 Single-line Code Block | ✅ Implemented | Single-statement code blocks that don't exceed line width use single-line format |
| §6.3 Empty Code Block | ✅ Implemented | Empty code blocks output `{}` rather than `{\n}` |
| §8.3 Single Quote Mode | ✅ Implemented | `single_quote` configuration is in effect |
| §E2 Exit Codes | ✅ Implemented | Uses specific exit codes per specification |

---

## Test Coverage

**77 tests + 1 proptest idempotency test**:

| Test Group | Count | Coverage |
|------------|-------|----------|
| handlers/tests/expr | 50 | literals, binary ops, function calls, lists, dicts, return, cast, match, f-string, try, unsafe, syntax error tolerance, single quote mode, empty code block, single-line code block, parameter list wrap, **variable references, return/break/continue, tuples, index access, field access, ref, error recovery** |
| handlers/tests/types | 15 | int/float/bool/string/char/void/tuple/option/fn/ref/mut_ref/ptr/name/enum/sum |
| rules/tests/sort_imports | 2 | classification function + complete sort verification |
| tests/source_map | 9 | single-line/multi-line/doc/nested comments, blank lines, offset conversion |
| tests/properties | 1 | **Idempotency property test** (proptest) |

---

## Code Quality Assessment

| Dimension | Score | Description |
|-----------|-------|-------------|
| Outstanding Items | 0 | — |
| Test Coverage | Good | 77 tests + 1 proptest, covers all Expr variants |
| Documentation Completeness | High | Source code annotations complete, design document detailed (18 rules + 4 principles) |
| Code Quality | Good | Module division clear, handler/rules/tests layering reasonable |

---

## Items to Improve (Sorted by Priority)

All specifications are implemented; no items to improve.