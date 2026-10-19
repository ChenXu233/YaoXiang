---
title: 'Release Template'
---

# Release Template

> The `/release` command generates the changelog following this template.  
> The changelog is a **change description written for humans**, not a commit list.

## Format Specification

```
:bookmark: V<版本号>: <标题>
```

## Scope of CHANGELOG.md

`CHANGELOG.md` **only keeps the current version section**—after release, the file contains only the
current version section.

- **During development**: Maintain an "Unreleased (dev)" section at the top of the file,
  accumulating the change descriptions to be written for this release. This section is created by
  the first commit that accumulates changes during development and is not part of the release
  process—if it does not exist at release time, simply create a new section for this version.
- **At release time**: Rewrite the "Unreleased (dev)" section as the current version section
  (heading `## :bookmark: V<version>: <title>`), **and delete all other historical version sections
  in the file**; before release, self-check `grep -c '^## ' CHANGELOG.md` must be 1.
- **Why it must be deleted**: The three artifacts for this release all come from this file—the PR
  body, the version bump commit body, the merge commit body, and the GitHub Release body generated
  by CI (`scripts/release/generate-commit-list.ts --changelog CHANGELOG.md`, which reads the file at
  the commit when the tag is created). Keeping old version sections = the old version's change
  descriptions will appear in **this release**.
- **Where to find historical versions**: The complete changelog for each version is published with
  the GitHub Release; the file contents of any historical version can also be retrieved via
  `git show <tag>:CHANGELOG.md`. The current file in the repository therefore does not need to serve
  as a historical archive.

**Why this is specified this way**: The readers of this file are concerned with "what changed in the
current version"; cross-version tracing is handled by the Release page and git; the file does not
grow with versions, thereby avoiding the unsolvable judgment issue of "which sections should be
kept".

> Step 4 of the `/release` skill follows this section.

## 📦 Version Information

| Item           | Value                             |
| -------------- | --------------------------------- |
| Release Date   | YYYY-MM-DD                        |
| Version Change | `<old version>` → `<new version>` |
| Commits        | N commits                         |

## 📋 Release Summary

Use 2-3 sentences to summarize the core content and significance of this release.

## ✨ New Features

### `<Feature Area Title>`

`<A paragraph explaining what this feature does, what problem it solves, and what it means for users>`

- Specific change 1
- Specific change 2
- Specific change 3

### `<Another Feature Area>`

`<Description>`

- Specific change

## 🐛 Bug Fixes

### `<Fix Area>`

`<Description of what was fixed and the scope of impact>`

- Specific fix 1
- Specific fix 2

## ♻️ Refactoring and Optimization

### `<Refactoring Direction>`

`<Description of why the refactoring was done and the benefits>`

- Specific change

## Title Rules

Summarize the core change in one sentence, no more than 50 characters:

```
:bookmark: V0.7.2: REPL 重写与类型系统改进
:bookmark: V0.7.3: 类型系统修复与所有权模型改进
:bookmark: V0.8.0: 并发模型与泛型系统
```

## Classification Rules

Classify by `type` prefix, omit empty categories:

|    type    |     Category     |        Prefix        |
| :--------: | :--------------: | :------------------: |
|   `feat`   | ✨ New Features  |     `:sparkles:`     |
|   `fix`    |   🐛 Bug Fixes   |       `:bug:`        |
| `refactor` |  ♻️ Refactoring  |     `:recycle:`      |
|   `perf`   |  ⚡ Performance  |       `:zap:`        |
|   `docs`   | 📝 Documentation |       `:memo:`       |
|  `style`   |     🎨 Style     |       `:art:`        |
|   `test`   |     ✅ Tests     | `:white_check_mark:` |
|  `chore`   |  🔧 Build/Tools  |      `:wrench:`      |
|    `ci`    |     💚 CI/CD     |   `:green_heart:`    |

## Complete Example

`:bookmark: V0.7.3: Type system fixes and ownership model improvements`

```markdown
## 📦 版本信息

| 项目     | 值                |
| -------- | ----------------- |
| 发布日期 | 2026-06-07        |
| 版本变更 | `0.7.2` → `0.7.3` |
| 提交数   | 22 个 commit      |

## 📋 本次更新概要

本次发版聚焦类型系统和所有权模型的稳定性修复。泛型类型实例化（如
`List(Int) = List(1, 2, 3)`）现在可以正确工作，`&T`
字段访问和元组解构赋值的多个边界情况已修复。同时完成了所有权检查的统一重构，为后续 move 语义的完善打下基础。

## ✨ 新功能

### 泛型类型实例化

支持 `List(Int) = List(1, 2, 3)`
语法，类型构造器正确注册为 Struct。泛型函数多次调用时类型推断不再冲突，每次调用获得独立的类型变量实例。

- 类型构造器注册为 Struct，修复实例化根因
- 泛型函数多次调用独立推断
- 接口方法绑定修复

### Move 后重新绑定

变量被 move 后可以重新绑定新值，赋值优先查找现在正确处理 moved 状态，避免误报"变量已移动"错误。

- VarInfo 增加 moved 状态标记
- 赋值优先查找增加 moved 分支

### Resource marker trait

新增 `Resource` marker trait，标记实现了 IO 副作用的类型。为后续副作用追踪和并发安全分析提供基础。

### PLDI SRC demo

完成 PLDI SRC demo MVP 全部任务，包含端到端的类型检查和代码生成演示。

## 🐛 Bug 修复

### &T 字段访问

修复不可变引用类型的字段访问在类型检查中的多个问题，包括字段赋值和通过引用调用构造函数的推断。

- `&T` 字段访问类型检查修复
- 字段赋值目标类型推断
- 构造函数通过引用调用的推断

### LSP 语义高亮

修复 `spawn {}` 块内变量和元组解构赋值 `(a, b) = ...` 的语义高亮缺失。AST 中
`DestructureAssign.names` 现在携带每个变量名的位置信息。

- spawn 块内变量正确染色
- 元组解构赋值的变量名正确染色
- 为后续 move 语义的 LSP 支持打下基础

### freeze 移除

移除已废弃的 `freeze` 函数，清理相关测试用例。

## ♻️ 重构优化

### 统一诊断系统

将剩余 5 个错误枚举迁移至统一的 `ErrorCodeDefinition`
诊断系统，错误码覆盖率从 60% 提升至 80%。所有编译错误现在遵循统一的格式和错误码规范。

### 所有权检查统一

将分散在多处的所有权检查逻辑统一为
`OwnershipPass`，单一入口处理 borrow、move、生命周期检查。降低维护成本，提高错误一致性。

### 并发模型简化

移除 Send/Sync 约束系统，`spawn {}` 重构为分组执行模型。移除 `@block`/`@eager`/`@auto`
装饰器相关代码（RFC-024 phase 1）。简化并发模型，为后续可扩展并发原语设计铺路。
```

## Process Overview

```
收集提交 → 生成 changelog → 创建 PR → 等待 CI 全绿 → bump 版本 → 合并
```

See `.claude/commands/release.md` for details
