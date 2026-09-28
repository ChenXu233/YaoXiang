# Commit Submission Guide

This document defines the Git commit conventions for the YaoXiang project, aiming to keep the commit
history clear, readable, and easy to understand.

---

## Table of Contents

- [Commit Format](#commit-format)
- [Commit Type](#commit-type)
- [Complete Emoji Reference](#complete-emoji-reference)
- [Scope](#scope)
- [Version Management](#version-management)
- [Message Conventions](#message-conventions)
- [Language Conventions](#language-conventions)
- [🔖 Release Commit](#-release-commit)
- [Examples](#examples)
- [Using Commit Template](#using-commit-template)
- [FAQ](#faq)

---

## Commit Format

**VERY IMPORTANT！！！！！！ DO NOT FORGET！！！** All commit messages follow the format below:

```
:emoji_code: type(scope): subject (in Chinese)

[Optional body content]

[Optional footer]
```

> ⚠️ **Important**: You **must** use **emoji code** (e.g., `:sparkles:`) rather than typing the
> emoji character directly.
>
> **It is recommended to use Chinese commit messages** to maintain consistency in team
> communication.

### Components

| Component  | Description                                  | Required |
| ---------- | -------------------------------------------- | -------- |
| emoji_code | Emoji identifier indicating commit type      | ✅       |
| type       | Commit type                                  | ✅       |
| scope      | Area of impact                               | ✅       |
| subject    | Brief description (in Chinese, max 50 chars) | ✅       |
| body       | Detailed description (optional)              | ❌       |
| footer     | Breaking changes or issue closure (optional) | ❌       |

---

## Commit Type

| emoji_code              | type     | Description                            |
| ----------------------- | -------- | -------------------------------------- |
| :sparkles:              | feat     | New feature                            |
| :bug:                   | fix      | Bug fix                                |
| :memo:                  | docs     | Documentation changes only             |
| :lipstick:              | style    | Code formatting (no functional impact) |
| :recycle:               | refactor | Code refactoring                       |
| :zap:                   | perf     | Performance improvement                |
| :white_check_mark:      | test     | Add or modify tests                    |
| :wrench:                | chore    | Build tools, auxiliary tool changes    |
| :building_construction: | build    | Build system changes                   |
| :rocket:                | ci       | CI configuration changes               |

---

## Complete Emoji Reference

The following is the complete emoji list consistent with the gitmoji project. Choose the appropriate
emoji based on the commit content:

| emoji | emoji code                    | commit description                      |
| :---- | :---------------------------- | :-------------------------------------- |
| 🎨    | `:art:`                       | Improve code structure/formatting       |
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
| 🍎    | `:apple:`                     | Fix macOS-related issues                |
| 🐧    | `:penguin:`                   | Fix Linux-related issues                |
| 🏁    | `:checkered_flag:`            | Fix Windows-related issues              |
| 🤖    | `:robot:`                     | Fix Android-related issues              |
| 🍏    | `:green_apple:`               | Fix iOS-related issues                  |
| 🔖    | `:bookmark:`                  | Release/version tag                     |
| 🚨    | `:rotating_light:`            | Remove linter warnings                  |
| 🚧    | `:construction:`              | Work in progress                        |
| 💚    | `:green_heart:`               | Fix CI build issues                     |
| ⬇️    | `:arrow_down:`                | Downgrade dependencies                  |
| ⬆️    | `:arrow_up:`                  | Upgrade dependencies                    |
| 📌    | `:pushpin:`                   | Pin dependencies to specific versions   |
| 👷    | `:construction_worker:`       | Add CI build system                     |
| 📈    | `:chart_with_upwards_trend:`  | Add analytics or tracking code          |
| ♻️    | `:recycle:`                   | Refactor code                           |
| 🔨    | `:hammer:`                    | Major refactoring                       |
| ➖    | `:heavy_minus_sign:`          | Remove a dependency                     |
| 🐳    | `:whale:`                     | Docker-related work                     |
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
| 🥚    | `:egg:`                       | Add an Easter egg                       |
| 🙈    | `:see_no_evil:`               | Add or update .gitignore file           |
| 📸    | `:camera_flash:`              | Add or update snapshots                 |

---

## Scope

The scope is based on the `src/` directory structure of the project. **You must use the following
defined scopes**:

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
| `util`      | `src/util/`             | Utilities: diagnostics, cache, i18n              |

### Frontend Sub-Modules

| Scope       | Corresponding Directory        | Description             |
| ----------- | ------------------------------ | ----------------------- |
| `parser`    | `src/frontend/core/parser/`    | Syntax parser           |
| `lexer`     | `src/frontend/core/lexer/`     | Lexical analyzer        |
| `typecheck` | `src/frontend/core/typecheck/` | Type checking           |
| `types`     | `src/frontend/core/types/`     | Type system definitions |

### Middle Layer Sub-Modules

| Scope          | Corresponding Directory           | Description                |
| -------------- | --------------------------------- | -------------------------- |
| `codegen`      | `src/middle/passes/codegen/`      | Code generation (bytecode) |
| `monomorphize` | `src/middle/passes/monomorphize/` | Monomorphization           |
| `lifetime`     | `src/middle/passes/lifetime/`     | Lifetime analysis          |

### Backend Sub-Modules

| Scope     | Corresponding Directory     | Description                   |
| --------- | --------------------------- | ----------------------------- |
| `repl`    | `src/backends/dev/repl/`    | REPL interactive command line |
| `shell`   | `src/backends/dev/shell.rs` | Shell command handling        |
| `runtime` | `src/backends/runtime/`     | Runtime execution engine      |

### Documentation Scope

| Scope    | Description                          |
| -------- | ------------------------------------ |
| `docs`   | General documentation updates        |
| `design` | Language design specifications (RFC) |
| `plan`   | Implementation plan documents        |

### Other Scopes

| Scope     | Description                                            |
| --------- | ------------------------------------------------------ |
| `build`   | Build system, Cargo configuration                      |
| `ci`      | CI/CD configuration (GitHub Actions)                   |
| `test`    | Test-related                                           |
| `release` | Release-related                                        |
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
| **patch**    | Bug fixes, backward compatible         | 0.7.2 → 0.7.3 |

> ⚠️ When releasing, **update the `Cargo.toml` version number on the `dev` branch**. After the PR is
> merged into `main`, CI will automatically create the tag and Release. **Do not push tags
> manually**, otherwise CI will skip the release process.

---

## CI Release Process

Releases are performed automatically by GitHub Actions (`release.yml`). The process is as follows:

```
1. Update the version field in Cargo.toml on the dev branch
2. Run cargo build to update Cargo.lock
3. Commit according to the release format (see 🔖 Release Commit below)
   - The commit message must include all changes since the last release (i.e., the full content of the PR)
4. Create a PR from dev to main
5. Merge the PR into main
6. CI automatically detects:
   - Reads the Cargo.toml version number → "v{version}"
   - Checks whether the tag already exists
   - Does not exist → triggers the full release process
   - Exists → skips (will not republish)
7. CI automatically executes:
   - In parallel: cross-platform build (Linux/Windows/macOS) + security audit + tests
   - After all pass: create tag, package artifacts, publish GitHub Release
```

### Key Rules

| Rule                                           | Description                                                                                                   |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| **Do not push tags manually**                  | CI determines whether to publish based on whether the tag exists. Manually pushing tags will cause CI to skip |
| **Bump version on dev**                        | The release commit is done on dev and merged into main via PR                                                 |
| **Release commit contains complete changelog** | The commit message must include all the changes for this release, as it is the source of the PR description   |
| **Do not merge main back to dev**              | dev will be automatically synced after the PR is merged; no reverse merge is needed                           |

---

## Message Conventions

### Language Conventions

**It is recommended to use Chinese commit messages** to maintain consistency in team communication.

- Use Chinese for the subject, keep it concise and clear
- Use Chinese for detailed descriptions in the body
- Keep English for special technical terms when necessary

### Subject

- Use Chinese, keep it concise and clear
- Length should not exceed 50 characters
- No period at the end

### Body

- Describe the reason and method of the change in detail
- No more than 72 characters per line
- Use `-` or `*` to list points

### Footer

- **Breaking changes**: Start with `BREAKING CHANGE:`
- **Close issues**: Use `关闭 #123` or `修复 #456`

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
:bug: fix(repl): Fix completer failure during multi-line input

SessionREPL did not register the completer correctly in multi-line mode,
causing Tab completion to fail to trigger.

Fixes #128
```

### 📝 docs - Documentation Update

```
:memo: docs(design): Update ownership model and type system specifications

Sync the latest design changes from RFC-009 and RFC-011.
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

Add depth limit (default 128) for recursive evaluation
to prevent stack overflow caused by maliciously constructed type expressions.
```

### ✅ test - Test

```
:white_check_mark: test(typecheck): Add scope VarInfo mutability tests

Coverage scenarios:
- Read-only access to immutable bindings
- Mutability tracking of mut bindings
- Cross-scope mutability propagation
```

### 🔧 chore - Miscellaneous

```
:wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap

Upgrade 6 production dependencies to the latest stable versions.
```

### 🚀 ci - CI Configuration

```
:rocket: ci: Fix Rust version too low in nightly build

Update RUST_TOOLCHAIN from 1.91.0 to 1.96.0
to match the rust-version requirement in Cargo.toml.
```

### 💄 style - Formatting Adjustment

```
:lipstick: style(frontend): Apply cargo fmt formatting

Unify the line break style of function signatures.
```

---

---

## 🔖 Release Commit

When this commit is a **release**, the following conventions must be followed:

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



### Release Requirements

1. **Message header**: Must use `:bookmark:` + `V<version>` format
2. **Version number**: Follow the semantic versioning specification
3. **Content completeness**: Must include descriptions of **all commits** since the last release
4. **Categorize by type**: Organize by `feat`, `fix`, `refactor`, `chore`, etc.

### Release Example

```

:bookmark: V0.7.2: REPL Rewrite and Type System Improvements

## 📦 Version Information

**Release Date:** 2026-06-01

**Version:** 0.7.1 → 0.7.2

---

## ✨ New Features

- :sparkles: feat(typecheck): Implement automatic type parameter inference for generics
- :sparkles: feat(typecheck): Add MonoType::Generic structured generics representation
- feat: Hook CLI REPL command into SessionREPL

---

## ♻️ Refactoring & Optimization

- :recycle: refactor(backends): Remove tui_repl module, rewrite as SessionREPL
- :recycle: refactor(typecheck): Introduce VarInfo to track mutability in scope variable storage
- :recycle: refactor(typecheck): Separate primitive value types from Dup shallow copy semantics

---

## 🐛 Bug Fixes

- :bug: fix(repl): Configure default REPL history, fix shell evaluate_code
- :bug: fix(repl): Register completer and fix multi-line input
- :bug: fix(repl): Remove extra semicolon in wrap_code to preserve expression value

---

## ⚡ Performance Optimization

- :zap: perf(types): Add recursion depth limit for const generic evaluation

---

## 🔧 Other Changes

- :wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap, owo-colors
- :white_check_mark: test(typecheck): Add scope VarInfo mutability tests

### Reference Template

For release documents, please refer to the [`release.md`](release.md) template format.

---

### 1. Set Up Commit Template

```bash
# Run in the project root directory
git config commit.template .gitmessage.txt
```

### 2. Template File

The `.gitmessage.txt` file at the project root directory has the following format:

```
# emoji_code type(scope): subject (in Chinese)
#
# Body content (optional)
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

- **feat**: Changes to features visible to the user
- **fix**: Fix issues reported by the user
- **docs**: README, comments and other documentation
- **chore**: Dependency updates, configuration files
- **refactor**: Code optimization that does not change behavior

### Q: When should commits be split?

- Each commit should do **one thing only**
- Commit related features together, separate unrelated ones
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [Complete Emoji List](#complete-emoji-reference)
- [release.md](release.md) - Release template

---

> 💡 **Tip**: Keep commits atomic and descriptions clear, making code review and backtracking more
> efficient!
