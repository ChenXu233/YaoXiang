# Commit Submission Guidelines

This document defines the Git commit conventions for the YaoXiang project, aiming to keep the commit history clear, readable, and easy to understand.

---

## Table of Contents

- [Commit Format](#commit-format)
- [Commit Types](#commit-types)
- [Full Emoji Reference](#full-emoji-reference)
- [Scopes](#scopes)
- [Version Management](#version-management)
- [Message Guidelines](#message-guidelines)
- [Language Guidelines](#language-guidelines)
- [🔖 Release Commits](#-release-commits)
- [Examples](#examples)
- [Using Commit Template](#using-commit-template)
- [FAQ](#faq)

---

## Commit Format

**Very important!!!! Don't forget!!!!** All commit messages follow this format:

```
:emoji_code: type(scope): subject (in English)

[optional body]

[optional footer]
```

> ⚠️ **Important**: Must use **emoji codes** (like `:sparkles:`) instead of directly entering emoji characters.
>
> **It is recommended to use English for commit messages** to maintain consistency in team communication.

### Components

| Part     | Description                      | Required |
| -------- | -------------------------------- | -------- |
| emoji code | Emoji symbol marking commit type | ✅       |
| type     | Commit type                      | ✅       |
| scope    | Affected scope                   | ✅       |
| subject  | Brief description (English, max 50 chars) | ✅ |
| body     | Detailed explanation (optional)  | ❌       |
| footer   | Breaking changes or issue closure (optional) | ❌ |

---

## Commit Types

| emoji code               | type     | Description                        |
| ----------------------- | -------- | ----------------------------------- |
| :sparkles:              | feat     | New feature                         |
| :bug:                   | fix      | Bug fix                             |
| :memo:                  | docs     | Documentation changes only          |
| :lipstick:              | style    | Code formatting (no functional change) |
| :recycle:               | refactor | Code refactoring                    |
| :zap:                   | perf     | Performance improvement             |
| :white_check_mark:      | test     | Adding or modifying tests           |
| :wrench:                | chore    | Build tools, auxiliary tool changes |
| :building_construction: | build    | Build system changes                |
| :rocket:                | ci       | CI configuration changes            |

---

## Full Emoji Reference

The following is the complete emoji list consistent with the gitmoji project. Select the appropriate emoji based on commit content:

| emoji | emoji code                        | commit description                     |
| :---- | :-------------------------------- | :------------------------------------- |
| 🎨    | `:art:`                           | Improve code structure/code format    |
| ⚡️    | `:zap:` / `:racehorse:`           | Improve performance                   |
| 🔥    | `:fire:`                          | Remove code or files                  |
| 🐛    | `:bug:`                           | Fix a bug                             |
| 🚑    | `:ambulance:`                     | Critical patch                        |
| ✨    | `:sparkles:`                      | Introduce new features                |
| 📝    | `:memo:`                          | Write docs                            |
| 🚀    | `:rocket:`                        | Deploy stuff                          |
| 💄    | `:lipstick:`                      | Update UI and style files             |
| 🎉    | `:tada:`                          | Begin a project                       |
| ✅    | `:white_check_mark:`              | Add tests                             |
| 🔒    | `:lock:`                          | Fix security issue                    |
| 🍎    | `:apple:`                         | Fix something on macOS                |
| 🐧    | `:penguin:`                       | Fix something on Linux                |
| 🏁    | `:checkered_flag:`                | Fix something on Windows              |
| 🤖    | `:robot:`                          | Fix something on Android              |
| 🍏    | `:green_apple:`                   | Fix something on iOS                  |
| 🔖    | `:bookmark:`                      | Release/Version tag                   |
| 🚨    | `:rotating_light:`                | Remove linter warnings                |
| 🚧    | `:construction:`                  | Work in progress                      |
| 💚    | `:green_heart:`                   | Fix CI build                          |
| ⬇️    | `:arrow_down:`                    | Downgrade dependencies                |
| ⬆️    | `:arrow_up:`                      | Upgrade dependencies                  |
| 📌    | `:pushpin:`                       | Pin dependencies to specific versions |
| 👷    | `:construction_worker:`           | Add CI build system                   |
| 📈    | `:chart_with_upwards_trend:`      | Add analytics or tracking code        |
| ♻️    | `:recycle:`                       | Refactor code                         |
| 🔨    | `:hammer:`                        | Major refactoring                    |
| ➖    | `:heavy_minus_sign:`              | Remove a dependency                   |
| 🐳    | `:whale:`                         | Docker related work                   |
| ➕    | `:heavy_plus_sign:`               | Add a dependency                      |
| 🔧    | `:wrench:`                        | Change config files                   |
| 🌐    | `:globe_with_meridians:`          | Internationalization and localization |
| ✏️    | `:pencil2:`                       | Fix typo                              |
| 💩    | `:hankey:`                        | Write bad code that needs to be improved |
| ⏪️    | `:rewind:`                        | Revert changes                        |
| 🔀    | `:twisted_rightwards_arrows:`     | Merge branches                       |
| 📦    | `:package:`                       | Update compiled files or packages     |
| 👽    | `:alien:`                         | Update code due to external API changes |
| 🚚    | `:truck:`                         | Move or rename files                 |
| 📄    | `:page_facing_up:`                | Add or update license                 |
| 💥    | `:boom:`                          | Introduce breaking changes            |
| 🍱    | `:bento:`                         | Add or update assets                  |
| 👌    | `:ok_hand:`                       | Update code due to code review        |
| ♿️    | `:wheelchair:`                    | Improve accessibility                  |
| 💡    | `:bulb:`                          | Document source code                  |
| 🍻    | `:beers:`                         | Code醉酒 with friends                 |
| 💬    | `:speech_balloon:`                | Update text and literals              |
| 🗃️    | `:card_file_box:`                 | Perform database related changes       |
| 🔊    | `:loud_sound:`                    | Add logs                              |
| 🔇    | `:mute:`                          | Remove logs                           |
| 👥    | `:busts_in_silhouette:`           | Add contributors                      |
| 🚸    | `:children_crossing:`             | Improve UX/availability               |
| 🏗️    | `:building_construction:`         | Make architectural changes            |
| 📱    | `:iphone:`                        | Work on responsive design             |
| 🤡    | `:clown_face:`                    | Mock things                           |
| 🥚    | `:egg:`                           | Add an Easter egg                     |
| 🙈    | `:see_no_evil:`                   | Add or update .gitignore file         |
| 📸    | `:camera_flash:`                  | Add or update snapshots               |

---

## Scopes

Scopes are based on the project `src/` directory structure. **The following defined scopes must be used**:

### Top-level Modules

| scope       | Corresponding Directory   | Description                                     |
| ----------- | ------------------------- | ----------------------------------------------- |
| `frontend`  | `src/frontend/`           | Frontend: lexer, parser, type checking         |
| `middle`    | `src/middle/`             | Middle layer: IR, optimization, monomorphization |
| `backends`  | `src/backends/`           | Backend: interpreter, runtime, REPL            |
| `std`       | `src/std/`               | Standard library                                |
| `formatter` | `src/formatter/`          | Code formatter                                  |
| `lsp`       | `src/lsp/`               | Language Server Protocol                        |
| `package`   | `src/package/`            | Package manager                                 |
| `util`      | `src/util/`              | Utilities: diagnostics, cache, i18n            |

### Frontend Sub-modules

| scope       | Corresponding Directory              | Description       |
| ----------- | ----------------------------------- | ----------------- |
| `parser`    | `src/frontend/core/parser/`         | Parser            |
| `lexer`     | `src/frontend/core/lexer/`          | Lexer             |
| `typecheck` | `src/frontend/core/typecheck/`      | Type checking     |
| `types`     | `src/frontend/core/types/`          | Type system definitions |

### Middle Layer Sub-modules

| scope          | Corresponding Directory                   | Description                  |
| -------------- | ----------------------------------------- | ---------------------------- |
| `codegen`      | `src/middle/passes/codegen/`              | Code generation (bytecode)   |
| `monomorphize` | `src/middle/passes/monomorphize/`         | Monomorphization             |
| `lifetime`     | `src/middle/passes/lifetime/`             | Lifetime analysis            |

### Backend Sub-modules

| scope    | Corresponding Directory           | Description               |
| --------- | -------------------------------- | ------------------------- |
| `repl`    | `src/backends/dev/repl/`         | REPL interactive CLI      |
| `shell`   | `src/backends/dev/shell.rs`      | Shell command handling    |
| `runtime` | `src/backends/runtime/`          | Runtime execution engine  |

### Documentation Scopes

| scope   | Description                    |
| -------- | ------------------------------ |
| `docs`   | General documentation updates  |
| `design` | Language design specs (RFC)    |
| `plan`   | Implementation plan documents  |

### Other Scopes

| scope    | Description                                  |
| --------- | -------------------------------------------- |
| `build`   | Build system, Cargo configuration            |
| `ci`      | CI/CD configuration (GitHub Actions)         |
| `test`    | Test-related                                 |
| `release` | Release-related                              |
| `meta`    | Project meta configuration (.claude, .gitignore, etc.) |

---

## Message Guidelines

### Version Management

Version numbers are defined in the `version` field in `Cargo.toml` at the project root:

```toml
[package]
version = "0.7.2"
```

Semantic versioning `MAJOR.MINOR.PATCH` is used:

| Version Type | Description                           | Example          |
| ----------- | ------------------------------------- | ---------------- |
| **major**   | Breaking updates, incompatible API changes | 0.7.2 → 1.0.0 |
| **minor**   | New features, backward compatible     | 0.7.2 → 0.8.0    |
| **patch**   | Bug fixes, backward compatible        | 0.7.2 → 0.7.3    |

> ⚠️ When releasing, **update the `Cargo.toml` version number on the dev branch**. After merging the PR to main via CI, tags and Releases are created automatically. **Do not manually push tags**, otherwise CI will skip the release process.

---

## CI Release Process

Releases are automatically completed by GitHub Actions (`release.yml`). The process is as follows:

```
1. Update Cargo.toml version field on dev branch
2. cargo build updates Cargo.lock
3. Commit in release format (see below 🔖 Release Commits)
   - Commit message must contain all changes since last release (full PR content)
4. Create PR from dev to main
5. Merge PR to main
6. CI auto-detects:
   - Reads Cargo.toml version → "v{version}"
   - Checks if tag exists
   - Not exists → triggers full release process
   - Exists → skips (no duplicate publishing)
7. CI auto-executes:
   - Parallel: cross-platform builds (Linux/Windows/macOS) + security audit + tests
   - After all pass: create tag, package artifacts, publish GitHub Release
```

### Key Rules

| Rule                               | Description                                                                  |
| ---------------------------------- | ---------------------------------------------------------------------------- |
| **Do not manually push tags**      | CI decides whether to publish based on tag existence; manual tags cause CI to skip |
| **Version bump on dev**            | Release commit is completed on dev, merged to main via PR                    |
| **Release commit contains full changelog** | Commit message needs all changes for this release, as it's the PR description source |
| **Do not merge main back to dev**  | dev auto-syncs after PR merge; no reverse merge needed                       |

---

## Message Guidelines

### Language Guidelines

**It is recommended to use English for commit messages** to maintain consistency in team communication.

- Subject should be in English, concise and clear
- Body can use English for detailed explanations
- If there are special technical terms, English may be retained

### Subject

- Use English, concise and clear
- Length does not exceed 50 characters
- No period at the end

### Body

- Detailed explanation of the reason and method of the change
- Each line does not exceed 72 characters
- Use - or * to list points

### Footer

- **Breaking Changes**: Start with `BREAKING CHANGE:`
- **Issue Closure**: Use `Closes #123` or `Fixes #456`

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

SessionREPL doesn't correctly register completer in multi-line mode,
causing Tab completion to not trigger.

Fix #128
```

### 📝 docs - Documentation Update

```
:memo: docs(design): Update ownership model and type system specification

Sync latest design changes from RFC-009 and RFC-011.
```

### ♻️ refactor - Refactoring

```
:recycle: refactor(typecheck): Separate primitive value types from Dup shallow copy semantics

Decouple value types and copy semantics in MonoType,
eliminating special cases in match branches.
```

### ⚡ perf - Performance Optimization

```
:zap: perf(types): Optimize const generic evaluation performance

Add depth limit (default 128) for recursive evaluation,
preventing stack overflow from maliciously constructed type expressions.
```

### ✅ test - Testing

```
:white_check_mark: test(typecheck): Add scope VarInfo mutability tests

Cover scenarios:
- Read-only access of immutable bindings
- Mutability tracking of mut bindings
- Mutability propagation across scopes
```

### 🔧 chore - Chores

```
:wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap

Upgrade 6 production dependencies to latest stable versions.
```

### 🚀 ci - CI Configuration

```
:rocket: ci: Fix nightly build Rust version being too low

Update RUST_TOOLCHAIN from 1.91.0 to 1.96.0,
matching the rust-version requirement in Cargo.toml.
```

### 💄 style - Formatting Changes

```
:lipstick: style(frontend): Apply cargo fmt formatting

Unified function signature line wrapping style.
```

---

---

## 🔖 Release Commits

When the commit is a **Release**, it must follow these conventions:

### Release Commit Format

```
:bookmark: V<version>: <release title>

## 📦 Version Information

**Release Date:** YYYY-MM-DD

**Version:** <old version> → <new version>

---

## ✨ New Features

### <Feature Module>
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

## 📝 Commit History

| Commit | Description |
|:---:|------|
| `<hash>` | :bookmark: V<version> |
| `<hash>` | <commit message> |
```

### Release Requirements

1. **Message Header**: Must use `:bookmark:` + `V<version>` format
2. **Version Number**: Follow semantic versioning conventions
3. **Content Completeness**: Must include all commit content since the last release
4. **Categorize by Type**: Organize by types like `feat`, `fix`, `refactor`, `chore`, etc.
5. **Commit History**: List all relevant commit hashes and descriptions

### Release Example

```
:bookmark: V0.7.2: REPL rewrite and type system improvements

## 📦 Version Information

**Release Date:** 2026-06-01

**Version:** 0.7.1 → 0.7.2

---

## ✨ New Features

- :sparkles: feat(typecheck): Implement generic type parameter inference
- :sparkles: feat(typecheck): Add MonoType::Generic structured generic representation
- :sparkles: feat: Integrate CLI REPL commands into SessionREPL

---

## ♻️ Refactoring

- :recycle: refactor(backends): Remove tui_repl module, rewrite as SessionREPL
- :recycle: refactor(typecheck): scope variable storage introduces VarInfo for mutability tracking
- :recycle: refactor(typecheck): Separate primitive value types from Dup shallow copy semantics

---

## 🐛 Bug Fixes

- :bug: fix(repl): Configure default REPL history, fix shell evaluate_code
- :bug: fix(repl): Register completer and fix multi-line input
- :bug: fix(repl): Remove extra semicolon in wrap_code to preserve expression values

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
| `f438aab` | :sparkles: feat(typecheck): Implement generic type parameter inference |
| `bf0c121` | :zap: perf(types): Recursive depth limit |
| `6edac15` | :sparkles: feat: Integrate CLI REPL into SessionREPL |
| `02cf54f` | :sparkles: feat(typecheck): MonoType::Generic |
| `3160a28` | :recycle: refactor(typecheck): VarInfo for mutability tracking |
| `f00a2a4` | :recycle: refactor(backends): Remove tui_repl module |
| `afe3e0c` | :bug: fix(repl): REPL history and shell fixes |
| `c4d2242` | :wrench: chore(build): Dependency bump |
```

### How to Get Commit History

```bash
# View all commits since last release
git log --oneline <last release commit>..HEAD

# Or view last N commits
git log --oneline -20
```

### Reference Template

Please refer to [`release.md`](release.md) template format for release documentation.

---

### 1. Set Commit Template

```bash
# Execute in project root
git config commit.template .gitmessage.txt
```

### 2. Template File

The format of `.gitmessage.txt` file in the project root is as follows:

```
# emoji_code type(scope): subject (in English)
#
# Body content (optional)
#
# Footer (optional)
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: frontend, parser, lexer, typecheck, types, middle, codegen,
#         monomorphize, lifetime, backends, repl, shell, runtime,
#         std, formatter, lsp, package, util, docs, design, plan,
#         build, ci, test, release, meta
#
# Examples:
# ✨ feat(db): Add batch delete todos feature
# 🐛 fix(provider): Fix timer background recovery issue
#
# Release format: 🔖 V1.0.0: Release title
```

---

## FAQ

### Q: How to choose a commit type?

- **feat**: Visible functional changes for users
- **fix**: Fixes for issues reported by users
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

> 💡 **Tip**: Keep commits atomic and descriptions clear, making code review and backtracking more efficient!
