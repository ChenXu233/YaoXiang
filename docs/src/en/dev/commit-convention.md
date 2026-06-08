# Commit Submission Guidelines

This document defines the Git commit conventions for the YaoXiang project, aiming to keep the commit history clear, readable, and easy to understand.

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

**Very important!!!!!!!! Don't forget!!!!!!**
All commit messages follow this format:

```
:emoji: type(scope): subject (Chinese)

[Optional body]

[Optional footer]
```

> ⚠️ **Important**: Must use **emoji codes** (e.g., `:sparkles:`) instead of directly typing emoji characters.
>
> **It is recommended to use Chinese for commit messages** to maintain consistency in team communication.

### Components

| Part | Description | Required |
|------|-------------|----------|
| emoji | Emoji code marking commit type | ✅ |
| type | Commit type | ✅ |
| scope | Affected scope | ✅ |
| subject | Brief description (Chinese, no more than 50 characters) | ✅ |
| body | Detailed explanation (optional) | ❌ |
| footer | Breaking changes or issue closing (optional) | ❌ |

---

## Commit Types

| emoji | type | Description |
|-------|------|-------------|
| :sparkles: | feat | New feature |
| :bug: | fix | Bug fix |
| :memo: | docs | Documentation changes only |
| :lipstick: | style | Code formatting (no functional change) |
| :recycle: | refactor | Code refactoring |
| :zap: | perf | Performance optimization |
| :white_check_mark: | test | Add or modify tests |
| :wrench: | chore | Build tools, auxiliary tool changes |
| :building_construction: | build | Build system changes |
| :rocket: | ci | CI configuration changes |

---

## Complete Emoji Reference

The following is a complete emoji list consistent with the gitmoji project. Select the appropriate emoji based on commit content:

| emoji | emoji code | commit description |
| :---- | :---------------------------- | :--------------------------- |
| 🎨 | `:art:` | Improve code structure/formatting |
| ⚡️ | `:zap:` / `:racehorse:` | Improve performance |
| 🔥 | `:fire:` | Remove code or files |
| 🐛 | `:bug:` | Fix a bug |
| 🚑 | `:ambulance:` | Critical patch |
| ✨ | `:sparkles:` | Introduce new features |
| 📝 | `:memo:` | Write docs |
| 🚀 | `:rocket:` | Deploy stuff |
| 💄 | `:lipstick:` | Update UI and style files |
| 🎉 | `:tada:` | Begin a project |
| ✅ | `:white_check_mark:` | Add tests |
| 🔒 | `:lock:` | Fix security issues |
| 🍎 | `:apple:` | Fix something on macOS |
| 🐧 | `:penguin:` | Fix something on Linux |
| 🏁 | `:checkered_flag:` | Fix something on Windows |
| 🤖 | `:robot:` | Fix something on Android |
| 🍏 | `:green_apple:` | Fix something on iOS |
| 🔖 | `:bookmark:` | Release/Version tag |
| 🚨 | `:rotating_light:` | Remove linter warnings |
| 🚧 | `:construction:` | Work in progress |
| 💚 | `:green_heart:` | Fix CI build |
| ⬇️ | `:arrow_down:` | Downgrade dependencies |
| ⬆️ | `:arrow_up:` | Upgrade dependencies |
| 📌 | `:pushpin:` | Pin dependencies to specific versions |
| 👷 | `:construction_worker:` | Add or update CI build system |
| 📈 | `:chart_with_upwards_trend:` | Add analytics or tracking code |
| ♻️ | `:recycle:` | Refactor code |
| 🔨 | `:hammer:` | Major refactoring |
| ➖ | `:heavy_minus_sign:` | Remove a dependency |
| 🐳 | `:whale:` | Docker related work |
| ➕ | `:heavy_plus_sign:` | Add a dependency |
| 🔧 | `:wrench:` | Modify configuration files |
| 🌐 | `:globe_with_meridians:` | Internationalization and localization |
| ✏️ | `:pencil2:` | Fix typos |
| 💩 | `:hankey:` | Write bad code that needs to be improved |
| ⏪️ | `:rewind:` | Revert changes |
| 🔀 | `:twisted_rightwards_arrows:` | Merge branches |
| 📦 | `:package:` | Update compiled files or packages |
| 👽 | `:alien:` | Update code due to external API changes |
| 🚚 | `:truck:` | Move or rename files |
| 📄 | `:page_facing_up:` | Add or update license |
| 💥 | `:boom:` | Introduce breaking changes |
| 🍱 | `:bento:` | Add or update assets |
| 👌 | `:ok_hand:` | Update code due to code review changes |
| ♿️ | `:wheelchair:` | Improve accessibility |
| 💡 | `:bulb:` | Document source code |
| 🍻 | `:beers:` | Write code drunkenly |
| 💬 | `:speech_balloon:` | Update text and literals |
| 🗃️ | `:card_file_box:` | Perform database related changes |
| 🔊 | `:loud_sound:` | Add logging |
| 🔇 | `:mute:` | Remove logging |
| 👥 | `:busts_in_silhouette:` | Add contributors |
| 🚸 | `:children_crossing:` | Improve user experience/usability |
| 🏗️ | `:building_construction:` | Make architectural changes |
| 📱 | `:iphone:` | Work on responsive design |
| 🤡 | `:clown_face:` | Mock things |
| 🥚 | `:egg:` | Add an easter egg |
| 🙈 | `:see_no_evil:` | Add or update .gitignore files |
| 📸 | `:camera_flash:` | Add or update snapshots |

