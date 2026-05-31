# Commit Submission Guidelines

This document defines the Git commit specification for the YaoXiang project, aiming to keep the commit history clear, readable, and easy to understand.

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

**Very important!!!! Do not forget!!!!**
All commit messages follow this format:

```
:emoji code: type(scope): subject (Chinese)

[optional body]

[optional footer]
```

> ⚠️ **Important**: Must use **emoji codes** (e.g., `:sparkles:`) instead of directly entering emoji characters.
>
> **Chinese commit messages are recommended** to maintain consistency in team communication.

### Components

| Part | Description | Required |
|------|-------------|----------|
| emoji code | Emoji symbol identifying commit type | ✅ |
| type | Commit type | ✅ |
| scope | Affected scope | ✅ |
| subject | Brief description (Chinese, no more than 50 characters) | ✅ |
| body | Detailed explanation (optional) | ❌ |
| footer | Breaking changes or issue closing (optional) | ❌ |

---

## Commit Types

| emoji code | type | Description |
|------------|------|-------------|
| :sparkles: | feat | New feature |
| :bug: | fix | Bug fix |
| :memo: | docs | Documentation changes only |
| :lipstick: | style | Code formatting (no functional impact) |
| :recycle: | refactor | Code refactoring |
| :zap: | perf | Performance optimization |
| :white_check_mark: | test | Add or modify tests |
| :wrench: | chore | Build tool, auxiliary tool changes |
| :building_construction: | build | Build system changes |
| :rocket: | ci | CI configuration changes |

---

## Complete Emoji Reference

The following is a complete emoji list consistent with the gitmoji project. Select the appropriate emoji based on commit content:

| emoji | emoji code | commit description |
| :---- | :---------------------------- | :--------------------------- |
| 🎨 | `:art:` | Improve code structure/code formatting |
| ⚡️ | `:zap:` / `:racehorse:` | Improve performance |
| 🔥 | `:fire:` | Remove code or files |
| 🐛 | `:bug:` | Fix a bug |
| 🚑 | `:ambulance:` | Important patch |
| ✨ | `:sparkles:` | Introduce new features |
| 📝 | `:memo:` | Write documentation |
| 🚀 | `:rocket:` | Deploy features |
| 💄 | `:lipstick:` | Update UI and style files |
| 🎉 | `:tada:` | Initial commit |
| ✅ | `:white_check_mark:` | Add tests |
| 🔒 | `:lock:` | Fix security issues |
| 🍎 | `:apple:` | Fix macOS-related issues |
| 🐧 | `:penguin:` | Fix Linux-related issues |
| 🏁 | `:checkered_flag:` | Fix Windows-related issues |
| 🤖 | `:robot:` | Fix Android-related issues |
| 🍏 | `:green_apple:` | Fix iOS-related issues |
| 🔖 | `:bookmark:` | Release/version tags |
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
| 🐳 | `:whale:` | Docker-related work |
| ➕ | `:heavy_plus_sign:` | Add a dependency |
| 🔧 | `:wrench:` | Modify configuration files |
| 🌐 | `:globe_with_meridians:` | Internationalization and localization |
| ✏️ | `:pencil2:` | Fix typos |
| 💩 | `:hankey:` | Write code that needs improvement |
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
| 🍻 | `:beers:` | Write code while having fun |
| 💬 | `:speech_balloon:` | Update text and literals |
| 🗃️ | `:card_file_box:` | Perform database-related changes |
| 🔊 | `:loud_sound:` | Add logs |
| 🔇 | `:mute:` | Remove logs |
| 👥 | `:busts_in_silhouette:` | Add contributors |
| 🚸 | `:children_crossing:` | Improve user experience/usability |
| 🏗️ | `:building_construction:` | Make architectural changes |
| 📱 | `:iphone:` | Work on responsive design |
| 🤡 | `:clown_face:` | Mock things |
| 🥚 | `:egg:` | Add an easter egg |
| 🙈 | `:see_no_evil:` | Add or update .gitignore file |
| 📸 | `:camera_flash:` | Add or update snapshots |

---

## Scopes

Based on the project structure, the following scopes are recommended:

### Code Scopes

