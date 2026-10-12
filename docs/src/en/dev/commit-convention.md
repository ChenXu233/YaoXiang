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
- [🔖 Release Commits](#-release-commits)
- [Examples](#examples)
- [Using the Commit Template](#using-the-commit-template)
- [FAQ](#faq)

---

## Commit Format

**Very important!!!!!! Don't forget!!!** All commit messages must follow this format:

```
:emoji_code: type(scope): subject (in Chinese)

[optional body]

[optional footer]
```

> ⚠️ **Important**: You must use the **emoji code** (e.g. `:sparkles:`) rather than typing the emoji
> character directly.
>
> **Chinese commit messages are recommended** to keep team communication consistent.

### Components

| Part       | Description                                           | Required |
| ---------- | ----------------------------------------------------- | -------- |
| emoji_code | Emoji symbol identifying the commit type              | ✅       |
| type       | Commit type                                           | ✅       |
| scope      | Area affected                                         | ✅       |
| subject    | Short description (in Chinese, no more than 50 chars) | ✅       |
| body       | Detailed explanation (optional)                       | ❌       |
| footer     | Breaking change or issue closure (optional)           | ❌       |

---

## Commit Types

| emoji_code                | type     | Description                            |
| ------------------------- | -------- | -------------------------------------- |
| `:sparkles:`              | feat     | New feature                            |
| `:bug:`                   | fix      | Bug fix                                |
| `:memo:`                  | docs     | Documentation-only changes             |
| `:lipstick:`              | style    | Code formatting (no functional impact) |
| `:recycle:`               | refactor | Code refactoring                       |
| `:zap:`                   | perf     | Performance optimization               |
| `:white_check_mark:`      | test     | Add or modify tests                    |
| `:wrench:`                | chore    | Build tools, helper tool changes       |
| `:building_construction:` | build    | Build system changes                   |
| `:rocket:`                | ci       | CI configuration changes               |

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

The scope is based on the repository directory structure. **You must use the following defined
scopes**:

### Top-level Modules

| Scope       | Corresponding Directory              | Description                                                                            |
| ----------- | ------------------------------------ | -------------------------------------------------------------------------------------- |
| `frontend`  | `src/frontend/`                      | Frontend: lexical analysis, syntax parsing, type checking                              |
| `middle`    | `src/middle/`                        | Middle layer: IR, optimization, monomorphization                                       |
| `backends`  | `src/backends/`                      | Backend: interpreter, runtime, REPL                                                    |
| `std`       | `src/std/`                           | Standard library                                                                       |
| `formatter` | `src/formatter/`                     | Code formatter                                                                         |
| `lsp`       | `src/lsp/`                           | Language Server Protocol                                                               |
| `package`   | `src/package/`                       | Package manager                                                                        |
| `util`      | `src/util/`                          | Utility library: diagnostics, cache, i18n                                              |
| `proof`     | `src/frontend/core/typecheck/proof/` | Proof and verification support (promoted to top-level domain by RFC-039 final state)   |
| `driver`    | `src/driver/`                        | L1 orchestration layer: unified Driver and phase contracts (established by RFC-039 P4) |

### Frontend Submodules

| Scope       | Corresponding Directory        | Description            |
| ----------- | ------------------------------ | ---------------------- |
| `parser`    | `src/frontend/core/parser/`    | Syntax parser          |
| `lexer`     | `src/frontend/core/lexer/`     | Lexical analyzer       |
| `typecheck` | `src/frontend/core/typecheck/` | Type checking          |
| `types`     | `src/frontend/core/types/`     | Type system definition |

### Middle Layer Submodules

| Scope          | Corresponding Directory       | Description                                                              |
| -------------- | ----------------------------- | ------------------------------------------------------------------------ |
| `codegen`      | `src/middle/passes/codegen/`  | Code generation (bytecode)                                               |
| `monomorphize` | `src/middle/passes/mono/`     | Monomorphization processing (directory rename pending RFC-039 P6 ruling) |
| `lifetime`     | `src/middle/passes/lifetime/` | Lifetime analysis                                                        |

### Backend Submodules

| Scope     | Corresponding Directory | Description                   |
| --------- | ----------------------- | ----------------------------- |
| `repl`    | `src/repl/`             | REPL interactive command line |
| `runtime` | `src/backends/runtime/` | Runtime execution engine      |

### Documentation Scopes

| Scope    | Description                                           |
| -------- | ----------------------------------------------------- |
| `docs`   | General documentation updates                         |
| `design` | Language design specifications (docs/src/dev/design/) |
| `plan`   | Implementation plan documents                         |
| `rfc`    | RFC documents (docs/src/rfc/)                         |

### Other Scopes

| Scope      | Description                                            |
| ---------- | ------------------------------------------------------ |
| `build`    | Build system, Cargo configuration                      |
| `ci`       | CI/CD configuration (GitHub Actions)                   |
| `test`     | Test-related                                           |
| `release`  | Release-related                                        |
| `meta`     | Project meta configuration (.claude, .gitignore, etc.) |
| `deps`     | Dependency upgrades (same convention as dependabot)    |
| `examples` | Examples directory `examples/`                         |
| `vscode`   | VS Code extension `vscode-extension/`                  |
| `benches`  | Benchmarks `benches/`                                  |

---

## Message Conventions

### Version Management

The version number is defined in the `version` field of `Cargo.toml` at the project root:

```toml
[package]
version = "0.7.2"
```

Semantic versioning `MAJOR.MINOR.PATCH` is used:

| Version Type | Description                           | Example       |
| ------------ | ------------------------------------- | ------------- |
| **major**    | Major update, incompatible API change | 0.7.2 → 1.0.0 |
| **minor**    | New feature, backward compatible      | 0.7.2 → 0.8.0 |
| **patch**    | Bug fix, backward compatible          | 0.7.2 → 0.7.3 |

> ⚠️ When releasing, **update the `Cargo.toml` version on the `dev` branch**, then merge to `main`
> via PR. CI will automatically create the tag and Release. **Do not push tags manually**, otherwise
> CI will skip the release workflow.

---

## CI Release Workflow

Releases are performed automatically by GitHub Actions (`dist-release.yml`). The workflow is as
follows:

```
1. Update the version field in Cargo.toml on the dev branch
2. cargo build to update Cargo.lock
3. Commit following the release format (see 🔖 Release Commits below)
   - The commit message must include all changes since the last release (i.e. the full PR content)
4. Create a PR from dev to main
5. Merge the PR into main
6. The gate job automatically detects:
   - Read the Cargo.toml version number → "v{version}"
   - Check whether the tag already exists
   - Does not exist → trigger the full release workflow
   - Exists → skip (no duplicate release)
7. Gate: security audit + fmt/clippy/unit tests/doc tests all pass
8. After all pass: create and push the tag
9. Post-tag build and publish:
   - cargo-dist cross-platform build (5 platforms for Linux/Windows/macOS) + reorganized packages (bin/ + lib/) + wasm + Inno setup wizard
   - Publish GitHub Release (body generated by generate-commit-list.ts); apt repository published separately via GitHub Pages
```

### Key Rules

| Rule                                       | Description                                                                                                                                                                                       |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Do not push tags manually**              | The release only recognizes the gate check on push to main; manually pushed tags will not trigger the release (for re-releasing, use `dist-release.yml`'s `workflow_dispatch` to specify the tag) |
| **Bump the version on dev**                | The release commit is done on dev, then merged to main via PR                                                                                                                                     |
| **Release commit includes full changelog** | The commit message must include all changes in this release, since it is the source of the PR description                                                                                         |
| **Do not merge main back into dev**        | After the PR is merged, dev will be synced automatically; no reverse merge is needed                                                                                                              |

---

## Message Conventions

### Language Conventions

**Chinese commit messages are recommended** to keep team communication consistent.

- Subject in Chinese, concise and clear
- Body in Chinese for detailed explanation
- Special technical terms may be kept in English

### Subject

- Use Chinese, concise and clear
- No more than 50 characters
- No period at the end

### Body

- Explain in detail the reason for and approach of the change
- No more than 72 characters per line
- Use `-` or `*` to list key points

### Footer

- **Breaking change**: Start with `BREAKING CHANGE:`
- **Close issue**: Use `关闭 #123` (Close #123) or `修复 #456` (Fix #456)

---

## Examples

### ✨ feat - New Feature

```
:sparkles: feat(parser): Add closure syntax parsing support

Implement closure expression parsing:
- Support |args| body shorthand syntax
- Support move-semantic capture
- Add closure type inference

关闭 #42
```

### 🐛 fix - Bug Fix

```
:bug: fix(repl): Fix completer failure on multi-line input

SessionREPL did not register the completer correctly in multi-line mode,
causing Tab completion to fail to trigger.

修复 #128
```

### 📝 docs - Documentation Update

```
:memo: docs(design): Update ownership model and type system specifications

Sync the latest design changes from RFC-009 and RFC-011.
```

### ♻️ refactor - Refactor

```
:recycle: refactor(typecheck): Separate primitive value types from Dup shallow-copy semantics

Decouple value types and copy semantics in MonoType,
eliminating special cases in match branches.
```

### ⚡️ perf - Performance Optimization

```
:zap: perf(types): Optimize const generic evaluation performance

Add a depth limit for recursive evaluation (default 128),
to prevent stack overflow from maliciously crafted type expressions.
```

### ✅ test - Test

```
:white_check_mark: test(typecheck): Add scope VarInfo mutability tests

Coverage scenarios:
- Read-only access of immutable bindings
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
:rocket: ci: Fix nightly build using too-old Rust version

Update RUST_TOOLCHAIN from 1.91.0 to 1.96.0,
to match the rust-version requirement in Cargo.toml.
```

### 💄 style - Formatting Adjustment

```
:lipstick: style(frontend): Apply cargo fmt formatting

Unify the line-breaking style of function signatures.
```

---

---

## 🔖 Release Commits

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

- :recycle: refactor(<scope>): <refactor description>

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
2. **Version number**: Follow semantic versioning
3. **Content completeness**: Must include **all commit** descriptions since the last release
4. **Categorize by type**: Organize by `feat`, `fix`, `refactor`, `chore`, etc.

### Release Example

```

:bookmark: V0.7.2: REPL rewrite and type system improvements

## 📦 Version Information

**Release Date:** 2026-06-01

**Version:** 0.7.1 → 0.7.2

---

## ✨ New Features

- :sparkles: feat(typecheck): Implement automatic inference of generic type parameters
- :sparkles: feat(typecheck): Add structured generic representation for MonoType::Generic
- feat: Wire CLI REPL commands into SessionREPL

---

## ♻️ Refactoring & Optimization

- :recycle: refactor(backends): Remove tui_repl module, rewrite as SessionREPL
- :recycle: refactor(typecheck): Introduce VarInfo to track mutability in scope variable storage
- :recycle: refactor(typecheck): Separate primitive value types from Dup shallow-copy semantics

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

Please refer to the [`release.md`](release.md) template for release documents.

---

### 1. Set Up the Commit Template

```bash
# Run in the project root
git config commit.template .gitmessage.txt
```

### 2. Template File

The `.gitmessage.txt` file at the project root has the following format:

```
# emoji_code type(scope): subject (in Chinese)
#
# Body content (optional)
#
# Footer (optional)
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: frontend, parser, lexer, typecheck, types, middle, codegen,
#         monomorphize, lifetime, backends, repl, runtime, proof,
#         std, formatter, lsp, package, util, docs, design, plan, rfc,
#         build, ci, test, release, meta, deps, examples, vscode, benches
#
# Example:
# ✨ feat(db): Add batch delete todo feature
# 🐛 fix(provider): Fix timer background recovery issue
#
# Release format: 🔖 V1.0.0: Release title
```

---

## FAQ

### Q: How to choose the commit type?

- **feat**: A feature change that users can see
- **fix**: Fix a user-reported issue
- **docs**: README, comments, and other documentation
- **chore**: Dependency updates, configuration files
- **refactor**: Code optimization that does not change behavior

### Q: When should a commit be split?

- Each commit should do **one thing**
- Commit related features together, separate unrelated ones
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [Full Emoji List](#complete-emoji-reference)
- [release.md](release.md) - Release template

---

> 💡 **Tip**: Keep commits atomic and well-described, making code review and traceability more
> efficient!
