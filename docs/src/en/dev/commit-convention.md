# Commit Submission Guidelines

This document defines the Git commit specification for the YaoXiang project, aimed at keeping the commit history clear, readable, and easy to understand.

---

## Table of Contents

- [Commit Format](#commit-format)
- [Commit Types](#commit-types)
- [Complete Emoji Reference](#complete-emoji-reference)
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

**Very important!!!!!!!!Don't forget!!!!!!!!!**
All commit messages follow this format:

```
:emoji code: type(scope): subject (in English)

[Optional body]

[Optional footer]
```

> ⚠️ **Important**: Must use **emoji codes** (e.g., `:sparkles:`) instead of directly inputting emoji characters.
> 
> **English commit messages are recommended** to maintain consistency for international collaboration.

### Components

| Component | Description | Required |
|-----------|-------------|----------|
| emoji code | Emoji identifier for commit type | ✅ |
| type | Commit type | ✅ |
| scope | Affected scope | ✅ |
| subject | Brief description (in English, max 50 characters) | ✅ |
| body | Detailed explanation (optional) | ❌ |
| footer | Breaking changes or issue closing (optional) | ❌ |

---

## Commit Types

| emoji code | type | Description |
|-----------|------|------|
| :sparkles: | feat | New feature |
| :bug: | fix | Bug fix |
| :memo: | docs | Documentation changes only |
| :lipstick: | style | Code formatting (no functional changes) |
| :recycle: | refactor | Code refactoring |
| :zap: | perf | Performance optimization |
| :white_check_mark: | test | Adding or modifying tests |
| :wrench: | chore | Build tool or auxiliary tool changes |
| :building_construction: | build | Build system changes |
| :rocket: | ci | CI configuration changes |

---

## Complete Emoji Reference

The following is the complete emoji list consistent with the gitmoji project. Choose the appropriate emoji based on your commit content:

| emoji | emoji code | commit description |
| :---- | :---------------------------- | :--------------------------- |
| 🎨 | `:art:` | Improve code structure/formatting |
| ⚡️ | `:zap:` / `:racehorse:` | Improve performance |
| 🔥 | `:fire:` | Remove code or files |
| 🐛 | `:bug:` | Fix a bug |
| 🚑 | `:ambulance:` | Critical patch |
| ✨ | `:sparkles:` | Introduce new features |
| 📝 | `:memo:` | Write documentation |
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
| 🔨 | `:hammer:` | Heavy refactoring |
| ➖ | `:heavy_minus_sign:` | Remove a dependency |
| 🐳 | `:whale:` | Work about Docker |
| ➕ | `:heavy_plus_sign:` | Add a dependency |
| 🔧 | `:wrench:` | Manipulate configuration files |
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
| 🗃️ | `:card_file_box:` | Perform database-related changes |
| 🔊 | `:loud_sound:` | Add logging |
| 🔇 | `:mute:` | Remove logging |
| 👥 | `:busts_in_silhouette:` | Add contributors |
| 🚸 | `:children_crossing:` | Improve user experience/usability |
| 🏗️ | `:building_construction:` | Make architectural changes |
| 📱 | `:iphone:` | Work on responsive design |
| 🤡 | `:clown_face:` | Mock things |
| 🥚 | `:egg:` | Add an Easter egg |
| 🙈 | `:see_no_evil:` | Add or update .gitignore |
| 📸 | `:camera_flash:` | Add or update snapshots |

---

## Scopes

Based on the project structure, the following scopes are recommended:

### Code Scopes

| Scope | Description |
|--------|------|
| `frontend` | Frontend module: lexer, parser, type checking |
| `parser` | Parser |
| `lexer` | Lexer |
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
| `util` | Utility library: diagnostics, cache, Span |
| `cache` | Cache management |
| `diagnostic` | Diagnostic information |

### Documentation Scopes

| Scope | Description |
|--------|------|
| `docs` | General documentation updates |
| `architecture` | Architecture design documents |
| `design` | Language design specification |
| `plan` | Implementation plan documents |
| `guides` | Guide documents |
| `tutorial` | Tutorial documents |
| `examples` | Example code |

### Other Scopes

| Scope | Description |
|--------|------|
| `build` | Build system, dependency management |
| `ci` | CI/CD configuration |
| `test` | Test-related |
| `chore` | Miscellaneous tasks |
| `release` | Release-related |
| `meta` | Project meta configuration (e.g., .claude, cargo config) |

---

## Message Guidelines

### Version Management

**You must bump the version number before each commit**:

| Version Type | Update Location | Description |
|----------|----------|------|
| **major** | `pubspec.yaml` (version) + `release_v*.md` | Major update, breaking API changes |
| **minor** | `pubspec.yaml` (version) | New features, backward compatible |
| **patch** | `pubspec.yaml` (version) | Bug fixes, backward compatible |

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

### Commit Flow

```bash
# 1. After modifying code, bump the version first
# Use semantic_release tool to automatically manage version and Changelog
npx semantic-release

# Or manually update the version
# Edit the version field in pubspec.yaml

# 2. Commit code (version changes will be automatically generated on next release)
git add .
git commit -m ":tada: Release v1.0.0"
git push
```

> 💡 Version bump and Changelog generation are done automatically by CI. Just ensure the code changes include version updates when committing.

---

## Message Guidelines

### Language Guidelines

**English commit messages are recommended** to facilitate international collaboration.

- Subject in English, concise and clear
- Body can use English for detailed explanations
- Technical terms can be kept in English if needed

### Subject

- In English, concise and clear
- No more than 50 characters
- No period at the end

### Body

- Explain the reason and method of changes in detail
- Each line no more than 72 characters
- Use - or * for bullet points

### Footer

- **Breaking changes**: Start with `BREAKING CHANGE:`
- **Closing Issue**: Use `Closes #123` or `Fixes #456`

---

## Examples

### ✨ feat - New Feature

```
:sparkles: feat(db): Add batch delete for todos

Implement batch delete functionality:
- Add batchDelete method to TodoRepository
- Add delete confirmation dialog
- Update UI to support multi-select operations

Closes #42
```

### 🐛 fix - Bug Fix

```
:bug: fix(provider): Fix Pomodoro timer not recovering from background

When the app resumes from background, the Pomodoro timer cannot continue.
Added state recovery logic in initState.

Fixes #128
```

### 📝 docs - Documentation Update

```
:memo: docs: Update README with new feature descriptions

Added the following sections:
- Focus mode
- Data statistics
- Background music
```

### ♻️ refactor - Refactoring

```
:recycle: refactor(ui): Extract common glass container component

Created a reusable GlassContainer component to reduce code duplication across multiple screens.
```

### ⚡️ perf - Performance Optimization

```
:zap: perf(db): Optimize completed todo query performance

Added index on completed_at field,
using WHERE clause to filter completed todos instead of filtering in memory.

Before: 45ms
After: 12ms
```

### ✅ test - Tests

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

Raised minimum Flutter version requirement and updated compatible dependencies.
```

### 🚀 ci - CI Configuration

```
:rocket: ci: Add GitHub Actions test workflow

Created workflow with the following steps:
- Unit tests
- Integration tests
- Code coverage
```

### 💄 style - Formatting

```
:lipstick: style(todo_item): Format code with dart fix

Applied automated formatting fixes to maintain consistent code style.
```

---

---

## 🔖 Release Commits

When the current commit is a **Release**, the following specifications must be followed:

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
| `<hash>` | :bookmark: V<version number> |
| `<hash>` | <commit message> |
```

### Release Requirements

1. **Header**: Must use `:bookmark:` + `V<version number>` format
2. **Version number**: Follow semantic versioning specification
3. **Content completeness**: Must include **all commits** since last release
4. **Categorized by type**: Organize by `feat`, `fix`, `refactor`, `chore`, etc.
5. **Commit history**: List all related commit hashes and descriptions

### Release Example

```
:bookmark: V0.8.0: Add task statistics and data analysis features

## 📦 Version Information

**Release Date:** 2025-01-15

**Version:** 0.7.0 → 0.8.0

---

## ✨ New Features

### Statistics
- :sparkles: feat(statistics): Add focus duration statistics page
- :sparkles: feat(statistics): Add daily/weekly data visualization charts

### Todo Enhancements
- :sparkles: feat(todo): Add task priority filtering

---

## ♻️ Refactoring & Optimization

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

## 📦 New Files

- `lib/screens/statistics/statistics.dart` - Statistics page
- `lib/widgets/chart/data_chart.dart` - Chart component

---

## 📝 Commit History

| Commit | Description |
|:---:|------|
| `abc1234` | :bookmark: V0.8.0 |
| `def5678` | :sparkles: feat(statistics): Add focus duration statistics |
| `ghi9012` | :sparkles: feat(todo): Add priority filtering |
| `jkl3456` | :recycle: refactor(db): Optimize queries |
| `mno7890` | :bug: fix(todo): Fix scrolling lag |
```

### How to Get Commit History

```bash
# View all commits since last release
git log --oneline <last release commit>..HEAD

# Or view last N commits
git log --oneline -20
```

### Reference Template

For release documentation, refer to the [`release.md`](release.md) template format.

---

### 1. Set Commit Template

```bash
# Execute in project root
git config commit.template .gitmessage.txt
```

### 2. Template File

The format of the `.gitmessage.txt` file in the project root is as follows:

```
# emoji code type(scope): subject (in English)
#
# Body content (optional)
#
# Footer (optional)
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: core, db, ui, screen, widget, provider, repo, i18n, router, dep
#
# Examples:
# ✨ feat(db): Add batch delete for todos
# 🐛 fix(provider): Fix timer recovery from background
#
# Release format: 🔖 V1.0.0: Release title
```

---

## FAQ

### Q: How to choose a commit type?

- **feat**: Functional changes users can see
- **fix**: Fix issues reported by users
- **docs**: README, comments, etc.
- **chore**: Dependency updates, configuration files
- **refactor**: Code optimization without behavior changes

### Q: When should commits be split?

- Each commit should only do **one thing**
- Related features should be committed together, unrelated ones separately
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Complete Emoji List
- [release.md](release.md) - Release Template

---

> 💡 **Tip**: Keep commits atomic and descriptions clear for more efficient code review and traceability!