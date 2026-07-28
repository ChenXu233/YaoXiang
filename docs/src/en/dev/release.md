---
title: 'Release Template'
---

# Release Template

> The `/release` command generates the changelog following this template.  
> The changelog is **a change description written for humans**, not a commit list.

## Format Specification

```
:bookmark: V<version>: <title>
```

## 📦 Version Info

| Item           | Value                             |
| -------------- | --------------------------------- |
| Release Date   | YYYY-MM-DD                        |
| Version Change | `<old version>` → `<new version>` |
| Commit Count   | N commits                         |

## 📋 This Release Summary

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

<Description of what was fixed and the scope of impact>

- Specific fix 1
- Specific fix 2

## ♻️ Refactoring & Optimization

### <Refactoring Direction>

<Explanation of why the refactoring was done and the benefits after refactoring>

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
|   `perf`   |  ⚡ Performance Optimization  |       `:zap:`        |
|   `docs`   |       📝 Documentation        |       `:memo:`       |
|  `style`   |         🎨 Formatting         |       `:art:`        |
|   `test`   |          ✅ Testing           | `:white_check_mark:` |
|  `chore`   |       🔧 Build/Tooling        |      `:wrench:`      |
|    `ci`    |           💚 CI/CD            |   `:green_heart:`    |

## Get Commit Records

```bash
git log <latest tag>..HEAD --oneline --no-merges
```

## Complete Example

`:bookmark: V0.7.3: Type System Fixes and Ownership Model Improvements`

```markdown
## 📦 Version Info

| Item           | Value             |
| -------------- | ----------------- |
| Release Date   | 2026-06-07        |
| Version Change | `0.7.2` → `0.7.3` |
| Commit Count   | 22 commits        |

## 📋 This Release Summary

This release focuses on stability fixes for the type system and ownership model. Generic type
instantiation (e.g., `List(Int) = List(1, 2, 3)`) now works correctly, and multiple edge cases in
`&T` field access and tuple destructuring assignment have been fixed. At the same time, the unified
refactoring of ownership checks has been completed, laying the foundation for future improvements to
move semantics.

## ✨ New Features

### Generic Type Instantiation

Supports the `List(Int) = List(1, 2, 3)` syntax, with type constructors correctly registered as
Struct. Type inference no longer conflicts when generic functions are called multiple times; each
call gets an independent type variable instance.

- Type constructor registered as Struct, fixing the root cause of instantiation
- Independent inference for multiple calls of generic functions
- Interface method binding fix

### Rebinding After Move

After a variable is moved, it can be rebound to a new value. The assignment priority lookup now
correctly handles the moved state, avoiding false "variable already moved" errors.

- VarInfo adds a moved state flag
- Assignment priority lookup adds a moved branch

### Resource marker trait

Adds the `Resource` marker trait to mark types that implement IO side effects. Provides the
foundation for future side effect tracking and concurrency safety analysis.

### PLDI SRC demo

Completes all tasks for the PLDI SRC demo MVP, including end-to-end type checking and code
generation demos.

## 🐛 Bug Fixes

### &T Field Access

Fixes multiple issues with field access on immutable reference types in type checking, including
field assignment and inference for constructors called through references.

- Type checking fix for `&T` field access
- Target type inference for field assignment
- Inference for constructors called through references

### LSP Semantic Highlighting

Fixes missing semantic highlighting for variables inside `spawn {}` blocks and tuple destructuring
assignment `(a, b) = ...`. `DestructureAssign.names` in the AST now carries the position information
for each variable name.

- Variables in spawn blocks correctly tokenized
- Variable names in tuple destructuring assignment correctly tokenized
- Lays the foundation for future LSP support of move semantics

### freeze Removal

Removes the deprecated `freeze` function and cleans up related test cases.

## ♻️ Refactoring & Optimization

### Unified Diagnostic System

Migrates the remaining 5 error enums to the unified `ErrorCodeDefinition` diagnostic system, with
error code coverage increased from 60% to 80%. All compile errors now follow a unified format and
error code specification.

### Unified Ownership Check

Unifies the ownership check logic scattered across multiple places into `OwnershipPass`, with a
single entry point handling borrow, move, and lifetime checks. Reduces maintenance cost and improves
error consistency.

### Concurrency Model Simplification

Removes the Send/Sync constraint system, refactors `spawn {}` to a grouped execution model, and
removes code related to `@block`/`@eager`/`@auto` decorators (RFC-024 phase 1). Simplifies the
concurrency model, paving the way for future extensible concurrency primitive designs.
```

## Process Overview

```
Collect commits → Generate changelog → Create PR → Wait for CI to fully pass → Bump version → Merge
```

See `.claude/commands/release.md` for details
