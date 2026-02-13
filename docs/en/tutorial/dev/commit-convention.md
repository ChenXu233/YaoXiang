# Commit Guide

This document defines Git commit conventions for the YaoXiang project, aiming to keep commit history clear, readable, and easy to understand.

---

## Table of Contents

- [Commit Format](#commit-format)
- [Commit Types](#commit-types)
- [Complete Emoji Reference](#complete-emoji-reference)
- [Scopes](#scopes)
- [Version Management](#version-management)
- [Message Specifications](#message-specifications)
- [Language Specifications](#language-specifications)
- [🔖 Release Commits](#-release-commits)
- [Examples](#examples)
- [Using Commit Template](#using-commit-template)
- [FAQ](#faq)

---

## Commit Format

**Very Important!!!!!! Don't forget!!!!!**
All commit messages follow this format:

```
:emoji_code: type(scope): subject (in English)

[optional body content]

[optional footer]
```

> ⚠️ **Important**: Must use **emoji codes** (like `:sparkles:`) instead of directly entering emoji characters.
>
> **English commit messages are recommended** to maintain consistency for international collaboration.

### Components

| Part | Description | Required |
|------|-------------|----------|
| emoji code | Emoji symbol identifying commit type | ✅ |
| type | Commit type | ✅ |
| scope | Affected scope | ✅ |
| subject | Brief description (English, max 50 characters) | ✅ |
| body | Detailed explanation (optional) | ❌ |
| footer | Breaking changes or issue closure (optional) | ❌ |

---

## Commit Types

| emoji code | type | Description |
|------------|------|-------------|
| :sparkles: | feat | New feature |
| :bug: | fix | Bug fix |
| :memo: | docs | Documentation changes only |
| :lipstick: | style | Code formatting (no functional changes) |
| :recycle: | refactor | Code refactoring |
| :zap: | perf | Performance optimization |
| :white_check_mark: | test | Add or modify tests |
| :wrench: | chore | Build tools, auxiliary tools changes |
| :building_construction: | build | Build system changes |
| :rocket: | ci | CI configuration changes |

---

## Complete Emoji Reference

The following is a complete emoji list consistent with the gitmoji project, choose the appropriate emoji based on commit content:

| emoji | emoji code | commit description |
|-------|------------|-------------------|
| 🎨 | `:art:` | Improve code structure/format |
| ⚡️ | `:zap:` / `:racehorse:` | Improve performance |
| 🔥 | `:fire:` | Remove code or files |
| 🐛 | `:bug:` | Fix a bug |
| 🚑 | `:ambulance:` | Critical hotfix |
| ✨ | `:sparkles:` | Introduce new features |
| 📝 | `:memo:` | Write documentation |
| 🚀 | `:rocket:` | Deploy features |
| 💄 | `:lipstick:` | Update UI and style files |
| 🎉 | `:tada:` | Initial commit |
| ✅ | `:white_check_mark:` | Add tests |
| 🔒 | `:lock:` | Fix security issues |
| 🍎 | `:apple:` | Fix something on macOS |
| 🐧 | `:penguin:` | Fix something on Linux |
| 🏁 | `:checkered_flag:` | Fix something on Windows |
| 🤖 | `:robot:` | Fix something on Android |
| 🍏 | `:green_apple:` | Fix something on iOS |
| 🔖 | `:bookmark:` | Release/version tag |
| 🚨 | `:rotating_light:` | Remove linter warnings |
| 🚧 | `:construction:` | Work in progress |
| 💚 | `:green_heart:` | Fix CI build |
| ⬇️ | `:arrow_down:` | Downgrade dependencies |
| ⬆️ | `:arrow_up:` | Upgrade dependencies |
| 📌 | `:pushpin:` | Pin dependencies to specific versions |
| 👷 | `:construction_worker:` | Add CI build system |
| 📈 | `:chart_with_upwards_trend:` | Add analytics or tracking code |
| ♻️ | `:recycle:` | Refactor code |
| 🔨 | `:hammer:` | Major refactoring |
| ➖ | `:heavy_minus_sign:` | Remove a dependency |
| 🐳 | `:whale:` | Docker related work |
| ➕ | `:heavy_plus_sign:` | Add a dependency |
| 🔧 | `:wrench:` | Change configuration files |
| 🌐 | `:globe_with_meridians:` | Internationalization and localization |
| ✏️ | `:pencil2:` | Fix typos |
| 💩 | `:hankey:` | Write bad code that needs improvement |
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
| 🍻 | `:beers:` | Write code with pleasure |
| 💬 | `:speech_balloon:` | Update text and literals |
| 🗃️ | `:card_file_box:` | Perform database-related changes |
| 🔊 | `:loud_sound:` | Add logs |
| 🔇 | `:mute:` | Remove logs |
| 👥 | `:busts_in_silhouette:` | Add contributors |
| 🚸 | `:children_crossing:` | Improve user experience/usability |
| 🏗️ | `:building_construction:` | Make architectural changes |
| 📱 | `:iphone:` | Work on responsive design |
| 🤡 | `:clown_face:` | Mock things |
| 🥚 | `:egg:` | Add an Easter egg |
| 🙈 | `:see_no_evil:` | Add or update .gitignore file |
| 📸 | `:camera_flash:` | Add or update snapshots |

---

## Scopes

Based on project structure, the following scopes are recommended:

### Code Scopes

| Scope | Description |
|-------|-------------|
| `frontend` | Frontend module: lexing, parsing, type checking |
| `parser` | Syntax parser |
| `lexer` | Lexical analyzer |
| `typecheck` | Type checking |
| `middle` | Middle layer: IR, optimizer |
| `codegen` | Code generator |
| `monomorphize` | Monomorphization |
| `lifetime` | Lifetime analysis |
| `vm` | Virtual machine: instruction execution, stack frames, opcodes |
| `executor` | Executor |
| `frames` | Stack frame management |
| `instructions` | Instruction set |
| `runtime` | Runtime: memory management, scheduler |
| `memory` | Memory management |
| `scheduler` | Task scheduling |
| `std` | Standard library |
| `concurrent` | Concurrency library |
| `io` | IO library |
| `net` | Network library |
| `util` | Utilities: diagnostics, cache, Span |
| `cache` | Cache management |
| `diagnostic` | Diagnostic information |

### Documentation Scopes

| Scope | Description |
|-------|-------------|
| `docs` | General documentation update |
| `architecture` | Architecture design documents |
| `design` | Language design specifications |
| `plan` | Implementation plan documents |
| `guides` | Guide documents |
| `tutorial` | Tutorial documents |
| `examples` | Example code |

### Other Scopes

| Scope | Description |
|-------|-------------|
| `build` | Build system, dependency management |
| `ci` | CI/CD configuration |
| `test` | Testing related |
| `chore` | Miscellaneous tasks |
| `release` | Release related |
| `meta` | Project meta-configuration (e.g., .claude, cargo config) |

---

## Version Management

**You must bump the version number before each commit:**

| Version Type | Update Location | Description |
|--------------|-----------------|-------------|
| **major** | `Cargo.toml` (version) + `release_v*.md` | Major update, breaking API changes |
| **minor** | `Cargo.toml` (version) | New features, backward compatible |
| **patch** | `Cargo.toml` (version) | Bug fixes, backward compatible |

### Version Number Format

Semantic versioning `MAJOR.MINOR.PATCH`:

```
# Major version (breaking changes)
1.0.0 -> 2.0.0

# Minor version (new features)
1.0.0 -> 1.1.0

# Patch version (bug fixes)
1.0.0 -> 1.0.1
```

### Commit Workflow

```bash
# 1. After modifying code, bump version first
# Use semantic_release tool to automatically manage version and Changelog
npx semantic-release

# Or update version manually
# Edit version field in Cargo.toml

# 2. Commit code (version changes will be automatically generated on next release)
git add .
git commit -m ":tada: Release v1.0.0"
git push
```

> 💡 Version bump and Changelog generation are done automatically by CI. When committing, just ensure code changes include version updates.

---

## Message Specifications

### Language Specifications

**English commit messages are recommended** for international collaboration.

- Subject in English, concise and clear
- Body can use English for detailed explanation
- Special technical terms can be kept in English

### Subject

- Use English, concise and clear
- No more than 50 characters
- No period at the end

### Body

- Detailed explanation of changes and reasons
- Each line no more than 72 characters
- Use - or * for bullet points

### Footer

- **Breaking Changes**: Start with `BREAKING CHANGE:`
- **Close Issue**: Use `Close #123` or `Fix #456`

---

## Examples

### ✨ feat - New Feature

```
:sparkles: feat(db): Add batch delete todo feature

Implement batch delete todo items feature:
- Add batchDelete method in TodoRepository
- Add delete confirmation dialog
- Update UI to support multi-select operations

Close #42
```

### 🐛 fix - Bug Fix

```
:bug: fix(provider): Fix pomodoro timer not recovering in background

When the app resumes from background, the pomodoro timer cannot continue timing.
Added state recovery logic in initState.

Fix #128
```

### 📝 docs - Documentation

```
:memo: docs: Update README with new features

Added the following sections:
- Focus mode
- Data statistics
- Background music
```

### ♻️ refactor - Refactoring

```
:recycle: refactor(ui): Extract common glass container component

Create reusable GlassContainer component,
reduce code duplication across multiple screens.
```

### ⚡️ perf - Performance Optimization

```
:zap: perf(db): Optimize completed todo query performance

Added index on completed_at field,
use WHERE clause to filter completed todos instead of filtering in memory.

Before optimization: 45ms
After optimization: 12ms
```

### ✅ test - Testing

```
:white_check_mark: test(provider): Add TodoProvider unit tests

Covered scenarios:
- Add todo
- Toggle completion status
- Delete todo
```

### 🔧 chore - Miscellaneous

```
:wrench: chore: Update Flutter version to 3.19.0

Increase minimum Flutter version requirement and update compatible dependencies.
```

### 🚀 ci - CI Configuration

```
:rocket: ci: Add GitHub Actions test workflow

Create workflow with following steps:
- Unit tests
- Integration tests
- Code coverage
```

### 💄 style - Formatting

```
:lipstick: style(todo_item): Format code using dart fix

Apply automatic formatting fixes,
maintain code style consistency.
```

---

## 🔖 Release Commits

When a commit is a **Release**, it must follow these conventions:

### Release Commit Format

```
:bookmark: V<version_number>: <release_title>

## 📦 Version Information

**Release Date:** YYYY-MM-DD

**Version:** <old_version> → <new_version>

---

## ✨ New Features

### <feature_module>
- :sparkles: feat(<scope>): <feature_description>

---

## ♻️ Refactoring and Optimization

- :recycle: refactor(<scope>): <refactoring_description>

---

## 🐛 Bug Fixes

- :bug: fix(<scope>): <fix_description>

---

## 🔧 Other Changes

- :wrench: chore: <change_description>

---

## 📦 New Files Added

- `<file_path>` - <file_description>

---

## 📝 Commit History

| Commit | Description |
|:---:|------|
| `<hash>` | :bookmark: V<version_number> |
| `<hash>` | <commit_message> |
```

### Release Requirements

1. **Message Header**: Must use `:bookmark:` + `V<version_number>` format
2. **Version Number**: Follow semantic versioning
3. **Content Completeness**: Must include introduction of **all commits** since last release
4. **Categorization by Type**: Organize by `feat`, `fix`, `refactor`, `chore`, etc.
5. **Commit History**: List all related commit hashes and descriptions

### Release Example

```
:bookmark: V0.8.0: Add task statistics and data analysis features

## 📦 Version Information

**Release Date:** 2025-01-15

**Version:** 0.7.0 → 0.8.0

---

## ✨ New Features

### Statistics Feature
- :sparkles: feat(statistics): Add focus time statistics page
- :sparkles: feat(statistics): Add daily/weekly data visualization charts

### Todo Enhancement
- :sparkles: feat(todo): Add task priority filtering

---

## ♻️ Refactoring and Optimization

- :recycle: refactor(db): Optimize database query performance
- :recycle: refactor(provider): Refactor state management logic

---

## 🐛 Bug Fixes

- :bug: fix(todo): Fix task list scrolling lag

---

## 🔧 Other Changes

- :wrench: chore: Update dependency versions to latest stable
- :memo: docs: Update README installation instructions

---

## 📦 New Files Added

- `lib/screens/statistics/statistics.dart` - Statistics page
- `lib/widgets/chart/data_chart.dart` - Chart component

---

## 📝 Commit History

| Commit | Description |
|:---:|------|
| `abc1234` | :bookmark: V0.8.0 |
| `def5678` | :sparkles: feat(statistics): Add focus time statistics |
| `ghi9012` | :sparkles: feat(todo): Add priority filtering |
| `jkl3456` | :recycle: refactor(db): Optimize queries |
| `mno7890` | :bug: fix(todo): Fix scrolling lag |
```

### How to Get Commit History

```bash
# View all commits since last release
git log --oneline <last_release_commit>..HEAD

# Or view recent N commits
git log --oneline -20
```

### Reference Template

For release documentation, please refer to [`release.md`](release.md) template format.

---

## Using Commit Template

### 1. Set Commit Template

```bash
# Execute in project root
git config commit.template .gitmessage.txt
```

### 2. Template File

The `.gitmessage.txt` file in project root has the following format:

```
# emoji_code type(scope): subject (in English)
#
# Body content (optional)
#
# Footer (optional)
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: frontend, parser, lexer, typecheck, middle, codegen, vm, runtime, std, docs, etc.
#
# Examples:
# ✨ feat(db): Add batch delete todo feature
# 🐛 fix(provider): Fix timer background recovery
#
# Release format: 🔖 V1.0.0: Release title
```

---

## FAQ

### Q: How to choose commit type?

- **feat**: Functional changes visible to users
- **fix**: Fix issues reported by users
- **docs**: README, comments, etc.
- **chore**: Dependency updates, configuration files
- **refactor**: Code optimization without behavior changes

### Q: When should commits be split?

- Each commit should do **only one thing**
- Related features together, unrelated separated
- Follow Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Complete emoji list
- [release.md](release.md) - Release template

---

> 💡 **Tip**: Keep commits atomic and descriptions clear to make code review and追溯 more efficient!
