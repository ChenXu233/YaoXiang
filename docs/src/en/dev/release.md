---
title: "Release Template"
---

# Release Template

> The `/release` command generates changelogs based on this template.  
> A changelog is a **human-readable description of changes**, not a commit list.

## Format Specification

```
:bookmark: V<version>: <title>
```

## 📦 Version Information

| Item           | Value                    |
| -------------- | ------------------------ |
| Release Date   | YYYY-MM-DD               |
| Version Change | `<old version>` → `<new version>` |
| Commit Count   | N commits                |

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

<Describe what problem was fixed and the scope of impact>

- Specific fix 1
- Specific fix 2

## ♻️ Refactoring & Optimization

### <Refactoring Direction>

<Explain why the refactoring was done and what benefits it brings>

- Specific change

## 📝 Commit Records

|   Hash    | Description           |
| :-------: | --------------------- |
| `abc1234` | feat(scope): description |
| `def5678` | fix(scope): description  |

## Title Rules

Summarize the core change in one sentence, no more than 50 characters:

```
:bookmark: V0.7.2: REPL Rewrite and Type System Improvements
:bookmark: V0.7.3: Type System Fixes and Ownership Model Improvements
:bookmark: V0.8.0: Concurrency Model and Generics System
```

## Category Rules

Categorize by `type` prefix; omit empty categories:
| type | Category | Prefix |
|:---:|:---:|:---:|
| `feat` | ✨ New Features | `:sparkles:` |
| `fix` | 🐛 Bug Fixes | `:bug:` |
| `refactor` | ♻️ Refactoring | `:recycle:` |
| `perf` | ⚡ Performance | `:zap:` |
| `docs` | 📝 Documentation | `:memo:` |
| `style` | 🎨 Style | `:art:` |
| `test` | ✅ Tests | `:white_check_mark:` |
| `chore` | 🔧 Build/Tools | `:wrench:` |
| `ci` | 💚 CI/CD | `:green_heart:` |

## Getting Commit Records

```bash
git log <latest tag>..HEAD --oneline --no-merges
```

## Complete Example

`:bookmark: V0.7.3: Type System Fixes and Ownership Model Improvements`

```markdown
## 📦 Version Information

| Item           | Value                |
| -------------- | -------------------- |
| Release Date   | 2026-06-07           |
| Version Change | `0.7.2` → `0.7.3`    |
| Commit Count   | 22 commits           |

## 📋 Release Summary

This release focuses on stability fixes for the type system and ownership
model. Generic type instantiation (e.g. `List(Int) = List(1, 2, 3)`) now
works correctly, and several edge cases in `&T` field access and tuple
destructuring assignment have been fixed. Additionally, a unified refactoring
of ownership checks has been completed, laying the foundation for further
move semantics improvements.

## ✨ New Features

### Generic Type Instantiation

Supports the `List(Int) = List(1, 2, 3)` syntax, with type constructors
correctly registered as Struct. Type inference no longer conflicts when a
generic function is called multiple times — each call gets an independent
type variable instance.

- Type constructor registered as Struct, fixing the root cause of instantiation
- Independent type inference for multiple calls of generic functions
- Interface method binding fix

### Re-binding After Move

Variables can be re-bound with new values after being moved. Assignment
priority lookup now correctly handles the moved state, avoiding false
"variable has been moved" errors.

- VarInfo gains a moved state marker
- Assignment priority lookup gains a moved branch

### Resource Marker Trait

Added the `Resource` marker trait to mark types that implement IO side
effects. This provides a foundation for future side-effect tracking and
concurrency safety analysis.

### PLDI SRC Demo

Completed all tasks for the PLDI SRC demo MVP, including end-to-end type
checking and code generation demonstrations.

## 🐛 Bug Fixes

### &T Field Access

Fixed several issues with field access on immutable reference types during
type checking, including field assignment and inference when calling
constructors through references.

- `&T` field access type checking fix
- Field assignment target type inference
- Constructor inference when called through a reference

### LSP Semantic Highlighting

Fixed the missing semantic highlighting for variables inside `spawn {}`
blocks and tuple destructuring assignment `(a, b) = ...`. `DestructureAssign.names`
in the AST now carries position information for each variable name.

- Variables inside spawn blocks are now correctly highlighted
- Variables in tuple destructuring assignment are now correctly highlighted
- Lays the groundwork for LSP support of upcoming move semantics

### freeze Removal

Removed the deprecated `freeze` function and cleaned up related test cases.

## ♻️ Refactoring & Optimization

### Unified Diagnostic System

Migrated the remaining 5 error enums to the unified `ErrorCodeDefinition`
diagnostic system, raising error code coverage from 60% to 80%. All compile
errors now follow a unified format and error code specification.

### Unified Ownership Check

Consolidated ownership check logic scattered across multiple locations into
a single `OwnershipPass`, with a single entry point handling borrow, move,
and lifetime checks. Reduces maintenance cost and improves error consistency.

### Concurrency Model Simplification

Removed the Send/Sync constraint system, and refactored `spawn {}` into a
grouped execution model. Removed code related to `@block`/`@eager`/`@auto`
decorators (RFC-024 phase 1). Simplifies the concurrency model and paves
the way for future extensible concurrency primitive design.

## 📝 Commit Records

|   Hash    | Description                                                       |
| :-------: | ----------------------------------------------------------------- |
| `7297c65` | feat: Generic function multiple calls + yx_runner error test support |
| `57a9893` | feat: Generic type instantiation — List(Int) = List(1, 2, 3)      |
| `ab8a133` | feat: Root cause fix — type constructor registered as Struct      |
| `196daec` | feat: Fix generic instantiation + interface method binding        |
| `148e7a2` | feat: Complete all PLDI SRC demo MVP tasks                        |
| `9a5a1b3` | fix(lsp): Fix semantic highlighting for spawn blocks and tuple destructuring |
| `75489c4` | feat: Fix &T field access + constructor inference + remove freeze |
| `a5b6135` | feat: Fix &T field access + field assignment + tuple destructuring |
| `0e24fcd` | refactor(diagnostic): Migrate remaining 5 error enums to unified diagnostic system |
| `c262ddc` | refactor(diagnostic): Migrate error codes to unified diagnostic system |
| `e8869c2` | feat(middle): Add Resource marker trait for IO side-effect awareness |
| `a76cdd3` | refactor(lifetime): Unify ownership checks into OwnershipPass      |
| `c7af770` | feat(typecheck): Add moved branch to assignment priority lookup   |
| `bb83e12` | feat(typecheck): Add moved state marker to VarInfo                |
| `e4a44c4` | refactor(middle): Remove Send/Sync constraint system              |
| `1d3fe2d` | refactor(frontend): Remove @block/@eager/@auto related code       |
| `4fd4e0a` | feat(formatter): Implement missing formatting rules                |
| `13fbc21` | fix(backends): Fix syntax errors in execute.rs                     |
```

## Process Overview

```
Collect commits → Generate changelog → Create PR → Wait for CI green → Bump version → Merge
```

See `.claude/commands/release.md` for details.