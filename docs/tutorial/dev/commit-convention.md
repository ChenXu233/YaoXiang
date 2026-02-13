# Commit 提交指南

本文档定义了 YaoXiang 项目的 Git 提交规范，旨在保持提交历史清晰、可读且易于理解。

---

## 目录

- [提交格式](#提交格式)
- [提交类型](#提交类型)
- [完整 Emoji 参考](#完整-emoji-参考)
- [作用域](#作用域)
- [版本管理](#版本管理)
- [消息规范](#消息规范)
- [语言规范](#语言规范)
- [🔖 发版提交](#-发版提交)
- [示例](#示例)
- [使用 Commit Template](#使用-commit-template)
- [常见问题](#常见问题)

---

## 提交格式

**非常重要！！！！！！不可以忘记！！！**
所有提交消息遵循以下格式：

```
:emoji代码: type(scope): 主题（中文）

[可选的主体内容]

[可选的页脚]
```

> ⚠️ **重要**: 必须使用 **emoji 代码**（如 `:sparkles:`）而非直接输入 emoji 字符。
> 
> **推荐使用中文提交信息**，保持团队沟通一致性。

### 组成部分

| 部分 | 说明 | 必填 |
|------|------|------|
| emoji代码 | 表情符号标识提交类型 | ✅ |
| type | 提交类型 | ✅ |
| scope | 影响范围 | ✅ |
| subject | 简短描述（中文，不超过50字符） | ✅ |
| body | 详细说明（可选） | ❌ |
| footer | 破坏性变更或关闭问题（可选） | ❌ |

---

## 提交类型

| emoji代码 | type | 说明 |
|-----------|------|------|
| :sparkles: | feat | 新功能 |
| :bug: | fix | 修复 bug |
| :memo: | docs | 仅文档变更 |
| :lipstick: | style | 代码格式（不影响功能） |
| :recycle: | refactor | 重构代码 |
| :zap: | perf | 性能优化 |
| :white_check_mark: | test | 添加或修改测试 |
| :wrench: | chore | 构建工具、辅助工具变更 |
| :building_construction: | build | 构建系统变更 |
| :rocket: | ci | CI 配置变更 |

---

## 完整 Emoji 参考

以下是与 gitmoji 项目一致的完整 emoji 列表，可根据提交内容选择合适的 emoji：

| emoji | emoji 代码 | commit 说明 |
| :---- | :---------------------------- | :--------------------------- |
| 🎨 | `:art:` | 改进代码结构/代码格式 |
| ⚡️ | `:zap:` / `:racehorse:` | 提升性能 |
| 🔥 | `:fire:` | 移除代码或文件 |
| 🐛 | `:bug:` | 修复 bug |
| 🚑 | `:ambulance:` | 重要补丁 |
| ✨ | `:sparkles:` | 引入新功能 |
| 📝 | `:memo:` | 撰写文档 |
| 🚀 | `:rocket:` | 部署功能 |
| 💄 | `:lipstick:` | 更新 UI 和样式文件 |
| 🎉 | `:tada:` | 初次提交 |
| ✅ | `:white_check_mark:` | 增加测试 |
| 🔒 | `:lock:` | 修复安全问题 |
| 🍎 | `:apple:` | 修复 macOS 下的内容 |
| 🐧 | `:penguin:` | 修复 Linux 下的内容 |
| 🏁 | `:checkered_flag:` | 修复 Windows 下的内容 |
| 🤖 | `:robot:` | 修复 Android 上的某些内容 |
| 🍏 | `:green_apple:` | 解决 iOS 上的某些问题 |
| 🔖 | `:bookmark:` | 发行/版本标签 |
| 🚨 | `:rotating_light:` | 移除 linter 警告 |
| 🚧 | `:construction:` | 工作进行中 |
| 💚 | `:green_heart:` | 修复 CI 构建问题 |
| ⬇️ | `:arrow_down:` | 降级依赖 |
| ⬆️ | `:arrow_up:` | 升级依赖 |
| 📌 | `:pushpin:` | 将依赖关系固定到特定版本 |
| 👷 | `:construction_worker:` | 添加 CI 构建系统 |
| 📈 | `:chart_with_upwards_trend:` | 添加分析或跟踪代码 |
| ♻️ | `:recycle:` | 重构代码 |
| 🔨 | `:hammer:` | 重大重构 |
| ➖ | `:heavy_minus_sign:` | 减少一个依赖 |
| 🐳 | `:whale:` | Docker 相关工作 |
| ➕ | `:heavy_plus_sign:` | 增加一个依赖 |
| 🔧 | `:wrench:` | 修改配置文件 |
| 🌐 | `:globe_with_meridians:` | 国际化与本地化 |
| ✏️ | `:pencil2:` | 修复 typo |
| 💩 | `:hankey:` | 编写需要改进的错误代码 |
| ⏪️ | `:rewind:` | 恢复更改 |
| 🔀 | `:twisted_rightwards_arrows:` | 合并分支 |
| 📦 | `:package:` | 更新编译的文件或包 |
| 👽 | `:alien:` | 由于外部 API 更改而更新代码 |
| 🚚 | `:truck:` | 移动或重命名文件 |
| 📄 | `:page_facing_up:` | 添加或更新许可证 |
| 💥 | `:boom:` | 引入突破性变化 |
| 🍱 | `:bento:` | 添加或更新资产 |
| 👌 | `:ok_hand:` | 由于代码审查更改而更新代码 |
| ♿️ | `:wheelchair:` | 提高可访问性 |
| 💡 | `:bulb:` | 记录源代码 |
| 🍻 | `:beers:` | 醉生梦死的写代码 |
| 💬 | `:speech_balloon:` | 更新文本和文字 |
| 🗃️ | `:card_file_box:` | 执行与数据库相关的更改 |
| 🔊 | `:loud_sound:` | 添加日志 |
| 🔇 | `:mute:` | 删除日志 |
| 👥 | `:busts_in_silhouette:` | 添加贡献者 |
| 🚸 | `:children_crossing:` | 改善用户体验/可用性 |
| 🏗️ | `:building_construction:` | 进行架构更改 |
| 📱 | `:iphone:` | 致力于响应式设计 |
| 🤡 | `:clown_face:` | 嘲笑事物 |
| 🥚 | `:egg:` | 添加一个复活节彩蛋 |
| 🙈 | `:see_no_evil:` | 添加或更新 .gitignore 文件 |
| 📸 | `:camera_flash:` | 添加或更新快照 |

---

## 作用域

根据项目结构，推荐使用以下作用域：

### 代码作用域

| 作用域 | 说明 |
|--------|------|
| `frontend` | 前端模块：词法分析、语法解析、类型检查 |
| `parser` | 语法解析器 |
| `lexer` | 词法分析器 |
| `typecheck` | 类型检查 |
| `middle` | 中间层：IR、优化器 |
| `codegen` | 代码生成器 |
| `monomorphize` | 单态化处理 |
| `lifetime` | 生命周期分析 |
| `vm` | 虚拟机：指令执行、栈帧、操作码 |
| `executor` | 执行器 |
| `frames` | 栈帧管理 |
| `instructions` | 指令集 |
| `runtime` | 运行时：内存管理、调度器 |
| `memory` | 内存管理 |
| `scheduler` | 任务调度 |
| `std` | 标准库 |
| `concurrent` | 并发库 |
| `io` | IO 库 |
| `net` | 网络库 |
| `util` | 工具库：诊断、缓存、Span |
| `cache` | 缓存管理 |
| `diagnostic` | 诊断信息 |

### 文档作用域

| 作用域 | 说明 |
|--------|------|
| `docs` | 通用文档更新 |
| `architecture` | 架构设计文档 |
| `design` | 语言设计规范 |
| `plan` | 实现计划文档 |
| `guides` | 指南文档 |
| `tutorial` | 教程文档 |
| `examples` | 示例代码 |

### 其他作用域

| 作用域 | 说明 |
|--------|------|
| `build` | 构建系统、依赖管理 |
| `ci` | CI/CD 配置 |
| `test` | 测试相关 |
| `chore` | 杂项任务 |
| `release` | 发版相关 |
| `meta` | 项目元配置（如 .claude, cargo 配置）|

---

## 消息规范

### 版本管理

**每次提交前必须先 bump 版本号**：

| 版本类型 | 更新位置 | 说明 |
|----------|----------|------|
| **major** | `pubspec.yaml` (version) + `release_v*.md` | 重大更新，不兼容的 API 变更 |
| **minor** | `pubspec.yaml` (version) | 新功能，向后兼容 |
| **patch** | `pubspec.yaml` (version) | 修复 bug，向后兼容 |

### 版本号格式

采用语义化版本 `MAJOR.MINOR.PATCH`：

```
# 重大版本 (breaking changes)
1.0.0 -> 2.0.0

# 次要版本 (new features)
1.0.0 -> 1.1.0

# 补丁版本 (bug fixes)
1.0.0 -> 1.0.1
```

### 提交流程

```bash
# 1. 修改代码后，先 bump 版本
# 使用 semantic_release 工具自动管理版本和 Changelog
npx semantic-release

# 或手动更新版本
# 编辑 pubspec.yaml 中的 version 字段

# 2. 提交代码（版本变更会在下一次 release 时自动生成）
git add .
git commit -m ":tada: Release v1.0.0"
git push
```

> 💡 版本 bump 和 Changelog 生成由 CI 自动完成，提交时只需确保代码变更已包含版本更新。

---

## 消息规范

### 语言规范

**推荐使用中文提交信息**，保持团队沟通一致性。

- Subject 使用中文，简洁明了
- Body 可使用中文详细说明
- 如有特殊技术术语，可保留英文

### Subject（主题）

- 使用中文，简洁明了
- 长度不超过 50 字符
- 末尾不加句号

### Body（主体）

- 详细说明变更原因和方式
- 每行不超过 72 字符
- 使用 - 或 * 列出要点

### Footer（页脚）

- **破坏性变更**: 以 `BREAKING CHANGE:` 开头
- **关闭 Issue**: 使用 `关闭 #123` 或 `修复 #456`

---

## 示例

### ✨ feat - 新功能

```
:sparkles: feat(db): 添加批量删除待办功能

实现批量删除待办事项功能：
- 在 TodoRepository 中添加 batchDelete 方法
- 添加删除确认对话框
- 更新 UI 支持多选操作

关闭 #42
```

### 🐛 fix - 修复 bug

```
:bug: fix(provider): 修复番茄钟计时器在后台无法恢复的问题

当应用从后台恢复时，番茄钟计时器无法继续计时。
在 initState 中添加了状态恢复逻辑。

修复 #128
```

### 📝 docs - 文档更新

```
:memo: docs: 更新 README 新功能说明

新增以下章节：
- 专注模式
- 数据统计
- 背景音乐
```

### ♻️ refactor - 重构

```
:recycle: refactor(ui): 提取公共玻璃态容器组件

创建可复用的 GlassContainer 组件，
减少多个屏幕间的代码重复。
```

### ⚡️ perf - 性能优化

```
:zap: perf(db): 优化已完成待办查询性能

在 completed_at 字段添加索引，
使用 WHERE 子句过滤已完成待办而非在内存中过滤。

优化前：45ms
优化后：12ms
```

### ✅ test - 测试

```
:white_check_mark: test(provider): 添加 TodoProvider 单元测试

覆盖场景：
- 添加待办
- 切换完成状态
- 删除待办
```

### 🔧 chore - 杂项

```
:wrench: chore: 更新 Flutter 版本至 3.19.0

提高最小 Flutter 版本要求并更新兼容依赖。
```

### 🚀 ci - CI 配置

```
:rocket: ci: 添加 GitHub Actions 测试工作流

创建包含以下步骤的工作流：
- 单元测试
- 集成测试
- 代码覆盖率
```

### 💄 style - 格式调整

```
:lipstick: style(todo_item): 使用 dart fix 格式化代码

应用自动格式化修复，
保持代码风格一致性。
```

---

---

## 🔖 发版提交

当本次提交为**发版（Release）**时，必须遵循以下规范：

### 发版提交格式

```
:bookmark: V<版本号>: <发版标题>

## 📦 版本信息

**发布日期:** YYYY-MM-DD

**版本号:** <旧版本> → <新版本>

---

## ✨ 新功能

### <功能模块>
- :sparkles: feat(<scope>): <功能描述>

---

## ♻️ 重构优化

- :recycle: refactor(<scope>): <重构描述>

---

## 🐛 Bug 修复

- :bug: fix(<scope>): <修复描述>

---

## 🔧 其他变更

- :wrench: chore: <变更描述>

---

## 📦 新增文件

- `<文件路径>` - <文件说明>

---

## 📝 提交记录

| 提交 | 描述 |
|:---:|------|
| `<hash>` | :bookmark: V<版本号> |
| `<hash>` | <提交消息> |
```

### 发版要求

1. **消息头**: 必须使用 `:bookmark:` + `V<版本号>` 格式
2. **版本号**: 遵循语义化版本规范
3. **内容完整性**: 必须包含自上次发版以来的**所有 commit** 内容介绍
4. **按类型分类**: 按 `feat`, `fix`, `refactor`, `chore` 等类型整理
5. **提交记录**: 列出所有相关提交的 hash 和描述

### 发版示例

```
:bookmark: V0.8.0: 新增任务统计和数据分析功能

## 📦 版本信息

**发布日期:** 2025-01-15

**版本号:** 0.7.0 → 0.8.0

---

## ✨ 新功能

### 统计功能
- :sparkles: feat(statistics): 新增专注时长统计页面
- :sparkles: feat(statistics): 添加每日/每周数据可视化图表

### 待办增强
- :sparkles: feat(todo): 添加任务优先级筛选功能

---

## ♻️ 重构优化

- :recycle: refactor(db): 优化数据库查询性能
- :recycle: refactor(provider): 重构状态管理逻辑

---

## 🐛 Bug 修复

- :bug: fix(todo): 修复任务列表滑动卡顿问题

---

## 🔧 其他变更

- :wrench: chore: 更新依赖版本到最新稳定版
- :memo: docs: 更新 README 安装说明

---

## 📦 新增文件

- `lib/screens/statistics/statistics.dart` - 统计页面
- `lib/widgets/chart/data_chart.dart` - 图表组件

---

## 📝 提交记录

| 提交 | 描述 |
|:---:|------|
| `abc1234` | :bookmark: V0.8.0 |
| `def5678` | :sparkles: feat(statistics): 新增专注时长统计 |
| `ghi9012` | :sparkles: feat(todo): 添加优先级筛选 |
| `jkl3456` | :recycle: refactor(db): 优化查询 |
| `mno7890` | :bug: fix(todo): 修复滑动卡顿 |
```

### 如何获取提交记录

```bash
# 查看上次发版以来的所有提交
git log --oneline <上次发版commit>..HEAD

# 或查看最近 N 条提交
git log --oneline -20
```

### 参考模板

发版文档请参考 [`release.md`](release.md) 模板格式进行编写。

---

### 1. 设置 Commit Template

```bash
# 在项目根目录执行
git config commit.template .gitmessage.txt
```

### 2. Template 文件

项目根目录的 `.gitmessage.txt` 文件格式如下：

```
# emoji代码 type(scope): 主题（中文）
#
# 主体内容（可选）
#
# 页脚（可选）
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: core, db, ui, screen, widget, provider, repo, i18n, router, dep
#
# 示例:
# ✨ feat(db): 添加批量删除待办功能
# 🐛 fix(provider): 修复计时器后台恢复问题
#
# 发版格式: 🔖 V1.0.0: 发版标题
```

---

## 常见问题

### Q: 如何选择提交类型？

- **feat**: 用户能看到的功能变更
- **fix**: 修复用户报告的问题
- **docs**: README、注释等文档
- **chore**: 依赖更新、配置文件
- **refactor**: 不改变行为的代码优化

### Q: 何时应该拆分提交？

- 每个提交只做**一件事**
- 相关功能一起提交，不相关分开
- 遵循 Atomic Commits 原则

---

## 参考资料

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Emoji 完整列表
- [release.md](release.md) - 发版模板

---

> 💡 **提示**: 保持提交原子性、描述清晰，让代码审查和回溯更高效！
