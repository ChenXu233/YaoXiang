---
title: "Formatter State"
---

# Formatter

> **Module Status**: Stable (0 items pending improvement)
> **Location**: `src/formatter/`
> **Last Updated**: 2026-06-04

---

## Module Overview

The formatter is responsible for automatically formatting YaoXiang source code. It supports full AST node formatting, including expressions, statements, types, etc.

**Code Size**: 2,397 lines (19 source files)

---

## Feature List

### Implemented Features

**Expression Formatting** (handlers/expr.rs, covers all Expr variants):
- ✅ Literals: Int / Float / Bool / Char / String (with escaping)
- ✅ Variable references
- ✅ Binary operators / Unary operators (with automatic line break when line width exceeded)
- ✅ Function calls (with named parameters)
- ✅ Function definitions (expression form fn def)
- ✅ if/elif/else
- ✅ match expressions (pattern alignment, line break when too long)
- ✅ for / while loops (with labels)
- ✅ Code blocks (with comment preservation, end-of-line comment handling)
- ✅ return / break / continue
- ✅ Type casting (as)
- ✅ Tuples / Lists / List comprehensions / Dictionaries
- ✅ Indexing / Field access (chained calls line break)
- ✅ try operator (?)
- ✅ ref / borrow (& / &mut)
- ✅ unsafe blocks
- ✅ eval blocks (@block / @auto / @eager)
- ✅ spawn blocks
- ✅ lambda expressions (single-expression concise form)
- ✅ f-string (with format specifications)
- ✅ Error nodes (insert `/* error */` placeholder)

**Statement Formatting** (handlers/stmt.rs):
- ✅ Variable declarations (mut / type annotations / initializers)
- ✅ for loop statements
- ✅ Unified binding statements (function / type / method bindings)
- ✅ use import statements (with items, alias)
- ✅ if statements
- ✅ External binding statements (External / Anonymous / DefaultExternal)

**Type Formatting** (handlers/types.rs, covers all Type variants):
- ✅ Basic types: Int(size) / Float(size) / Char / String / Bytes / Bool / Void
- ✅ Named types / Structs / Named structs
- ✅ Union / Enum / Variant
- ✅ Tuples / Function types / Option / Result
- ✅ Generics / Associated types / Sum types
- ✅ Literal types / Reference types / Pointer types / MetaType

**Other Features**:
- ✅ Delimited list automatic line breaking (handlers/delimited.rs)
- ✅ Comment preservation (source_map.rs)
- ✅ Import sorting (rules/sort_imports.rs)
- ✅ CLI commands: check / write / stdout mode (command.rs)
- ✅ Configuration options: line_width / indent_width / use_tabs / single_quote / sort_imports (options.rs)

---

## Incompletely Implemented Specifications

| Specification | Status | Difference Description |
|--------------|--------|------------------------|
| §2.2 Line Break Strategy Priority | ✅ Implemented | Full priority chain implemented |
| §4.2 Parameter List Line Break (Trailing Comma) | ✅ Implemented | Parameter list automatically breaks when exceeding line width, using trailing commas |
| §6.2 Single-Line Code Blocks | ✅ Implemented | Single-statement code blocks within line width use single-line format |
| §6.3 Empty Code Blocks | ✅ Implemented | Empty code blocks output `{}` instead of `{\n}` |
| §8.3 Single Quote Mode | ✅ Implemented | `single_quote` config takes effect |
| §E2 Exit Codes | ✅ Implemented | Specific exit codes used as specified |

---

## Test Coverage

**77 tests + 1 proptest idempotency test**:

| Test Group | Count | Coverage |
|------------|-------|----------|
| handlers/tests/expr | 50 | Literals, binary operations, function calls, lists, dictionaries, return, cast, match, f-string, try, unsafe, syntax error tolerance, single quote mode, empty code blocks, single-line code blocks, parameter list line breaking, **variable references, return/break/continue, tuples, index access, field access, ref, error recovery** |
| handlers/tests/types | 15 | int/float/bool/string/char/void/tuple/option/fn/ref/mut_ref/ptr/name/enum/sum |
| rules/tests/sort_imports | 2 | Classification functions + complete sorting verification |
| tests/source_map | 9 | Single-line/multi-line/document/nested comments, blank lines, offset conversion |
| tests/properties | 1 | **Idempotency property test** (proptest) |

---

## Code Quality Assessment

| Dimension | Score | Description |
|-----------|-------|-------------|
| Unfinished Items | 0 | — |
| Test Coverage | Good | 77 tests + 1 proptest, covers all Expr variants |
| Documentation Completeness | High | Source comments complete, design documentation detailed (18 rules + 4 principles) |
| Code Quality | Good | Clear module division, reasonable handler/rules/tests layering |

---

## Items Pending Improvement (Sorted by Priority)

All specifications have been implemented, no items pending improvement.