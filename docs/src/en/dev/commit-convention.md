# Commit Submission Guide

This document defines the Git commit conventions for the YaoXiang project, aimed at keeping the
commit history clear, readable, and easy to understand.

---

## Table of Contents

- [Commit Format](#commit-format)
- [Commit Types](#commit-types)
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

**Very Important!!!!!! Don't Forget!!!** All commit messages follow the format below:

```
:emoji_code: type(scope): subject (Chinese)

[Optional body content]

[Optional footer]
```

> ⚠️ **Important**: You **must** use **emoji codes** (e.g. `:sparkles:`) rather than typing the
> emoji characters directly.
>
> **Chinese commit messages are recommended** to maintain consistency in team communication.

### Components

| Part       | Description                                             | Required |
| ---------- | ------------------------------------------------------- | -------- |
| emoji_code | Emoji symbol identifying the commit type                | ✅       |
| type       | Commit type                                             | ✅       |
| scope      | Area of impact                                          | ✅       |
| subject    | Brief description (Chinese, no more than 50 characters) | ✅       |
| body       | Detailed explanation (optional)                         | ❌       |
| footer     | Breaking changes or closed issues (optional)            | ❌       |

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
| 🎉    | `:tada:`                      | Begin a project                         |
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
| 🙈    | `:see_no_evil:`               | Add or update .gitignore files          |
| 📸    | `:camera_flash:`              | Add or update snapshots                 |

---

## Scope

The scope is based on the `src/` directory structure of the project. **You must use one of the
scopes defined below**:

### Top-level Modules

| Scope       | Corresponding Directory | Description                                      |
| ----------- | ----------------------- | ------------------------------------------------ |
| `frontend`  | `src/frontend/`         | Frontend: lexer, parser, typecheck               |
| `middle`    | `src/middle/`           | Middle layer: IR, optimization, monomorphization |
| `backends`  | `src/backends/`         | Backends: interpreter, runtime, REPL             |
| `std`       | `src/std/`              | Standard library                                 |
| `formatter` | `src/formatter/`        | Code formatter                                   |
| `lsp`       | `src/lsp/`              | Language Server Protocol                         |
| `package`   | `src/package/`          | Package manager                                  |
| `util`      | `src/util/`             | Utilities: diagnostics, cache, i18n              |

### Frontend Sub-modules

| Scope       | Corresponding Directory        | Description            |
| ----------- | ------------------------------ | ---------------------- |
| `parser`    | `src/frontend/core/parser/`    | Syntax parser          |
| `lexer`     | `src/frontend/core/lexer/`     | Lexical analyzer       |
| `typecheck` | `src/frontend/core/typecheck/` | Type checking          |
| `types`     | `src/frontend/core/types/`     | Type system definition |

### Middle Layer Sub-modules

| Scope          | Corresponding Directory           | Description                |
| -------------- | --------------------------------- | -------------------------- |
| `codegen`      | `src/middle/passes/codegen/`      | Code generation (bytecode) |
| `monomorphize` | `src/middle/passes/monomorphize/` | Monomorphization           |
| `lifetime`     | `src/middle/passes/lifetime/`     | Lifetime analysis          |

### Backend Sub-modules

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
| `test`    | Test-related                                           |
| `release` | Release-related                                        |
| `meta`    | Project meta configuration (.claude, .gitignore, etc.) |

---

## Message Conventions

### Version Management

The version number is defined in the `version` field of `Cargo.toml` in the project root:

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

> ⚠️ When releasing, **update the version in `Cargo.toml` on the dev branch**, and after merging the
> PR into main, CI will automatically create the tag and Release. **Do not push tags manually**,
> otherwise CI will skip the release process.

---

## CI Release Process

The release is automatically completed by GitHub Actions (`dist-release.yml`). The process is as
follows:

```
1. Update the version field in Cargo.toml on the dev branch
2. cargo build to update Cargo.lock
3. Commit according to the release format (see 🔖 Release Commit below)
   - The commit message must include all changes since the last release (i.e., the full PR content)
4. Create a PR from dev to main
5. Merge the PR into main
6. The gate job automatically detects:
   - Read the Cargo.toml version → "v{version}"
   - Check whether the tag already exists
   - Does not exist → trigger the full release process
   - Already exists → skip (no duplicate release)
7. Gate: security audit + fmt/clippy/unit tests/doc tests all pass
8. After all checks pass: create and push the tag
9. Post-tag build and release:
   - cargo-dist cross-platform build (Linux/Windows/macOS, five platforms) + reorganized packages (bin/ + lib/) + wasm + Inno setup
   - Publish GitHub Release (body generated by generate-commit-list.ts); apt repo is published separately to GitHub Pages
```

### Key Rules

| Rule                                       | Description                                                                                                                                                                                      |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Do not push tags manually**              | Release only recognizes the gate judgment on push to main; manually pushed tags will not trigger a release (for re-releases, use the `workflow_dispatch` of `dist-release.yml` to specify a tag) |
| **Bump version on dev**                    | The release commit is done on dev, and merged into main via PR                                                                                                                                   |
| **Release commit includes full changelog** | The commit message must include all changes in this release, since it serves as the source of the PR description                                                                                 |
| **Do not merge main back into dev**        | After the PR is merged, dev will be synced automatically; no reverse merge is needed                                                                                                             |

---

## Message Conventions

### Language Conventions

**Chinese commit messages are recommended** to maintain consistency in team communication.

- Use Chinese for the Subject, keeping it concise and clear
- Use Chinese in the Body for detailed explanations
- Keep English for special technical terms when necessary

### Subject

- Use Chinese, concise and clear
- No more than 50 characters
- No period at the end

### Body

- Explain in detail the reason and manner of the change
- Each line should not exceed 72 characters
- Use `-` or `*` for bullet points

### Footer

- **Breaking change**: Begin with `BREAKING CHANGE:`
- **Close issue**: Use `Closes #123` or `Fixes #456`

---

## Examples

### ✨ feat - New Feature

```
:sparkles: feat(parser): Add closure syntax parsing support

Implement closure expression parsing:
- Support the |args| body shorthand syntax
- Support move semantic capture
- Add closure type inference

Closes #42
```

### 🐛 fix - Bug Fix

```
:bug: fix(repl): Fix completer failure on multi-line input

SessionREPL did not register the completer correctly in multi-line mode,
causing Tab completion to fail to trigger.

Fixes #128
```

### 📝 docs - Documentation Update

```
:memo: docs(design): Update ownership model and type system specification

Sync the latest design changes from RFC-009 and RFC-011.
```

### ♻️ refactor - Refactoring

```
:recycle: refactor(typecheck): Separate primitive value type from Dup shallow-copy semantics

Decouple the value type from copy semantics in MonoType,
eliminating special cases in match branches.
```

### ⚡️ perf - Performance Optimization

```
:zap: perf(types): Optimize const generic evaluation performance

Add a depth limit for recursive evaluation (default 128),
to avoid stack overflow caused by maliciously constructed type expressions.
```

### ✅ test - Test

```
:white_check_mark: test(typecheck): Add scope VarInfo mutability tests

Coverage scenarios:
- Read-only access to immutable bindings
- Mutability tracking for mut bindings
- Mutability propagation across scopes
```

### 🔧 chore - Miscellaneous

```
:wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap

Upgrade 6 production dependencies to the latest stable versions.
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

Unify the line-breaking style of function signatures.
```

---

---

## 🔖 Release Commit

When this commit is a **Release**, the following conventions must be followed:

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



### Release Requirements

1. **Message header**: Must use the `:bookmark:` + `V<version>` format
2. **Version number**: Follow the semantic versioning convention
3. **Content completeness**: Must include **all commits** since the last release
4. **Categorized by type**: Organize by `feat`, `fix`, `refactor`, `chore`, etc.

### Release Example

```

:bookmark: V0.7.2: REPL Rewrite and Type System Improvements

## 📦 Version Info

**Release Date:** 2026-06-01

**Version:** 0.7.1 → 0.7.2

---

## ✨ New Features

- :sparkles: feat(typecheck): Implement automatic generic type parameter inference
- :sparkles: feat(typecheck): Add structured generic representation MonoType::Generic
- feat: Wire CLI REPL commands into SessionREPL

---

## ♻️ Refactoring & Optimization

- :recycle: refactor(backends): Remove tui_repl module, rewrite as SessionREPL
- :recycle: refactor(typecheck): Introduce VarInfo in scope variable storage to track mutability
- :recycle: refactor(typecheck): Separate primitive value type from Dup shallow-copy semantics

---

## 🐛 Bug Fixes

- :bug: fix(repl): Configure default REPL history, fix shell evaluate_code
- :bug: fix(repl): Register completer and fix multi-line input
- :bug: fix(repl): Remove extra semicolons in wrap_code to preserve expression values

---

## ⚡ Performance Optimization

- :zap: perf(types): Add recursion depth limit for const generic evaluation

---

## 🔧 Other Changes

- :wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap, owo-colors
- :white_check_mark: test(typecheck): Add scope VarInfo mutability tests

### Reference Template

For the release document, please refer to the [`release.md`](release.md) template format.

---

### 1. Set Up Commit Template

```bash
# Execute in the project root
git config commit.template .gitmessage.txt
```

### 2. Template File

The format of `.gitmessage.txt` in the project root is as follows:

```
# emoji_code type(scope): subject (Chinese)
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
# 🐛 fix(provider): Fix timer background resume issue
#
# Release format: 🔖 V1.0.0: Release title
```

---

## FAQ

### Q: How to choose the commit type?

- **feat**: User-visible feature changes
- **fix**: Fix issues reported by users
- **docs**: README, comments, and other documentation
- **chore**: Dependency updates, configuration files
- **refactor**: Code optimization that does not change behavior

### Q: When should commits be split?

- Each commit should do **one thing**
- Related features are committed together, unrelated ones are separated
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [Complete Emoji List](#complete-emoji-reference)
- [release.md](release.md) - Release template

---

> 💡 **Tip**: Keep commits atomic and clearly described to make code review and history navigation
> more efficient!