---

## Scopes

Scopes are based on the project `src/` directory structure. **You must use the following defined scopes**:

### Top-level Modules

| Scope | Corresponding Directory | Description |
|-------|-------------------------|-------------|
| `frontend` | `src/frontend/` | Frontend: lexer, parser, type checker |
| `middle` | `src/middle/` | Middle layer: IR, optimization, monomorphization |
| `backends` | `src/backends/` | Backends: interpreter, runtime, REPL |
| `std` | `src/std/` | Standard library |
| `formatter` | `src/formatter/` | Code formatter |
| `lsp` | `src/lsp/` | Language Server Protocol |
| `package` | `src/package/` | Package manager |
| `util` | `src/util/` | Utilities: diagnostics, caching, i18n |

### Frontend Submodules

| Scope | Corresponding Directory | Description |
|-------|-------------------------|-------------|
| `parser` | `src/frontend/core/parser/` | Parser |
| `lexer` | `src/frontend/core/lexer/` | Lexer |
| `typecheck` | `src/frontend/core/typecheck/` | Type checking |
| `types` | `src/frontend/core/types/` | Type system definitions |

### Middle Layer Submodules

| Scope | Corresponding Directory | Description |
|-------|-------------------------|-------------|
| `codegen` | `src/middle/passes/codegen/` | Code generation (bytecode) |
| `monomorphize` | `src/middle/passes/monomorphize/` | Monomorphization |
| `lifetime` | `src/middle/passes/lifetime/` | Lifetime analysis |

### Backend Submodules

| Scope | Corresponding Directory | Description |
|-------|-------------------------|-------------|
| `repl` | `src/backends/dev/repl/` | REPL interactive command line |
| `shell` | `src/backends/dev/shell.rs` | Shell command handling |
| `runtime` | `src/backends/runtime/` | Runtime execution engine |

### Documentation Scopes

| Scope | Description |
|-------|-------------|
| `docs` | General documentation updates |
| `design` | Language design specifications (RFC) |
| `plan` | Implementation plan documents |

### Other Scopes

| Scope | Description |
|-------|-------------|
| `build` | Build system, Cargo configuration |
| `ci` | CI/CD configuration (GitHub Actions) |
| `test` | Test related |
| `release` | Release related |
| `meta` | Project meta configuration (.claude, .gitignore, etc.) |

---

## Message Conventions

### Version Management

Version numbers are defined in the `version` field of `Cargo.toml` in the project root:

