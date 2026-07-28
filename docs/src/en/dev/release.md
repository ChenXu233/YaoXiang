---
title: 'Release Template'
---

# Release Template

> The `/release` command generates changelogs based on this template.  
> Changelogs are **change descriptions for human readers**, not commit lists.

## Format Specification

```
:bookmark: V<version>: <title>
```

## 📦 Version Information

| Item        | Value                   |
| ----------- | ----------------------- |
| Release Date | YYYY-MM-DD             |
| Version Change | `<old>` → `<new>`     |
| Commit Count | N commits              |

## 📋 Release Summary

Summarize the core content and significance of this release in 2-3 sentences.

## ✨ New Features

### <Feature Area Title>

<Explain what this feature does, what problem it solves, and what it means for users>

- Specific change 1
- Specific change 2
- Specific change 3

### <Another Feature Area>

<Description>

- Specific change

## 🐛 Bug Fixes

### <Fix Area>

<Explain what was fixed and the impact>

- Specific fix 1
- Specific fix 2

## ♻️ Refactoring & Optimization

### <Refactoring Direction>

<Explain why the refactoring was done and its benefits>

- Specific change

## 📝 Commit History

|   Hash    | Description           |
| :-------: | --------------------- |
| `abc1234` | feat(scope): description |
| `def5678` | fix(scope): description |

## Title Rules

One sentence summarizing the core change, no more than 50 characters:

```
:bookmark: V0.7.2: REPL Rewrite and Type System Improvements
:bookmark: V0.7.3: Type System Fixes and Ownership Model Improvements
:bookmark: V0.8.0: Concurrency Model and Generics System
```

## Classification Rules

Categorized by `type` prefix, empty categories are omitted:

|    type    |       Category        |          Prefix          |
| :--------: | :-------------------: | :----------------------: |
|   `feat`   |    ✨ New Features    |       `:sparkles:`       |
|   `fix`    |      🐛 Bug Fixes     |          `:bug:`         |
| `refactor` | Refactoring & Optim.  |       `:recycle:`        |
|   `perf`   |   ⚡ Performance      |          `:zap:`         |
|   `docs`   |         📝 Docs       |         `:memo:`         |
|  `style`   |         🎨 Formatting  |          `:art:`         |
|   `test`   |         ✅ Tests       | `:white_check_mark:`     |
|  `chore`   |   🔧 Build/Tooling    |        `:wrench:`        |
|    `ci`    |        💚 CI/CD       |     `:green_heart:`      |

## Getting Commit History

```bash
git log <latest-tag>..HEAD --oneline --no-merges
```

## Full Example

`:bookmark: V0.7.3: Type System Fixes and Ownership Model Improvements`

```markdown
## 📦 Version Information

| Item         | Value                |
| ------------ | -------------------- |
| Release Date | 2026-06-07           |
| Version Change | `0.7.2` → `0.7.3`  |
| Commit Count | 22 commits           |

## 📋 Release Summary

This release focuses on stability fixes for the type system and ownership model. Generic type instantiation (e.g.
`List(Int) = List(1, 2, 3)`) now works correctly, and multiple edge cases for `&T`
field access and tuple destructuring assignment have been fixed. The ownership check unification has also been completed, laying the foundation for future move semantics improvements.

## ✨ New Features

### Generic Type Instantiation

Supports `List(Int) = List(1, 2, 3)` syntax. Type constructors are correctly registered as Structs. Type inference no longer conflicts across multiple calls to generic functions, with each call getting an independent type variable instance.

- Type constructor registered as Struct, fixing root cause of instantiation
- Independent inference for multiple generic function calls
- Interface method binding fixes

### Re-binding After Move

Variables can be re-bound to new values after being moved. Assignment priority lookup now correctly handles moved state, avoiding false "variable already moved" errors.

- VarInfo added moved state marker
- Assignment priority lookup added moved branch

### Resource Marker Trait

Added `Resource` marker trait to mark types that implement IO side effects. Provides a foundation for future side effect tracking and concurrency safety analysis.

### PLDI SRC Demo

Completed all PLDI SRC demo MVP tasks, including end-to-end type checking and code generation demonstrations.

## 🐛 Bug Fixes

### &T Field Access

Fixed multiple issues with immutable reference type field access in type checking, including field assignment and inference when calling constructors through references.

- `&T` field access type checking fixed
- Field assignment target type inference
- Constructor inference for reference calls

### LSP Semantic Highlighting

Fixed missing semantic highlighting for variables inside `spawn {}` blocks and tuple destructuring assignment `(a, b) = ...`. `DestructureAssign.names` in the AST now carries position information for each variable name.

- Variables inside spawn blocks correctly colored
- Variable names in tuple destructuring correctly colored
- Laid foundation for future LSP support of move semantics

### freeze Removal

Removed the deprecated `freeze` function, cleaning up related test cases.

## ♻️ Refactoring & Optimization

### Unified Diagnostic System

Migrated the remaining 5 error enums to the unified `ErrorCodeDefinition` diagnostic system. Error code coverage increased from 60% to 80%. All compilation errors now follow unified formatting and error code conventions.

### Unified Ownership Check

Unified ownership checking logic scattered across multiple places into `OwnershipPass`. A single entry point handles borrow, move, and lifetime checks. Reduces maintenance costs and improves error consistency.

### Concurrency Model Simplification

Removed Send/Sync constraint system. Refactored `spawn {}` to a grouped execution model. Removed decorator-related code for `@block`/`@eager`/`@auto` (RFC-024 phase 1). Simplified concurrency model to pave the way for future extensible concurrency primitives.

## 📝 Commit History

|   Hash    | Description                                                       |
| :-------: | ----------------------------------------------------------------- |
| `7297c65` | feat: multiple generic function calls + yx_runner error test support |
| `57a9893` | feat: generic type instantiation — List(Int) = List(1, 2, 3)      |
| `ab8a133` | feat: root cause fix — type constructor registered as Struct      |
| `196daec` | feat: generic instantiation fix + interface method binding        |
| `148e7a2` | feat: completed all PLDI SRC demo MVP tasks                      |
| `9a5a1b3` | fix(lsp): fixed semantic highlighting for spawn blocks and tuple destructuring |
| `75489c4` | feat: fixed &T field access + constructor inference + removed freeze |
| `a5b6135` | feat: fixed &T field access + field assignment + tuple destructuring |
| `0e24fcd` | refactor(diagnostic): migrated remaining 5 error enums to unified diagnostic system |
| `c262ddc` | refactor(diagnostic): migrated error codes to unified diagnostic system |
| `e8869c2` | feat(middle): added Resource marker trait for IO side effect awareness |
| `a76cdd3` | refactor(lifetime): unified ownership check into OwnershipPass     |
| `c7af770` | feat(typecheck): assignment priority lookup added moved branch    |
| `bb83e12` | feat(typecheck): VarInfo added moved state marker                |
| `e4a44c4` | refactor(middle): removed Send/Sync constraint system             |
| `1d3fe2d` | refactor(frontend): removed @block/@eager/@auto related code      |
| `4fd4e0a` | feat(formatter): implemented missing formatting rules            |
| `13fbc21` | fix(backends): fixed syntax error in execute.rs                   |
```

## Process Overview

```
Collect commits → Generate changelog → Create PR → Wait for CI green → Bump version → Merge
```

See `.claude/commands/release.md` for details.
