# Commit Submission Guide

This document defines the Git commit conventions for the YaoXiang project, aiming to keep the commit
history clear, readable, and easy to understand.

---

## Table of Contents

- [Commit Format](#commit-format)
- [Commit Types](#commit-types)
- [Complete Emoji Reference](#complete-emoji-reference)
- [Scopes](#scopes)
- [Version Management](#version-management)
- [Message Conventions](#message-conventions)
- [Language Conventions](#language-conventions)
- [🔖 Release Commits](#-release-commits)
- [Examples](#examples)
- [Using Commit Template](#using-commit-template)
- [FAQ](#faq)

---

## Commit Format

**Very important!!!!!! Don't forget!!!** All commit messages must follow the format below:

```
:emoji_code: type(scope): subject (in Chinese)

[Optional body]

[Optional footer]
```

> ⚠️ **Important**: You **must** use the **emoji code** (e.g., `:sparkles:`) instead of typing the
> emoji character directly.
>
> **Commit messages in Chinese are recommended** to maintain team communication consistency.

### Components

| Part       | Description                                   | Required |
| ---------- | --------------------------------------------- | -------- |
| emoji_code | Emoji identifier for the commit type          | ✅       |
| type       | Commit type                                   | ✅       |
| scope      | Area of impact                                | ✅       |
| subject    | Brief description (in Chinese, ≤ 50 chars)    | ✅       |
| body       | Detailed explanation (optional)               | ❌       |
| footer     | Breaking changes or closing issues (optional) | ❌       |

---

## Commit Types

| emoji code              | type     | Description                            |
| ----------------------- | -------- | -------------------------------------- |
| :sparkles:              | feat     | New feature                            |
| :bug:                   | fix      | Bug fix                                |
| :memo:                  | docs     | Documentation only                     |
| :lipstick:              | style    | Code formatting (no functional change) |
| :recycle:               | refactor | Code refactoring                       |
| :zap:                   | perf     | Performance optimization               |
| :white_check_mark:      | test     | Add or modify tests                    |
| :wrench:                | chore    | Build tools, auxiliary tool changes    |
| :building_construction: | build    | Build system changes                   |
| :rocket:                | ci       | CI configuration changes               |

---

## Complete Emoji Reference

Below is the complete emoji list consistent with the gitmoji project. Pick the appropriate emoji
based on the commit content:

| Emoji | Emoji Code                    | Commit Description                      |
| :---- | :---------------------------- | :-------------------------------------- |
| 🎨    | `:art:`                       | Improve code structure/format           |
| ⚡️    | `:zap:` / `:racehorse:`       | Improve performance                     |
| 🔥    | `:fire:`                      | Remove code or files                    |
| 🐛    | `:bug:`                       | Fix a bug                               |
| 🚑    | `:ambulance:`                 | Critical hotfix                         |
| ✨    | `:sparkles:`                  | Introduce new features                  |
| 📝    | `:memo:`                      | Write documentation                     |
| 🚀    | `:rocket:`                    | Deploy stuff                            |
| 💄    | `:lipstick:`                  | Update UI and style files               |
| 🎉    | `:tada:`                      | Initial commit                          |
| ✅    | `:white_check_mark:`          | Add tests                               |
| 🔒    | `:lock:`                      | Fix security issues                     |
| 🍎    | `:apple:`                     | Fix macOS-specific issues               |
| 🐧    | `:penguin:`                   | Fix Linux-specific issues               |
| 🏁    | `:checkered_flag:`            | Fix Windows-specific issues             |
| 🤖    | `:robot:`                     | Fix Android-specific issues             |
| 🍏    | `:green_apple:`               | Fix iOS-specific issues                 |
| 🔖    | `:bookmark:`                  | Release/version tag                     |
| 🚨    | `:rotating_light:`            | Remove linter warnings                  |
| 🚧    | `:construction:`              | Work in progress                        |
| 💚    | `:green_heart:`               | Fix CI build issues                     |
| ⬇️    | `:arrow_down:`                | Downgrade dependencies                  |
| ⬆️    | `:arrow_up:`                  | Upgrade dependencies                    |
| 📌    | `:pushpin:`                   | Pin dependencies to a specific version  |
| 👷    | `:construction_worker:`       | Add CI build system                     |
| 📈    | `:chart_with_upwards_trend:`  | Add analytics or tracking code          |
| ♻️    | `:recycle:`                   | Refactor code                           |
| 🔨    | `:hammer:`                    | Major refactoring                       |
| ➖    | `:heavy_minus_sign:`          | Remove a dependency                     |
| 🐳    | `:whale:`                     | Docker related work                     |
| ➕    | `:heavy_plus_sign:`           | Add a dependency                        |
| 🔧    | `:wrench:`                    | Modify configuration files              |
| 🌐    | `:globe_with_meridians:`      | Internationalization and localization   |
| ✏️    | `:pencil2:`                   | Fix typos                               |
| 💩    | `:hankey:`                    | Write bad code that needs improvement   |
| ⏪️    | `:rewind:`                    | Revert changes                          |
| 🔀    | `:twisted_rightwards_arrows:` | Merge branches                          |
| 📦    | `:package:`                   | Update compiled files or packages       |
| 👽    | `:alien:`                     | Update code due to external API changes |
| 🚚    | `:truck:`                     | Move or rename files                    |
| 📄    | `:page_facing_up:`            | Add or update license                   |
| 💥    | `:boom:`                      | Introduce breaking changes              |
| 🍱    | `:bento:`                     | Add or update assets                    |
| 👌    | `:ok_hand:`                   | Update code due to code review changes  |
| ♿️    | `:wheelchair:`                | Improve accessibility                   |
| 💡    | `:bulb:`                      | Document source code                    |
| 🍻    | `:beers:`                     | Write code under the influence          |
| 💬    | `:speech_balloon:`            | Update text and literals                |
| 🗃️    | `:card_file_box:`             | Perform database-related changes        |
| 🔊    | `:loud_sound:`                | Add logs                                |
| 🔇    | `:mute:`                      | Remove logs                             |
| 👥    | `:busts_in_silhouette:`       | Add contributors                        |
| 🚸    | `:children_crossing:`         | Improve user experience/usability       |
| 🏗️    | `:building_construction:`     | Make architectural changes              |
| 📱    | `:iphone:`                    | Work on responsive design               |
| 🤡    | `:clown_face:`                | Mock things                             |
| 🥚    | `:egg:`                       | Add an easter egg                       |
| 🙈    | `:see_no_evil:`               | Add or update .gitignore files          |
| 📸    | `:camera_flash:`              | Add or update snapshots                 |

---

## Scopes

Scopes are based on the `src/` directory structure of the project. **You must use one of the scopes
defined below**:

### Top-Level Modules

| Scope       | Corresponding Directory | Description                                      |
| ----------- | ----------------------- | ------------------------------------------------ |
| `frontend`  | `src/frontend/`         | Frontend: lexing, parsing, type checking         |
| `middle`    | `src/middle/`           | Middle layer: IR, optimization, monomorphization |
| `backends`  | `src/backends/`         | Backend: interpreter, runtime, REPL              |
| `std`       | `src/std/`              | Standard library                                 |
| `formatter` | `src/formatter/`        | Code formatter                                   |
| `lsp`       | `src/lsp/`              | Language Server Protocol                         |
| `package`   | `src/package/`          | Package manager                                  |
| `util`      | `src/util/`             | Utility library: diagnostics, cache, i18n        |

### Frontend Submodules

| Scope       | Corresponding Directory        | Description            |
| ----------- | ------------------------------ | ---------------------- |
| `parser`    | `src/frontend/core/parser/`    | Parser                 |
| `lexer`     | `src/frontend/core/lexer/`     | Lexer                  |
| `typecheck` | `src/frontend/core/typecheck/` | Type checking          |
| `types`     | `src/frontend/core/types/`     | Type system definition |

### Middle Layer Submodules

| Scope          | Corresponding Directory           | Description                |
| -------------- | --------------------------------- | -------------------------- |
| `codegen`      | `src/middle/passes/codegen/`      | Code generation (bytecode) |
| `monomorphize` | `src/middle/passes/monomorphize/` | Monomorphization           |
| `lifetime`     | `src/middle/passes/lifetime/`     | Lifetime analysis          |

### Backend Submodules

| Scope     | Corresponding Directory     | Description                   |
| --------- | --------------------------- | ----------------------------- |
| `repl`    | `src/backends/dev/repl/`    | REPL interactive command line |
| `shell`   | `src/backends/dev/shell.rs` | Shell command handling        |
| `runtime` | `src/backends/runtime/`     | Runtime execution engine      |

### Documentation Scopes

| Scope    | Description                         |
| -------- | ----------------------------------- |
| `docs`   | General documentation updates       |
| `design` | Language design specification (RFC) |
| `plan`   | Implementation plan documents       |

### Other Scopes

| Scope     | Description                                            |
| --------- | ------------------------------------------------------ |
| `build`   | Build system, Cargo configuration                      |
| `ci`      | CI/CD configuration (GitHub Actions)                   |
| `test`    | Testing related                                        |
| `release` | Release related                                        |
| `meta`    | Project meta configuration (.claude, .gitignore, etc.) |

---

## Message Conventions

### Version Management

The version number is defined in the `version` field of `Cargo.toml` at the project root:

```toml
[package]
version = "0.7.2"
```

Semantic versioning `MAJOR.MINOR.PATCH` is used:

| Version Type | Description                            | Example       |
| ------------ | -------------------------------------- | ------------- |
| **major**    | Major update, incompatible API changes | 0.7.2 → 1.0.0 |
| **minor**    | New features, backward compatible      | 0.7.2 → 0.8.0 |
| **patch**    | Bug fix, backward compatible           | 0.7.2 → 0.7.3 |

> ⚠️ When releasing, **update the `Cargo.toml` version number on the `dev` branch**. After merging
> the PR to `main`, CI will automatically create the tag and Release. **Do not push tags manually**,
> otherwise CI will skip the release workflow.

---

## CI Release Workflow

Releases are performed automatically by GitHub Actions (`release.yml`). The workflow is as follows:

```
1. Update the version field in Cargo.toml on the dev branch
2. cargo build to update Cargo.lock
3. Commit following the release format (see 🔖 Release Commits below)
   - The commit message must include all changes since the last release (i.e., the full content of the PR)
4. Create a PR from dev to main
5. Merge the PR to main
6. CI automatically detects:
   - Reads Cargo.toml version number → "v{version}"
   - Checks whether the tag already exists
   - Not exists → Triggers the full release workflow
   - Exists → Skips (no duplicate release)
7. CI automatically executes:
   - In parallel: cross-platform builds (Linux/Windows/macOS) + security audit + tests
   - After all pass: create tag, package artifacts, publish GitHub Release
```

### Key Rules

| Rule                                       | Description                                                                                            |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| **Do not push tags manually**              | CI decides whether to release based on whether the tag exists; manual tags will cause CI to skip       |
| **Bump version on dev**                    | The release commit is completed on dev, then merged to main via PR                                     |
| **Release commit includes full changelog** | The commit message must contain all changes in this release, as it is the source of the PR description |
| **Do not merge main back to dev**          | dev will sync automatically after the PR is merged; no reverse merge is needed                         |

---

## Message Conventions

### Language Conventions

**Commit messages in Chinese are recommended** to maintain team communication consistency.

- Subject: Use Chinese, be concise and clear
- Body: Can use Chinese for detailed explanations
- Keep English for special technical terms if needed

### Subject

- Use Chinese, be concise and clear
- Keep within 50 characters
- No trailing period

### Body

- Explain why and how the changes were made
- Each line should not exceed 72 characters
- Use `-` or `*` for bullet points

### Footer

- **Breaking changes**: Start with `BREAKING CHANGE:`
- **Closing issues**: Use `Closes #123` or `Fixes #456`

---

## Examples

### ✨ feat - New feature

```
:sparkles: feat(parser): Add closure syntax parsing support

Implement closure expression parsing:
- Support |args| body shorthand syntax
- Support move semantics capture
- Add closure type inference

Closes #42
```

### 🐛 fix - Bug fix

```
:bug: fix(repl): Fix completer failing on multi-line input

SessionREPL did not register the completer correctly in multi-line mode,
causing Tab completion to fail to trigger.

Fixes #128
```

### 📝 docs - Documentation update

```
:memo: docs(design): Update ownership model and type system specification

Sync the latest design changes from RFC-009 and RFC-011.
```

### ♻️ refactor - Refactoring

```
:recycle: refactor(typecheck): Separate primitive value types from Dup shallow copy semantics

Decouple value types from copy semantics in MonoType,
eliminating special cases in match branches.
```

### ⚡️ perf - Performance optimization

```
:zap: perf(types): Optimize const generic evaluation performance

Add a depth limit for recursive evaluation (default 128),
to prevent stack overflow caused by maliciously constructed type expressions.
```

### ✅ test - Tests

```
:white_check_mark: test(typecheck): Add scope VarInfo mutability tests

Coverage scenarios:
- Read-only access to immutable bindings
- Mutability tracking for mut bindings
- Cross-scope mutability propagation
```

### 🔧 chore - Miscellaneous

```
:wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap

Upgrade 6 production dependencies to the latest stable versions.
```

### 🚀 ci - CI configuration

```
:rocket: ci: Fix nightly build Rust version being too low

Update RUST_TOOLCHAIN from 1.91.0 to 1.96.0,
to match the rust-version requirement in Cargo.toml.
```

### 💄 style - Formatting

```
:lipstick: style(frontend): Apply cargo fmt formatting

Unify the line-breaking style of function signatures.
```

---

---

## 🔖 Release Commits

When this commit is a **Release**, you **must** follow the conventions below:

### Release Commit Format

```
:bookmark: V<version>: <release title>

## 📦 Version Info

**Release Date:** YYYY-MM-DD

**Version:** <old version> → <new version>

---

## ✨ New Features

### <feature module>
- :sparkles: feat(<scope>): <feature description>

---

## ♻️ Refactoring

- :recycle: refactor(<scope>): <refactoring description>

---

## 🐛 Bug Fixes

- :bug: fix(<scope>): <fix description>

---

## 🔧 Other Changes

- :wrench: chore: <change description>

---

## 📦 New Files

- `<file path>` - <file description>

---



### Release Requirements

1. **Message header**: Must use `:bookmark:` + `V<version>` format
2. **Version number**: Follow semantic versioning
3. **Content completeness**: Must include the introduction of **all commits** since the last release
4. **Categorize by type**: Organize by `feat`, `fix`, `refactor`, `chore`, etc.

### Release Example

```

:bookmark: V0.7.2: REPL Rewrite and Type System Improvements

## 📦 Version Info

**Release Date:** 2026-06-01

**Version:** 0.7.1 → 0.7.2

---

## ✨ New Features

- :sparkles: feat(typecheck): Implement automatic inference for generic type parameters
- :sparkles: feat(typecheck): Add MonoType::Generic structured generic representation
- feat: Wire up CLI REPL commands to SessionREPL

---

## ♻️ Refactoring

- :recycle: refactor(backends): Remove tui_repl module, rewrite as SessionREPL
- :recycle: refactor(typecheck): Introduce VarInfo in scope variable storage to track mutability
- :recycle: refactor(typecheck): Separate primitive value types from Dup shallow copy semantics

---

## 🐛 Bug Fixes

- :bug: fix(repl): Configure default REPL history, fix shell evaluate_code
- :bug: fix(repl): Register completer and fix multi-line input
- :bug: fix(repl): Remove redundant semicolon in wrap_code to preserve expression value

---

## ⚡ Performance

- :zap: perf(types): Add recursion depth limit for const generic evaluation

---

## 🔧 Other Changes

- :wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap, owo-colors
- :white_check_mark: test(typecheck): Add scope VarInfo mutability tests

### Reference Template

For the release document, please refer to the [`release.md`](release.md) template format.

---

### 1. Set the Commit Template

```bash
# Run in the project root directory
git config commit.template .gitmessage.txt
```

### 2. Template File

The `.gitmessage.txt` file at the project root has the following format:

```
# emoji_code type(scope): subject (in Chinese)
#
# Body (optional)
#
# Footer (optional)
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: frontend, parser, lexer, typecheck, types, middle, codegen,
#         monomorphize, lifetime, backends, repl, shell, runtime,
#         std, formatter, lsp, package, util, docs, design, plan,
#         build, ci, test, release, meta
#
# Examples:
# ✨ feat(db): Add batch delete todo feature
# 🐛 fix(provider): Fix timer background recovery issue
#
# Release format: 🔖 V1.0.0: Release title
```

---

## FAQ

### Q: How do I choose a commit type?

- **feat**: User-visible feature changes
- **fix**: Fix issues reported by users
- **docs**: README, comments, etc.
- **chore**: Dependency updates, configuration files
- **refactor**: Code optimization without behavior changes

### Q: When should I split commits?

- Each commit should do **one thing**
- Group related features together, separate unrelated ones
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Complete emoji list
- [release.md](release.md) - Release template

---

> 💡 **Tip**: Keep commits atomic and descriptions clear to make code review and history navigation
> more efficient!