```toml
[package]
version = "0.7.2"
```

Semantic versioning `MAJOR.MINOR.PATCH` is used:

| Version Type | Description | Example |
|--------------|-------------|---------|
| **major** | Major update, breaking API changes | 0.7.2 → 1.0.0 |
| **minor** | New features, backward compatible | 0.7.2 → 0.8.0 |
| **patch** | Bug fixes, backward compatible | 0.7.2 → 0.7.3 |

> ⚠️ When releasing, **update the `Cargo.toml` version number on the dev branch**. After merging the PR to main, CI will automatically create the tag and Release. **Do not manually push tags**, otherwise CI will skip the release process.

---

## CI Release Process

Releases are automatically handled by GitHub Actions (`release.yml`) with the following flow:

```
1. Update the version field in Cargo.toml on the dev branch
2. cargo build to update Cargo.lock
3. Commit following the release format (see 🔖 Release Commits below)
   - Commit message must include all changes since the last release (i.e., full PR content)
4. Create a PR from dev to main
5. Merge PR to main
6. CI automatically detects:
   - Read version from Cargo.toml → "v{version}"
   - Check if the tag already exists
   - Does not exist → trigger full release process
   - Already exists → skip (no duplicate release)
7. CI automatically executes:
   - Parallel: cross-platform builds (Linux/Windows/macOS) + security audit + tests
   - After all pass: create tag, package artifacts, publish GitHub Release
```

### Key Rules

| Rule | Description |
|------|-------------|
| **Do not manually push tags** | CI decides whether to release based on tag existence; manual tag push will cause CI to skip |
| **Bump version on dev** | Release commit is done on dev, merged to main via PR |
| **Release commit contains full changelog** | Commit message must include all changes for this release, as it serves as the PR description source |
| **Do not merge main back to dev** | After PR merges, dev will sync automatically; no reverse merge needed |

---

## Message Conventions

### Language Conventions

**It is recommended to use Chinese for commit messages** to maintain consistency in team communication.

- Subject in Chinese, concise and clear
- Body can be in Chinese with detailed explanation
- Technical terms may be kept in English if necessary

### Subject

- In Chinese, concise and clear
- No more than 50 characters
- No period at the end

### Body

- Detailed explanation of why and how changes were made
- Each line no more than 72 characters
- Use - or * for bullet points

### Footer

- **Breaking changes**: Start with `BREAKING CHANGE:`
- **Closing issues**: Use `关闭 #123` or `修复 #456`

---

## Examples

### ✨ feat - New Feature

```
:sparkles: feat(parser): Add closure syntax parsing support

Implement closure expression parsing:
- Support |args| body shorthand syntax
- Support move semantic capture
- Add closure type inference

Closes #42
```

### 🐛 fix - Bug Fix

```
:bug: fix(repl): Fix completer failure on multi-line input

SessionREPL does not correctly register the completer
in multi-line mode, causing Tab completion to fail.

Fixes #128
```

### 📝 docs - Documentation Update

```
:memo: docs(design): Update ownership model and type system specification

Synchronize latest design changes from RFC-009 and RFC-011.
```

### ♻️ refactor - Refactoring

```
:recycle: refactor(typecheck): Separate primitive value types from Dup shallow copy semantics

Decouple value types and copy semantics in MonoType,
eliminating special cases in match branches.
```

### ⚡️ perf - Performance Optimization

```
:zap: perf(types): Optimize const generic evaluation performance

Add depth limit (default 128) for recursive evaluation,
preventing stack overflow from maliciously constructed type expressions.
```

### ✅ test - Testing

```
:white_check_mark: test(typecheck): Add scope VarInfo mutability tests

Cover scenarios:
- Read-only access to immutable bindings
- Mutability tracking for mut bindings
- Mutability propagation across scopes
```

### 🔧 chore - Chores

```
:wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap

Upgrade 6 production dependencies to latest stable versions.
```

### 🚀 ci - CI Configuration

