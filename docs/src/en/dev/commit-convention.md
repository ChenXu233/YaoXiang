# Commit Guidelines

This document defines the Git commit specification for the YaoXiang project, aiming to keep the commit history clear, readable, and easy to understand.

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

**Very important!!!!!!!! Don't forget it!!!!!!** All commit messages follow this format:

```
:emoji_code: type(scope): subject (in Chinese)

[optional body]

[optional footer]
```

> ⚠️ **Important**: You must use **emoji codes** (such as `:sparkles:`) instead of directly inputting emoji characters.
>
> **Chinese commit messages are recommended** to maintain consistency in team communication.

### Components

| Part      | Description                            | Required |
| --------- | -------------------------------------- | -------- |
| emoji_code | Emoji symbol identifying commit type   | ✅       |
| type      | Commit type                            | ✅       |
| scope     | Affected area                          | ✅       |
| subject   | Brief description (in Chinese, max 50 chars) | ✅   |
| body      | Detailed explanation (optional)         | ❌       |
| footer    | Breaking changes or issue closure (optional) | ❌   |

---

## Commit Types

| emoji_code               | type     | Description                       |
| ----------------------- | -------- | --------------------------------- |
| :sparkles:              | feat     | New feature                       |
| :bug:                   | fix      | Bug fix                           |
| :memo:                  | docs     | Documentation changes only        |
| :lipstick:              | style    | Code formatting (no functionality impact) |
| :recycle:               | refactor | Code refactoring                  |
| :zap:                   | perf     | Performance optimization           |
| :white_check_mark:      | test     | Adding or modifying tests         |
| :wrench:                | chore    | Build tools, auxiliary tool changes |
| :building_construction: | build    | Build system changes               |
| :rocket:                | ci       | CI configuration changes           |

---

## Complete Emoji Reference

The following is a complete emoji list consistent with the gitmoji project. Choose the appropriate emoji based on your commit content:

| emoji | emoji code                    | commit description                |
| :---- | :---------------------------- | :------------------------------- |
| 🎨    | `:art:`                       | Improve code structure/format    |
| ⚡️    | `:zap:` / `:racehorse:`       | Improve performance              |
| 🔥    | `:fire:`                      | Remove code or files             |
| 🐛    | `:bug:`                       | Fix a bug                        |
| 🚑    | `:ambulance:`                 | Important patch                  |
| ✨    | `:sparkles:`                  | Introduce new features           |
| 📝    | `:memo:`                      | Write docs                       |
| 🚀    | `:rocket:`                    | Deploy stuff                     |
| 💄    | `:lipstick:`                  | Update UI and style files        |
| 🎉    | `:tada:`                      | Initial commit                   |
| ✅    | `:white_check_mark:`          | Add tests                        |
| 🔒    | `:lock:`                      | Fix security issues              |
| 🍎    | `:apple:`                     | Fix something on macOS           |
| 🐧    | `:penguin:`                   | Fix something on Linux           |
| 🏁    | `:checkered_flag:`            | Fix something on Windows         |
| 🤖    | `:robot:`                     | Fix something on Android         |
| 🍏    | `:green_apple:`               | Fix something on iOS             |
| 🔖    | `:bookmark:`                  | Release/Version tag              |
| 🚨    | `:rotating_light:`            | Remove linter warnings           |
| 🚧    | `:construction:`              | Work in progress                 |
| 💚    | `:green_heart:`               | Fix CI build                     |
| ⬇️    | `:arrow_down:`                | Downgrade dependencies           |
| ⬆️    | `:arrow_up:`                  | Upgrade dependencies             |
| 📌    | `:pushpin:`                   | Pin dependencies to specific versions |
| 👷    | `:construction_worker:`       | Add CI build system              |
| 📈    | `:chart_with_upwards_trend:`  | Add analytics or tracking code   |
| ♻️    | `:recycle:`                   | Refactor code                    |
| 🔨    | `:hammer:`                    | Major refactoring                |
| ➖    | `:heavy_minus_sign:`          | Remove a dependency              |
| 🐳    | `:whale:`                     | Docker related work              |
| ➕    | `:heavy_plus_sign:`           | Add a dependency                 |
| 🔧    | `:wrench:`                    | Modify config files              |
| 🌐    | `:globe_with_meridians:`      | Internationalization and localization |
| ✏️    | `:pencil2:`                   | Fix typos                        |
| 💩    | `:hankey:`                    | Write bad code that needs to be improved |
| ⏪️    | `:rewind:`                    | Revert changes                   |
| 🔀    | `:twisted_rightwards_arrows:` | Merge branches                   |
| 📦    | `:package:`                   | Updating compiled files or packages |
| 👽    | `:alien:`                     | Update code due to external API changes |
| 🚚    | `:truck:`                     | Move or rename files             |
| 📄    | `:page_facing_up:`            | Add or update license            |
| 💥    | `:boom:`                      | Introduce breaking changes       |
| 🍱    | `:bento:`                     | Add or update assets             |
| 👌    | `:ok_hand:`                   | Update code due to code review changes |
| ♿️    | `:wheelchair:`                | Improve accessibility             |
| 💡    | `:bulb:`                      | Document source code             |
| 🍻    | `:beers:`                     | Drink and write code             |
| 💬    | `:speech_balloon:`            | Update text and literals          |
| 🗃️    | `:card_file_box:`             | Perform database related changes  |
| 🔊    | `:loud_sound:`                | Add logs                         |
| 🔇    | `:mute:`                      | Remove logs                      |
| 👥    | `:busts_in_silhouette:`       | Add contributors                 |
| 🚸    | `:children_crossing:`         | Improve user experience/usability |
| 🏗️    | `:building_construction:`     | Make architectural changes        |
| 📱    | `:iphone:`                    | Work on responsive design        |
| 🤡    | `:clown_face:`                | Mock things                      |
| 🥚    | `:egg:`                       | Add an easter egg                |
| 🙈    | `:see_no_evil:`               | Add or update .gitignore          |
| 📸    | `:camera_flash:`              | Add or update snapshots           |