| Scope | Description |
|-------|-------------|
| `frontend` | Frontend module: lexical analysis, syntax parsing, type checking |
| `parser` | Syntax parser |
| `lexer` | Lexical analyzer |
| `typecheck` | Type checking |
| `middle` | Middle layer: IR, optimizer |
| `codegen` | Code generator |
| `monomorphize` | Monomorphization handling |
| `lifetime` | Lifetime analysis |
| `vm` | Virtual machine: instruction execution, stack frames, opcodes |
| `executor` | Executor |
| `frames` | Stack frame management |
| `instructions` | Instruction set |
| `runtime` | Runtime: memory management, scheduler |
| `memory` | Memory management |
| `scheduler` | Task scheduling |
| `std` | Standard library |
| `concurrent` | Concurrent library |
| `io` | IO library |
| `net` | Network library |
| `util` | Utility library: diagnostics, cache, Span |
| `cache` | Cache management |
| `diagnostic` | Diagnostic information |

### Documentation Scopes

| Scope | Description |
|-------|-------------|
| `docs` | General documentation updates |
| `architecture` | Architecture design documentation |
| `design` | Language design specification |
| `plan` | Implementation plan documentation |
| `guides` | Guide documentation |
| `tutorial` | Tutorial documentation |
| `examples` | Example code |

### Other Scopes

| Scope | Description |
|-------|-------------|
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

### Commit Flow

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

> 💡 Version bump and Changelog generation are done automatically by CI. Just ensure the code changes include version updates when committing.

---

## Message Guidelines

### Language Guidelines

**Chinese commit messages are recommended** to maintain consistency in team communication.

- Subject uses Chinese, concise and clear
- Body can use Chinese for detailed explanation
- Special technical terms can remain in English

### Subject

- Use Chinese, concise and clear
- Length does not exceed 50 characters
- No period at the end

### Body

- Explain the reason and method of changes in detail
- Each line does not exceed 72 characters
- Use - or * to list key points

### Footer

- **Breaking Changes**: Start with `BREAKING CHANGE:`
- **Closing Issues**: Use `关闭 #123` or `修复 #456`

---

## Examples

### ✨ feat - New Feature

```
:sparkles: feat(db): 添加批量删除待办功能 (Add batch delete todo feature)

实现批量删除待办事项功能： (Implement batch delete todo items feature:)
- 在 TodoRepository 中添加 batchDelete 方法 (Add batchDelete method in TodoRepository)
- 添加删除确认对话框 (Add delete confirmation dialog)
- 更新 UI 支持多选操作 (Update UI to support multi-select operations)

关闭 #42 (Close #42)
```

### 🐛 fix - Bug Fix

```
:bug: fix(provider): 修复番茄钟计时器在后台无法恢复的问题 (Fix Pomodoro timer not recovering from background)

当应用从后台恢复时，番茄钟计时器无法继续计时。 (When app recovers from background, Pomodoro timer cannot continue timing.)
在 initState 中添加了状态恢复逻辑。 (Added state recovery logic in initState.)

修复 #128 (Fix #128)
```

### 📝 docs - Documentation Update

```
:memo: docs: 更新 README 新功能说明 (Update README new feature description)

新增以下章节： (Added the following sections:)
- 专注模式 (Focus mode)
- 数据统计 (Data statistics)
- 背景音乐 (Background music)
```

### ♻️ refactor - Refactoring

```
:recycle: refactor(ui): 提取公共玻璃态容器组件 (Extract common glass container component)

创建可复用的 GlassContainer 组件， (Create reusable GlassContainer component,)
减少多个屏幕间的代码重复。 (reducing code duplication between multiple screens.)
```

### ⚡️ perf - Performance Optimization

```
:zap: perf(db): 优化已完成待办查询性能 (Optimize completed todo query performance)

在 completed_at 字段添加索引， (Add index on completed_at field,)
使用 WHERE 子句过滤已完成待办而非在内存中过滤。 (use WHERE clause to filter completed todos instead of filtering in memory.)

优化前：45ms (Before: 45ms)
优化后：12ms (After: 12ms)
```

### ✅ test - Test

```
:white_check_mark: test(provider): 添加 TodoProvider 单元测试 (Add TodoProvider unit tests)

覆盖场景： (Covered scenarios:)
- 添加待办 (Add todo)
- 切换完成状态 (Toggle completion status)
- 删除待办 (Delete todo)
```

### 🔧 chore - Miscellaneous

```
:wrench: chore: 更新 Flutter 版本至 3.19.0 (Update Flutter version to 3.19.0)

提高最小 Flutter 版本要求并更新兼容依赖。 (Increase minimum Flutter version requirement and update compatible dependencies.)
```

### 🚀 ci - CI Configuration