```
:rocket: ci: Fix nightly build Rust version too low

Update RUST_TOOLCHAIN from 1.91.0 to 1.96.0,
matching the rust-version requirement in Cargo.toml.
```

### 💄 style - Formatting

```
:lipstick: style(frontend): Apply cargo fmt formatting

Unify function signature line-breaking style.
```

---

---

## 🔖 Release Commits

When the commit is a **Release**, it must follow these conventions:

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

## ♻️ Refactoring & Optimization

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

## 📝 Commit History

| Commit | Description |
|:---:|------|
| `<hash>` | :bookmark: V<version> |
| `<hash>` | <commit message> |
```

### Release Requirements

1. **Message header**: Must use `:bookmark:` + `V<version>` format
2. **Version number**: Follow semantic versioning
3. **Content completeness**: Must include **all commit** content introductions since the last release
4. **Categorized by type**: Organize by `feat`, `fix`, `refactor`, `chore`, etc.
5. **Commit history**: List hashes and descriptions of all related commits

### Release Example

```
:bookmark: V0.7.2: REPL rewrite and type system improvements

## 📦 Version Info

**Release Date:** 2026-06-01

**Version:** 0.7.1 → 0.7.2

---

## ✨ New Features

- :sparkles: feat(typecheck): Implement automatic generic type parameter inference
- :sparkles: feat(typecheck): Add MonoType::Generic structured generic representation
- :sparkles: feat(repl): Integrate CLI REPL commands into SessionREPL

---

## ♻️ Refactoring & Optimization

- :recycle: refactor(backends): Remove tui_repl module, rewrite as SessionREPL
- :recycle: refactor(typecheck): Introduce VarInfo tracking for scope variable mutability
- :recycle: refactor(typecheck): Separate primitive value types from Dup shallow copy semantics

---

## 🐛 Bug Fixes

- :bug: fix(repl): Configure default REPL history, fix shell evaluate_code
- :bug: fix(repl): Register completer and fix multi-line input
- :bug: fix(repl): Remove extra semicolon in wrap_code to preserve expression value

---

## ⚡ Performance Optimization

- :zap: perf(types): Add recursive depth limit for const generic evaluation

---

## 🔧 Other Changes

- :wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap, owo-colors
- :white_check_mark: test(typecheck): Add scope VarInfo mutability tests

---

## 📝 Commit History

| Commit | Description |
|:---:|------|
| `f438aab` | :sparkles: feat(typecheck): Implement automatic generic type parameter inference |
| `bf0c121` | :zap: perf(types): Recursive depth limit |
| `6edac15` | :sparkles: feat(repl): Integrate CLI REPL into SessionREPL |
| `02cf54f` | :sparkles: feat(typecheck): MonoType::Generic |
| `3160a28` | :recycle: refactor(typecheck): VarInfo tracking for mutability |
| `f00a2a4` | :recycle: refactor(backends): Remove tui_repl module |
| `afe3e0c` | :bug: fix(repl): REPL history and shell fixes |
| `c4d2242` | :wrench: chore(build): Dependency bump |
```

### How to Get Commit History

```bash
# View all commits since last release
git log --oneline <last release commit>..HEAD

# Or view the last N commits
git log --oneline -20
```

### Reference Template

For release documentation, please refer to the [`release.md`](release.md) template format.

---

## Using Commit Template

### 1. Set Commit Template

```bash
# Execute in project root
git config commit.template .gitmessage.txt
```

### 2. Template File

The format of the `.gitmessage.txt` file in the project root:

```
# :emoji: type(scope): subject (Chinese)
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

### Q: How to choose a commit type?

- **feat**: Changes users can see
- **fix**: Fix issues reported by users
- **docs**: README, comments, etc.
- **chore**: Dependency updates, configuration files
- **refactor**: Code optimization without behavior changes

### Q: When should I split commits?

- Each commit should do **one thing**
- Related features together, unrelated separately
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Complete Emoji List
- [release.md](release.md) - Release Template

---

> 💡 **Tip**: Keep commits atomic and descriptions clear, making code review and tracing more efficient!