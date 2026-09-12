---
title: 'Release Template'
---

# Release Template

> The `/release` command generates changelogs based on this template.  
> A changelog is a **human-readable summary of changes**, not a list of commits.

## Format Specification

```
:bookmark: V<version>: <title>
```

## 📦 Version Information

| Item           | Value                             |
| -------------- | --------------------------------- |
| Release Date   | YYYY-MM-DD                        |
| Version Change | `<old version>` → `<new version>` |
| Commit Count   | N commits                         |

## 📋 Release Summary

Summarize the core content and significance of this release in 2-3 sentences.

## ✨ New Features

### <Feature Area Title>

<A paragraph explaining what this feature does, what problem it solves, and what it means for users>

- Specific change 1
- Specific change 2
- Specific change 3

### <Another Feature Area>

<Description>

- Specific change

## 🐛 Bug Fixes

### <Fix Area>

<Explain what problem was fixed and the scope of impact>

- Specific fix 1
- Specific fix 2

## ♻️ Refactoring & Optimization

### <Refactoring Direction>

<Explain why the refactoring was done and what benefits it brings>

- Specific change

## Title Rules

Summarize the core change in one sentence, no more than 50 characters:

```
:bookmark: V0.7.2: REPL Rewrite and Type System Improvements
:bookmark: V0.7.3: Type System Fixes and Ownership Model Improvements
:bookmark: V0.8.0: Concurrency Model and Generics System
```

## Classification Rules

Categorize by `type` prefix; omit empty categories:

|    type    |     Category     |        Prefix        |
| :--------: | :--------------: | :------------------: |
|   `feat`   | ✨ New Features  |     `:sparkles:`     |
|   `fix`    |   🐛 Bug Fixes   |       `:bug:`        |
| `refactor` |  ♻️ Refactoring  |     `:recycle:`      |
|   `perf`   |  ⚡ Performance  |       `:zap:`        |
|   `docs`   | 📝 Documentation |       `:memo:`       |
|  `style`   |     🎨 Style     |       `:art:`        |
|   `test`   |    ✅ Testing    | `:white_check_mark:` |
|  `chore`   |  🔧 Build/Tools  |      `:wrench:`      |
|    `ci`    |     💚 CI/CD     |   `:green_heart:`    |

## Complete Example

`:bookmark: V0.7.3: Type System Fixes and Ownership Model Improvements`

```markdown
## 📦 Version Information

| Item           | Value             |
| -------------- | ----------------- |
| Release Date   | 2026-06-07        |
| Version Change | `0.7.2` → `0.7.3` |
| Commit Count   | 22 commits        |

## 📋 Release Summary

This release focuses on stability fixes for the type system and ownership model. Generic type
instantiation (e.g. `List(Int) = List(1, 2, 3)`) now works correctly, and several edge cases in `&T`
field access and tuple destructuring assignment have been fixed. Additionally, a unified refactoring
of ownership checks has been completed, laying the foundation for the subsequent refinement of move
semantics.

## ✨ New Features

### Generic Type Instantiation

Supports the `List(Int) = List(1, 2, 3)` syntax, with type constructors correctly registered as
Struct. When a generic function is called multiple times, type inference no longer conflicts — each
call receives an independent type variable instance.

- Type constructors registered as Struct, fixing the root cause of instantiation
- Independent inference across multiple calls of generic functions
- Interface method binding fixes

### Rebinding After Move

Variables can be rebound to a new value after being moved. The assignment-priority lookup now
correctly handles the moved state, avoiding false "variable has been moved" errors.

- VarInfo gains a moved state flag
- Assignment-priority lookup adds a moved branch

### Resource Marker Trait

Added the `Resource` marker trait to mark types that implement IO side effects. This provides a
foundation for subsequent side-effect tracking and concurrency safety analysis.

### PLDI SRC Demo

Completed all tasks of the PLDI SRC demo MVP, including end-to-end type checking and code generation
demonstrations.

## 🐛 Bug Fixes

### &T Field Access

Fixed several issues in type checking for field access on immutable reference types, including field
assignment and inference for constructors invoked through references.

- `&T` field access type checking fixes
- Field assignment target type inference
- Inference for constructors called through references

### LSP Semantic Highlighting

Fixed missing semantic highlighting for variables inside `spawn {}` blocks and for tuple
destructuring assignments like `(a, b) = ...`. The AST's `DestructureAssign.names` now carries
position information for each variable name.

- Variables inside spawn blocks are correctly colored
- Variable names in tuple destructuring assignments are correctly colored
- Lays the groundwork for future LSP support of move semantics

### freeze Removal

Removed the deprecated `freeze` function and cleaned up related test cases.

## ♻️ Refactoring & Optimization

### Unified Diagnostic System

Migrated the remaining 5 error enums to the unified `ErrorCodeDefinition` diagnostic system,
increasing error code coverage from 60% to 80%. All compilation errors now follow a unified format
and error code specification.

### Unified Ownership Check

Consolidated scattered ownership check logic into a unified `OwnershipPass`, with a single entry
point handling borrow, move, and lifetime checks. This reduces maintenance cost and improves error
consistency.

### Concurrency Model Simplification

Removed the Send/Sync constraint system; `spawn {}` was refactored into a grouped execution model.
Removed code related to the `@block`/`@eager`/`@auto` decorators (RFC-024 phase 1). This simplifies
the concurrency model and paves the way for future designs of extensible concurrency primitives.
```

## Process Overview

```
Collect commits → Generate changelog → Create PR → Wait for CI to pass → Bump version → Merge
```

See `.claude/commands/release.md` for details.
