---
title: 'Release Template'
---

# Release Template

> The `/release` command generates changelog entries based on this template.  
> A changelog is a **human-readable description of changes**, not a commit list.

## Format Specification

```
:bookmark: V<version>: <title>
```

## 📦 Version Information

| Item           | Value                             |
| -------------- | --------------------------------- |
| Release Date   | YYYY-MM-DD                        |
| Version Change | `<old version>` → `<new version>` |
| Commits        | N commits                         |

## 📋 Release Summary

Summarize the core content and significance of this release in 2-3 sentences.

## ✨ New Features

### `<feature area title>`

`<a paragraph explaining what this feature does, what problem it solves, and what it means for users>`

- Specific change 1
- Specific change 2
- Specific change 3

### `<another feature area>`

`<description>`

- Specific change

## 🐛 Bug Fixes

### `<fix area>`

`<description of what was fixed and the scope of impact>`

- Specific fix 1
- Specific fix 2

## ♻️ Refactoring & Optimization

### `<refactoring direction>`

`<explanation of why the refactoring was done and the benefits>`

- Specific change

## Title Rules

Summarize the core change in one sentence, no more than 50 characters:

```
:bookmark: V0.7.2: REPL Rewrite and Type System Improvements
:bookmark: V0.7.3: Type System Fixes and Ownership Model Improvements
:bookmark: V0.8.0: Concurrency Model and Generics System
```

## Categorization Rules

Categorize by `type` prefix; omit empty categories:

|    type    |           Category            |        Prefix        |
| :--------: | :---------------------------: | :------------------: |
|   `feat`   |        ✨ New Features        |     `:sparkles:`     |
|   `fix`    |         🐛 Bug Fixes          |       `:bug:`        |
| `refactor` | ♻️ Refactoring & Optimization |     `:recycle:`      |
|   `perf`   |        ⚡ Performance         |       `:zap:`        |
|   `docs`   |       📝 Documentation        |       `:memo:`       |
|  `style`   |         🎨 Formatting         |       `:art:`        |
|   `test`   |           ✅ Tests            | `:white_check_mark:` |
|  `chore`   |        🔧 Build/Tools         |      `:wrench:`      |
|    `ci`    |           💚 CI/CD            |   `:green_heart:`    |

## Complete Example

`:bookmark: V0.7.3: Type System Fixes and Ownership Model Improvements`

```markdown
## 📦 Version Information

| Item           | Value             |
| -------------- | ----------------- |
| Release Date   | 2026-06-07        |
| Version Change | `0.7.2` → `0.7.3` |
| Commits        | 22 commits        |

## 📋 Release Summary

This release focuses on stability fixes for the type system and ownership model. Generic type
instantiation (e.g. `List(Int) = List(1, 2, 3)`) now works correctly, and several edge cases for
`&T` field access and tuple destructuring assignment have been fixed. A unified refactoring of
ownership checks was also completed, laying the groundwork for subsequent move semantics
improvements.

## ✨ New Features

### Generic Type Instantiation

Support for the `List(Int) = List(1, 2, 3)` syntax, with type constructors correctly registered as
Struct. Type inference no longer conflicts across multiple calls to the same generic function—each
call now gets an independent type variable instance.

- Type constructors registered as Struct, fixing the root cause of instantiation
- Independent inference across multiple generic function calls
- Interface method binding fix

### Re-binding After Move

Variables can be re-bound to new values after being moved. The assignment priority lookup now
correctly handles the moved state, avoiding false "variable has been moved" errors.

- VarInfo adds moved state flag
- Assignment priority lookup adds moved branch

### Resource Marker Trait

Added the `Resource` marker trait, marking types that implement IO side effects. This provides a
foundation for subsequent side-effect tracking and concurrency safety analysis.

### PLDI SRC Demo

Completed all tasks for the PLDI SRC demo MVP, including end-to-end type checking and code
generation demonstrations.

## 🐛 Bug Fixes

### &T Field Access

Fixed several issues in type checking for field access on immutable reference types, including field
assignment and inference for constructors called through references.

- `&T` field access type checking fix
- Field assignment target type inference
- Inference for constructors called through references

### LSP Semantic Highlighting

Fixed missing semantic highlighting for variables inside `spawn {}` blocks and for tuple
destructuring assignments like `(a, b) = ...`. The `DestructureAssign.names` field in the AST now
carries position information for each variable name.

- Correct coloring for variables inside spawn blocks
- Correct coloring for variables in tuple destructuring assignments
- Lays the groundwork for LSP support of subsequent move semantics

### Removal of freeze

Removed the deprecated `freeze` function and cleaned up related test cases.

## ♻️ Refactoring & Optimization

### Unified Diagnostics System

Migrated the remaining 5 error enums to the unified `ErrorCodeDefinition` diagnostics system,
raising error code coverage from 60% to 80%. All compilation errors now follow a unified format and
error code specification.

### Unified Ownership Checks

Consolidated scattered ownership check logic into a single `OwnershipPass`, with one entry point
handling borrow, move, and lifetime checks. Reduces maintenance cost and improves error consistency.

### Concurrency Model Simplification

Removed the Send/Sync constraint system; `spawn {}` was refactored into a grouped execution model.
Removed code related to the `@block`/`@eager`/`@auto` decorators (RFC-024 phase 1). This simplifies
the concurrency model and paves the way for future extensible concurrency primitive designs.
```

## Process Overview

```
Collect commits → Generate changelog → Create PR → Wait for CI to pass → Bump version → Merge
```

See `.claude/commands/release.md` for details.