---

## Scopes

Scopes are based on the project's `src/` directory structure. **You must use the following defined scopes**:

### Top-level Modules

| scope         | Corresponding directory | Description                                      |
| -----------   | ----------------------- | ------------------------------------------------ |
| `frontend`    | `src/frontend/`         | Frontend: lexing, parsing, type checking        |
| `middle`      | `src/middle/`           | Middle-end: IR, optimization, monomorphization    |
| `backends`    | `src/backends/`         | Backend: interpreter, runtime, REPL              |
| `std`         | `src/std/`              | Standard library                                 |
| `formatter`   | `src/formatter/`        | Code formatter                                   |
| `lsp`         | `src/lsp/`              | Language Server Protocol                         |
| `package`     | `src/package/`          | Package manager                                  |
| `util`        | `src/util/`             | Utilities: diagnostics, caching, i18n            |

### Frontend Sub-modules

| scope         | Corresponding directory                | Description          |
| -----------   | -------------------------------------- | -------------------- |
| `parser`      | `src/frontend/core/parser/`            | Parser               |
| `lexer`       | `src/frontend/core/lexer/`             | Lexer                |
| `typecheck`   | `src/frontend/core/typecheck/`        | Type checking        |
| `types`       | `src/frontend/core/types/`            | Type system definition |

### Middle-end Sub-modules

| scope           | Corresponding directory                   | Description                   |
| -------------- | ----------------------------------------- | ----------------------------- |
| `codegen`      | `src/middle/passes/codegen/`             | Code generation (bytecode)    |
| `monomorphize` | `src/middle/passes/monomorphize/`        | Monomorphization              |
| `lifetime`     | `src/middle/passes/lifetime/`            | Lifetime analysis             |

### Backend Sub-modules

| scope     | Corresponding directory          | Description              |
| --------- | -------------------------------- | ------------------------ |
| `repl`    | `src/backends/dev/repl/`         | REPL interactive shell   |
| `shell`   | `src/backends/dev/shell.rs`      | Shell command handling   |
| `runtime` | `src/backends/runtime/`          | Runtime execution engine |

### Documentation Scopes

| scope    | Description                          |
| -------- | ------------------------------------ |
| `docs`   | General documentation updates        |
| `design` | Language design specification (RFC) |
| `plan`   | Implementation plan documents       |

### Other Scopes

| scope     | Description                                      |
| --------- | ------------------------------------------------ |
| `build`   | Build system, Cargo configuration                |
| `ci`      | CI/CD configuration (GitHub Actions)            |
| `test`    | Test-related                                     |
| `release` | Release-related                                  |
| `meta`    | Project meta configuration (.claude, .gitignore, etc.) |

---

## Message Conventions

### Version Management

Version numbers are defined in the `version` field of `Cargo.toml` in the project root directory:

```toml
[package]
version = "0.7.2"
```

Semantic versioning `MAJOR.MINOR.PATCH` is used:

| Version Type | Description                                | Example          |
| ------------ | ------------------------------------------ | ---------------- |
| **major**    | Breaking changes, incompatible API changes | 0.7.2 → 1.0.0    |
| **minor**    | New features, backward compatible          | 0.7.2 → 0.8.0    |
| **patch**    | Bug fixes, backward compatible             | 0.7.2 → 0.7.3    |

> ⚠️ When releasing, **update the `Cargo.toml` version number on the dev branch**.
> After merging the PR to main, CI automatically creates tags and Releases. **Do not manually push tags**, otherwise CI will skip the release process.

---

## CI Release Process

Releases are automatically completed by GitHub Actions (`release.yml`). The process is as follows:

```
1. Update the version field in Cargo.toml on the dev branch
2. Run cargo build to update Cargo.lock
3. Commit in release format (see below for 🔖 Release Commits)
   - Commit message must contain all changes since the last release (i.e., complete PR content)
4. Create a PR from dev to main
5. Merge PR to main
6. CI automatically detects:
   - Read Cargo.toml version number → "v{version}"
   - Check if this tag already exists
   - Does not exist → trigger full release process
   - Already exists → skip (won't publish again)
7. CI automatically executes:
   - Parallel: cross-platform builds (Linux/Windows/macOS) + security audit + tests
   - After all pass: create tag, package artifacts, publish GitHub Release
```

### Key Rules

| Rule                                      | Description                                                                          |
| ----------------------------------------- | ------------------------------------------------------------------------------------- |
| **Do not manually push tags**             | CI decides whether to publish based on tag existence; manual tag push will cause CI to skip |
| **Bump version on dev**                   | Release commits are completed on dev, then merged to main via PR                      |
| **Release commit contains complete changelog** | Commit message must contain all changes for this release, as it's the source of PR description |
| **Do not merge main back to dev**        | After PR merge, dev will automatically sync; no reverse merge needed                  |

---

## Message Conventions

### Language Conventions

**Chinese commit messages are recommended** to maintain consistency in team communication.

- Subject in Chinese, concise and clear
- Body can use Chinese for detailed explanation
- If there are special technical terms, English can be retained

### Subject

- Use Chinese, concise and clear
- Maximum 50 characters
- No period at the end

### Body

- Detailed explanation of why and how the change was made
- Each line no more than 72 characters
- Use - or * to list points

### Footer

- **Breaking Changes**: Start with `BREAKING CHANGE:`
- **Closing Issues**: Use `Closes #123` or `Fixes #456`

---

## Examples

### ✨ feat - New Feature

```
:sparkles: feat(parser): Add closure syntax parsing support

Implement closure expression parsing:
- Support |args| body shorthand syntax
- Support move semantics for capture
- Add closure type inference

Closes #42
```

### 🐛 fix - Bug Fix

```
:bug: fix(repl): Fix completer failure in multi-line input

SessionREPL does not properly register the completer in multi-line mode,
causing Tab completion to fail to trigger.

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

Add depth limit for recursive evaluation (default 128),
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

### 🔧 chore - Miscellaneous

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

### 💄 style - Formatting

```
:lipstick: style(frontend): Apply cargo fmt formatting

Unify function signature line-breaking style.
```

---

---

## 🔖 Release Commits

When a commit is a **Release**, it must follow these specifications:

### Release Commit Format

```
:bookmark: V<version>: <release title>

## 📦 Version Information

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

```

### Release Requirements

1. **Message Header**: Must use `:bookmark:` + `V<version>` format
2. **Version Number**: Follow semantic versioning specification
3. **Content Completeness**: Must include introductions of **all commits** since the last release
4. **Categorize by Type**: Organize by types like `feat`, `fix`, `refactor`, `chore`, etc.

### Release Example

```
:bookmark: V0.7.2: REPL Rewrite and Type System Improvements

## 📦 Version Information

**Release Date:** 2026-06-01

**Version:** 0.7.1 → 0.7.2

---

## ✨ New Features

- :sparkles: feat(typecheck): Implement automatic inference for generic type parameters
- :sparkles: feat(typecheck): Add MonoType::Generic structured generic representation
- :sparkles: feat: Connect CLI REPL commands to SessionREPL

---

## ♻️ Refactoring & Optimization

- :recycle: refactor(backends): Remove tui_repl module, rewrite as SessionREPL
- :recycle: refactor(typecheck): Introduce VarInfo tracking for scope variable mutability
- :recycle: refactor(typecheck): Separate primitive value types from Dup shallow copy semantics

---

## 🐛 Bug Fixes

- :bug: fix(repl): Configure default REPL history, fix shell evaluate_code
- :bug: fix(repl): Register completer and fix multi-line input
- :bug: fix(repl): Remove extra semicolons in wrap_code to preserve expression values

---

## ⚡ Performance Optimization

- :zap: perf(types): Add recursive depth limit for const generic evaluation

---

## 🔧 Other Changes

- :wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap, owo-colors
- :white_check_mark: test(typecheck): Add scope VarInfo mutability tests

### Reference Template

For release documentation, please refer to the [`release.md`](release.md) template format for composition.

---

### 1. Set Commit Template

```bash
# Execute in project root directory
git config commit.template .gitmessage.txt
```

### 2. Template File

The `.gitmessage.txt` file in the project root directory has the following format:

```
# emoji_code type(scope): subject (in Chinese)
#
# body content (optional)
#
# footer (optional)
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

- **feat**: Visible functional changes to users
- **fix**: Fix issues reported by users
- **docs**: README, comments, etc.
- **chore**: Dependency updates, config files
- **refactor**: Code optimization without behavior changes

### Q: When should I split commits?

- Each commit does **one thing** only
- Related features are committed together, unrelated ones separately
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [Complete Emoji List](#complete-emoji-reference)
- [release.md](release.md) - Release template

---

> 💡 **Tip**: Keep commits atomic and descriptions clear to make code review and tracing more efficient!