```
:rocket: ci: 添加 GitHub Actions 测试工作流 (Add GitHub Actions test workflow)

创建包含以下步骤的工作流： (Create workflow with the following steps:)
- 单元测试 (Unit tests)
- 集成测试 (Integration tests)
- 代码覆盖率 (Code coverage)
```

### 💄 style - Format Adjustment

```
:lipstick: style(todo_item): 使用 dart fix 格式化代码 (Format code with dart fix)

应用自动格式化修复， (Apply automatic formatting fixes,)
保持代码风格一致性。 (maintaining code style consistency.)
```

---

---

## 🔖 Release Commits

When the current commit is a **Release**, the following specifications must be followed:

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

## ♻️ Refactoring and Optimization

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

1. **Header**: Must use `:bookmark:` + `V<version>` format
2. **Version Number**: Follow semantic versioning specification
3. **Content Completeness**: Must include **all commits** since the last release
4. **Categorization by Type**: Organize by `feat`, `fix`, `refactor`, `chore`, etc.
5. **Commit History**: List all related commit hashes and descriptions

### Release Example

```
:bookmark: V0.8.0: 新增任务统计和数据分析功能 (Add task statistics and data analysis features)

## 📦 Version Information

**Release Date:** 2025-01-15

**Version:** 0.7.0 → 0.8.0

---

## ✨ New Features

### Statistics Features
- :sparkles: feat(statistics): 新增专注时长统计页面 (Add focus duration statistics page)
- :sparkles: feat(statistics): 添加每日/每周数据可视化图表 (Add daily/weekly data visualization charts)

### Todo Enhancements
- :sparkles: feat(todo): 添加任务优先级筛选功能 (Add task priority filtering)

---

## ♻️ Refactoring and Optimization

- :recycle: refactor(db): 优化数据库查询性能 (Optimize database query performance)
- :recycle: refactor(provider): 重构状态管理逻辑 (Refactor state management logic)

---

## 🐛 Bug Fixes

- :bug: fix(todo): 修复任务列表滑动卡顿问题 (Fix task list scrolling lag)

---

## 🔧 Other Changes

- :wrench: chore: 更新依赖版本到最新稳定版 (Update dependency versions to latest stable)
- :memo: docs: 更新 README 安装说明 (Update README installation instructions)

---

## 📦 New Files

- `lib/screens/statistics/statistics.dart` - Statistics page
- `lib/widgets/chart/data_chart.dart` - Chart component

---

## 📝 Commit History

| Commit | Description |
|:---:|------|
| `abc1234` | :bookmark: V0.8.0 |
| `def5678` | :sparkles: feat(statistics): 新增专注时长统计 (Add focus duration statistics) |
| `ghi9012` | :sparkles: feat(todo): 添加优先级筛选 (Add priority filtering) |
| `jkl3456` | :recycle: refactor(db): 优化查询 (Optimize queries) |
| `mno7890` | :bug: fix(todo): 修复滑动卡顿 (Fix scrolling lag) |
```

### How to Get Commit History

```bash
# View all commits since last release
git log --oneline <last release commit>..HEAD

# Or view last N commits
git log --oneline -20
```

### Reference Template

For release documentation, please refer to the [`release.md`](release.md) template format for composition.

---

### 1. Setting Commit Template

```bash
# Execute in project root directory
git config commit.template .gitmessage.txt
```

### 2. Template File

The `.gitmessage.txt` file in the project root directory has the following format:

```
# emoji code type(scope): subject (Chinese)
#
# Body content (optional)
#
# Footer (optional)
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: core, db, ui, screen, widget, provider, repo, i18n, router, dep
#
# Examples:
# ✨ feat(db): 添加批量删除待办功能 (Add batch delete todo feature)
# 🐛 fix(provider): 修复计时器后台恢复问题 (Fix timer background recovery issue)
#
# Release format: 🔖 V1.0.0: Release Title
```

---

## FAQ

### Q: How to choose a commit type?

- **feat**: Changes that users can see
- **fix**: Fix issues reported by users
- **docs**: README, comments, etc.
- **chore**: Dependency updates, configuration files
- **refactor**: Code optimization without behavior changes

### Q: When should commits be split?

- Each commit does **one thing**
- Related features are committed together, unrelated ones are separated
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Complete emoji list
- [release.md](release.md) - Release template

---

> 💡 **Tip**: Keep commits atomic and descriptions clear to make code review and traceability more efficient!