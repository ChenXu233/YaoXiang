# Commit Submission Guidelines

This document defines the Git commit specification for the YaoXiang project, aiming to keep the commit history clear, readable, and easy to understand.

---

## Table of Contents

- [Commit Format](#commit-format)
- [Commit Types](#commit-types)
- [Complete Emoji Reference](#complete-emoji-reference)
- [Scopes](#scopes)
- [Version Management](#version-management)
- [Message Specification](#message-specification)
- [Language Specification](#language-specification)
- [🔖 Release Commits](#-release-commits)
- [Examples](#examples)
- [Using Commit Template](#using-commit-template)
- [FAQ](#faq)

---

## Commit Format

**Very important!!!!!!!!Do not forget!!!!!!!!!**
All commit messages follow this format:

```
:emoji code: type(scope): subject (in English)

[optional body]

[optional footer]
```

> ⚠️ **Important**: You must use **emoji codes** (e.g., `:sparkles:`) instead of directly inputting emoji characters.
>
> **It is recommended to use English for commit messages**, maintaining consistency in team communication.

### Components

| Component | Description | Required |
|-----------|-------------|----------|
| emoji code | Emoji symbol identifying commit type | ✅ |
| type | Commit type | ✅ |
| scope | Affected scope | ✅ |
| subject | Brief description (in English, no more than 50 characters) | ✅ |
| body | Detailed explanation (optional) | ❌ |
| footer | Breaking changes or issue closure (optional) | ❌ |

---

## Commit Types

| emoji code | type | Description |
|-----------|------|-------------|
| :sparkles: | feat | New feature |
| :bug: | fix | Bug fix |
| :memo: | docs | Documentation changes only |
| :lipstick: | style | Code formatting (no functional changes) |
| :recycle: | refactor | Code refactoring |
| :zap: | perf | Performance improvement |
| :white_check_mark: | test | Adding or modifying tests |
| :wrench: | chore | Build tool or auxiliary tool changes |
| :building_construction: | build | Build system changes |
| :rocket: | ci | CI configuration changes |

---

## Complete Emoji Reference

The following is the complete emoji list consistent with the gitmoji project. Select the appropriate emoji based on commit content:

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
| 👌 | `:ok_hand:` | Update code due to code review |
| ♿️ | `:wheelchair:` | Improve accessibility |
| 💡 | `:bulb:` | Document source code |
| 🍻 | `:beers:` | Write code while drinking |
| 💬 | `:speech_balloon:` | Update text and literals |
| 🗃️ | `:card_file_box:` | Perform database related changes |
| 🔊 | `:loud_sound:` | Add logs |
| 🔇 | `:mute:` | Remove logs |
| 👥 | `:busts_in_silhouette:` | Add contributors |
| 🚸 | `:children_crossing:` | Improve UX/navigation |
| 🏗️ | `:building_construction:` | Make architectural changes |
| 📱 | `:iphone:` | Work on responsive design |
| 🤡 | `:clown_face:` | Mock things |
| 🥚 | `:egg:` | Add an easter egg |
| 🙈 | `:see_no_evil:` | Add or update .gitignore |
| 📸 | `:camera_flash:` | Add or update snapshots |

---

## Scopes

Based on the project structure, the following scopes are recommended:

### Code Scopes

| Scope | Description |
|--------|-------------|
| `frontend` | Frontend module: Lexical analysis, syntax parsing, type checking |
| `parser` | Syntax parser |
| `lexer` | Lexical analyzer |
| `typecheck` | Type checking |
| `middle` | Middle layer: IR, Optimizer |
| `codegen` | Code generator |
| `monomorphize` | Monomorphization handling |
| `lifetime` | Lifetime analysis |
| `vm` | Virtual machine: Instruction execution, Stack frames, Opcodes |
| `executor` | Executor |
| `frames` | Stack frame management |
| `instructions` | Instruction set |
| `runtime` | Runtime: Memory management, Scheduler |
| `memory` | Memory management |
| `scheduler` | Task scheduling |
| `std` | Standard library |
| `concurrent` | Concurrency library |
| `io` | IO library |
| `net` | Network library |
| `util` | Utility library: Diagnostics, Cache, Span |
| `cache` | Cache management |
| `diagnostic` | Diagnostic information |

### Documentation Scopes

| Scope | Description |
|--------|-------------|
| `docs` | General documentation updates |
| `architecture` | Architecture design documents |
| `design` | Language design specifications |
| `plan` | Implementation plan documents |
| `guides` | Guide documents |
| `tutorial` | Tutorial documents |
| `examples` | Example code |

### Other Scopes

| Scope | Description |
|--------|-------------|
| `build` | Build system, dependency management |
| `ci` | CI/CD configuration |
| `test` | Testing related |
| `chore` | Miscellaneous tasks |
| `release` | Release related |
| `meta` | Project meta configuration (e.g., .claude, cargo config) |

---

## Message Specification

### Version Management

**You must bump the version number before each commit**:

| Version Type | Update Location | Description |
|--------------|-----------------|-------------|
| **major** | `pubspec.yaml` (version) + `release_v*.md` | Major update, breaking API changes |
| **minor** | `pubspec.yaml` (version) | New features, backward compatible |
| **patch** | `pubspec.yaml` (version) | Bug fixes, backward compatible |

### Version Number Format

Semantic versioning `MAJOR.MINOR.PATCH` is used:

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
# 1. After modifying code, bump the version first
# Use semantic_release tool to automatically manage versions and Changelog
npx semantic-release

# Or manually update the version
# Edit the version field in pubspec.yaml

# 2. Commit code (version changes will be automatically generated on next release)
git add .
git commit -m ":tada: Release v1.0.0"
git push
```

> 💡 Version bump and Changelog generation are done automatically by CI. When committing, just ensure the code changes include version updates.

---

## Message Specification

### Language Specification

**It is recommended to use English for commit messages**, maintaining consistency in team communication.

- Subject in English, concise and clear
- Body can use English for detailed explanation
- If there are special technical terms, English can be retained

### Subject

- In English, concise and clear
- No more than 50 characters
- No period at the end

### Body

- Explain the reasons and methods for changes in detail
- Each line no more than 72 characters
- Use - or * to list points

### Footer

- **Breaking Changes**: Start with `BREAKING CHANGE:`
- **Closing Issues**: Use `Closes #123` or `Fixes #456`

---

## Examples

### ✨ feat - New Feature

```
:sparkles: feat(db): add batch delete todo functionality

Implemented batch delete todos feature:
- Added batchDelete method in TodoRepository
- Added delete confirmation dialog
- Updated UI to support multi-select operations

Closes #42
```

### 🐛 fix - Bug Fix

```
:bug: fix(provider): fix pomodoro timer not resuming from background

When the app resumes from background, the pomodoro timer cannot continue.
Added state recovery logic in initState.

Fixes #128
```

### 📝 docs - Documentation Update

```
:memo: docs: update README new feature description

Added the following sections:
- Focus mode
- Data statistics
- Background music
```

### ♻️ refactor - Refactoring

```
:recycle: refactor(ui): extract common glass container component

Created reusable GlassContainer component,
reducing code duplication across multiple screens.
```

### ⚡️ perf - Performance Optimization

```
:zap: perf(db): optimize completed todo query performance

Added index on completed_at field,
using WHERE clause to filter completed todos instead of filtering in memory.

Before: 45ms
After: 12ms
```

### ✅ test - Testing

```
:white_check_mark: test(provider): add TodoProvider unit tests

Covered scenarios:
- Add todo
- Toggle completion status
- Delete todo
```

### 🔧 chore - Miscellaneous

```
:wrench: chore: update Flutter version to 3.19.0

Raised minimum Flutter version requirement and updated compatible dependencies.
```

### 🚀 ci - CI Configuration

```
:rocket: ci: add GitHub Actions test workflow

Created workflow with the following steps:
- Unit tests
- Integration tests
- Code coverage
```

### 💄 style - Formatting

```
:lipstick: style(todo_item): format code using dart fix

Applied automatic formatting fixes,
maintaining code style consistency.
```

---

---

## 🔖 Release Commits

When the current commit is a **Release**, you must follow these specifications:

### Release Commit Format

```
:bookmark: V<version number>: <release title>

## 📦 Version Information

**Release Date:** YYYY-MM-DD

**Version:** <old version> → <new version>

---

## ✨ New Features

### <Feature Module>
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

1. **Message Header**: Must use `:bookmark:` + `V<version number>` format
2. **Version Number**: Follow semantic versioning specification
3. **Content Completeness**: Must include all commit content introductions since the last release
4. **Categorized by Type**: Organize by types like `feat`, `fix`, `refactor`, `chore`, etc.
5. **Commit History**: List all related commit hashes and descriptions

### Release Example

```
:bookmark: V0.8.0: add task statistics and data analysis features

## 📦 Version Information

**Release Date:** 2025-01-15

**Version:** 0.7.0 → 0.8.0

---

## ✨ New Features

### Statistics Features
- :sparkles: feat(statistics): add focus duration statistics page
- :sparkles: feat(statistics): add daily/weekly data visualization charts

### Todo Enhancements
- :sparkles: feat(todo): add task priority filtering feature

---

## ♻️ Refactoring & Optimization

- :recycle: refactor(db): optimize database query performance
- :recycle: refactor(provider): refactor state management logic

---

## 🐛 Bug Fixes

- :bug: fix(todo): fix task list scrolling lag issue

---

## 🔧 Other Changes

- :wrench: chore: update dependency versions to latest stable
- :memo: docs: update README installation instructions

---

## 📦 New Files

- `lib/screens/statistics/statistics.dart` - Statistics page
- `lib/widgets/chart/data_chart.dart` - Chart component

---

## 📝 Commit History

| Commit | Description |
|:---:|------|
| `abc1234` | :bookmark: V0.8.0 |
| `def5678` | :sparkles: feat(statistics): add focus duration statistics |
| `ghi9012` | :sparkles: feat(todo): add priority filtering |
| `jkl3456` | :recycle: refactor(db): optimize queries |
| `mno7890` | :bug: fix(todo): fix scrolling lag |
```

### How to Get Commit History

```bash
# View all commits since last release
git log --oneline <last release commit>..HEAD

# Or view last N commits
git log --oneline -20
```

### Reference Template

For release documentation, please refer to the [`release.md`](release.md) template format.

---

### 1. Set Commit Template

```bash
# Execute in project root directory
git config commit.template .gitmessage.txt
```

### 2. Template File

The format of the `.gitmessage.txt` file in the project root directory is as follows:

```
# emoji code type(scope): subject (in English)
#
# body content (optional)
#
# footer (optional)
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: core, db, ui, screen, widget, provider, repo, i18n, router, dep
#
# Examples:
# ✨ feat(db): add batch delete todo functionality
# 🐛 fix(provider): fix timer resume from background issue
#
# Release format: 🔖 V1.0.0: Release title
```

---

## FAQ

### Q: How to choose a commit type?

- **feat**: Visible functional changes for users
- **fix**: Fix issues reported by users
- **docs**: README, comments, and other documentation
- **chore**: Dependency updates, configuration files
- **refactor**: Code optimization without changing behavior

### Q: When should commits be split?

- Each commit should only do **one thing**
- Related features should be committed together, unrelated ones separately
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Complete emoji list
- [release.md](release.md) - Release template

---

> 💡 **Tip**: Keep commits atomic and descriptive, making code review and traceability more efficient!