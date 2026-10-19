---
title: 'Release Template'
---

# Release Template

> The `/release` command generates the changelog following this template.  
> The changelog is **a human-readable description of changes**, not a commit list.

## Format Specification

```
:bookmark: V<version>: <title>
```

## Scope of CHANGELOG.md

`CHANGELOG.md` **keeps only the current version section** — after release, the file contains only
the current version's section.

- **During development**: maintain an "Unreleased (dev)" section at the top of the file,
  accumulating the change notes to be written for this release.
- **At release time**: the "Unreleased (dev)" section is rewritten as the current version section
  (title `## :bookmark: V<version>: <title>`), and **all other historical version sections in the
  file are deleted**.
- **Where to find historical versions**: The complete changelog for each version is published with
  its GitHub Release (the release page body = the changelog of that merge commit); the file content
  of any historical version can also be retrieved via `git show <tag>:CHANGELOG.md`. The current
  file in the repository therefore does not need to serve as a historical archive.

**Why this is defined this way**: The reader of this file cares about "what changed in the current
version"; cross-version traceability is handled by the Release page and git. The file does not grow
with each version, which avoids the unsolvable debate of "which sections to keep."

> Step 4 of the `/release` skill follows this section.

## 📦 Version Information

| Item           | Value                             |
| -------------- | --------------------------------- |
| Release Date   | YYYY-MM-DD                        |
| Version Change | `<old version>` → `<new version>` |
| Commit Count   | N commits                         |

## 📋 Release Summary

Use 2-3 sentences to summarize the core content and significance of this release.

## ✨ New Features

### `<Feature area title>`

`<A paragraph describing what this feature does, what problem it solves, and what it means for users>`

- Specific change 1
- Specific change 2
- Specific change 3

### `<Another feature area>`

`<Description>`

- Specific change

## 🐛 Bug Fixes

### `<Fix area>`

`<Describe what problem was fixed and the scope of impact>`

- Specific fix 1
- Specific fix 2

## ♻️ Refactoring & Optimization

### `<Refactoring direction>`

`<Describe why the refactoring was done and the benefits after refactoring>`

- Specific change

## Title Rules

Summarize the core change in one sentence, no more than 50 characters:

```
:bookmark: V0.7.2: REPL rewrite and type system improvements
:bookmark: V0.7.3: Type system fixes and ownership model improvements
:bookmark: V0.8.0: Concurrency model and generics system
```

## Classification Rules

Classify by the `type` prefix; omit empty categories:

|    type    |    Classification     |        Prefix        |
| :--------: | :-------------------: | :------------------: |
|   `feat`   |    ✨ New Features    |     `:sparkles:`     |
|   `fix`    |     🐛 Bug Fixes      |       `:bug:`        |
| `refactor` | ♻️ Refactoring & Opt. |     `:recycle:`      |
|   `perf`   |  ⚡ Performance Opt.  |       `:zap:`        |
|   `docs`   |   📝 Documentation    |       `:memo:`       |
|  `style`   |     🎨 Formatting     |       `:art:`        |
|   `test`   |      ✅ Testing       | `:white_check_mark:` |
|  `chore`   |    🔧 Build/Tools     |      `:wrench:`      |
|    `ci`    |       💚 CI/CD        |   `:green_heart:`    |

## Complete Example

`:bookmark: V0.7.3: Type system fixes and ownership model improvements`

```markdown
## 📦 Version Information

| Item           | Value             |
| -------------- | ----------------- |
| Release Date   | 2026-06-07        |
| Version Change | `0.7.2` → `0.7.3` |
| Commit Count   | 22 commits        |

## 📋 Release Summary

This release focuses on stability fixes for the type system and ownership model. Generic type
instantiation (e.g. `List(Int) = List(1, 2, 3)`) now works correctly, and several edge cases for
`&T` field access and tuple destructuring assignment have been fixed. Meanwhile, the unified
refactoring of ownership checks has been completed, laying the foundation for future move semantics
improvements.

## ✨ New Features

### Generic type instantiation

Supports the `List(Int) = List(1, 2, 3)` syntax, with the type constructor correctly registered as
Struct. Type inference no longer conflicts across multiple calls to a generic function; each call
gets an independent type variable instance.

- Type constructor registered as Struct, fixing the root cause of instantiation
- Independent inference across multiple calls to a generic function
- Interface method binding fix

### Re-binding after Move

After a variable is moved, it can be rebound to a new value; the priority lookup in assignment now
correctly handles the moved state, avoiding false "variable has been moved" errors.

- VarInfo adds a moved state flag
- Priority lookup in assignment adds a moved branch

### Resource marker trait

Added the `Resource` marker trait, marking types that implement IO side effects. Provides the
foundation for future side effect tracking and concurrency safety analysis.

### PLDI SRC demo

Completed all tasks of the PLDI SRC demo MVP, including end-to-end type checking and code generation
demos.

## 🐛 Bug Fixes

### &T field access

Fixed several issues in type checking for field access on immutable reference types, including field
assignment and inference when calling a constructor through a reference.

- `&T` field access type checking fix
- Field assignment target type inference
- Inference when calling a constructor through a reference

### LSP semantic highlighting

Fixed the missing semantic highlighting for variables inside `spawn {}` blocks and for tuple
destructuring assignment `(a, b) = ...`. `DestructureAssign.names` in the AST now carries the
position information for each variable name.

- Variables inside spawn blocks colored correctly
- Variable names in tuple destructuring assignment colored correctly
- Lays the foundation for future LSP support of move semantics

### freeze removal

Removed the deprecated `freeze` function and cleaned up related test cases.

## ♻️ Refactoring & Optimization

### Unified diagnostics system

Migrated the remaining 5 error enums to the unified `ErrorCodeDefinition` diagnostics system,
raising error code coverage from 60% to 80%. All compile errors now follow a unified format and
error code specification.

### Unified ownership check

Unified the ownership check logic previously scattered in multiple places into `OwnershipPass`, with
a single entry point handling borrow, move, and lifetime checks. Reduces maintenance cost and
improves error consistency.

### Concurrency model simplification

Removed the Send/Sync constraint system, refactored `spawn {}` into a grouped execution model.
Removed `@block`/`@eager`/`@auto` decorator-related code (RFC-024 phase 1). Simplified the
concurrency model, paving the way for future extensible concurrency primitive design.
```

## Workflow Overview

```
Collect commits → Generate changelog → Create PR → Wait for CI to go green → Bump version → Merge
```

See `.claude/commands/release.md` for details.
