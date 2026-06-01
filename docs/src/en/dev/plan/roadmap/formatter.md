---
title: "Formatter State"
---

# Formatter

> **Module State**: Basic completion
> **Location**: `src/formatter/`
> **Last Updated**: 2026-06-01

---

## Module Overview

The formatter is responsible for automatically formatting YaoXiang source code. It supports full AST node formatting, including expressions, statements, types, etc.

**Code Size**: 2,397 lines (19 source files)

---

## Feature List

### Implemented Features

**Expression Formatting** (handlers/expr.rs, covering all Expr variants):
- ✅ Literals: Int / Float / Bool / Char / String (with escape sequences)
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
- ✅ lambda expressions (single expression concise form)
- ✅ f-strings (with format specifications)
- ✅ Error nodes (insert `/* error */` placeholder)

**Statement Formatting** (handlers/stmt.rs):
- ✅ Variable declarations (mut / type annotations / initializers)
- ✅ for loop statements
- ✅ Unified binding statements (function / type / method bindings)
- ✅ use import statements (with items, alias)
- ✅ if statements
- ✅ External binding statements (External / Anonymous / DefaultExternal)

**Type Formatting** (handlers/types.rs, covering all Type variants):
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
- ✅ CLI commands: check / write / stdout modes (command.rs)
- ✅ Configuration options: line_width / indent_width / use_tabs / single_quote / sort_imports (options.rs)

---

## Incompletely Implemented Specifications

| Specification | Status | Difference Description |
|------|------|----------|
| §2.2 Line Break Strategy Priority | ⚠️ Partial implementation | Only binop line breaking implemented, full priority chain not implemented |
| §4.2 Parameter List Line Break (trailing comma) | ⚠️ Partial implementation | Parameter list formatting always outputs single line, no automatic line break |
| §6.2 Single-line Code Block | ❌ Not implemented | Always outputs multi-line format `{ stmt }` |
| §6.3 Empty Code Block | ⚠️ Not fully implemented | Empty blocks output `{\n}` (two lines), instead of `{}` |
| §8.3 Single Quote Mode | ❌ Not implemented | `single_quote` field exists but not read |
| §E2 Exit Codes | ❌ Not implemented | Specific exit codes not used per specification |

---

## Test Coverage

**51 tests + 1 proptest idempotency test**:

| Test Group | Count | Coverage |
|--------|------|----------|
| handlers/tests/expr | 24 | Literals, binary operations, function calls, lists, dictionaries, return, cast, match, f-strings, try, unsafe, syntax error tolerance |
| handlers/tests/types | 15 | int/float/bool/string/char/void/tuple/option/fn/ref/mut_ref/ptr/name/enum/sum |
| rules/tests/sort_imports | 2 | Classification function + complete sorting validation |
| tests/source_map | 9 | Single-line/multi-line/document/nested comments, blank lines, offset conversion |
| tests/properties | 1 | **Idempotency property test** (proptest) |

---

## Code Quality Assessment

| Dimension | Score | Description |
|------|------|----------|
| Feature Completeness | 90% | All AST nodes have formatting logic, 6 specifications not fully implemented |
| Test Coverage | Medium | 51 tests + 1 proptest, missing end-to-end integration tests |
| Documentation Completeness | High | Source code comments complete, design documentation detailed (18 rules + 4 principles) |
| Code Quality | Good | Clear module separation, reasonable layering of handlers/rules/tests |

---

## Items to Improve (Sorted by Priority)

1. **Empty code blocks output `{}` instead of `{\n}`** (§6.3)
2. **`single_quote` configuration takes effect** (§8.3)
3. **Parameter list supports automatic line breaking** (§4.2)
4. **Single-statement code blocks compress to single line** (§6.2)
5. **Complete line break strategy priority chain implementation** (§2.2)
6. **CLI exit codes implemented per specification** (§E2)