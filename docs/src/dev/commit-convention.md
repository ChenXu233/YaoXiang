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

作用域基于项目 `src/` 目录结构，**必须使用以下已定义的 scope**：

### 顶层模块

| 作用域 | 对应目录 | 说明 |
|--------|----------|------|
| `frontend` | `src/frontend/` | 前端：词法分析、语法解析、类型检查 |
| `middle` | `src/middle/` | 中间层：IR、优化、单态化 |
| `backends` | `src/backends/` | 后端：解释器、运行时、REPL |
| `std` | `src/std/` | 标准库 |
| `formatter` | `src/formatter/` | 代码格式化器 |
| `lsp` | `src/lsp/` | 语言服务器协议 |
| `package` | `src/package/` | 包管理器 |
| `util` | `src/util/` | 工具库：诊断、缓存、i18n |

### 前端子模块

| 作用域 | 对应目录 | 说明 |
|--------|----------|------|
| `parser` | `src/frontend/core/parser/` | 语法解析器 |
| `lexer` | `src/frontend/core/lexer/` | 词法分析器 |
| `typecheck` | `src/frontend/core/typecheck/` | 类型检查 |
| `types` | `src/frontend/core/types/` | 类型系统定义 |

### 中间层子模块

| 作用域 | 对应目录 | 说明 |
|--------|----------|------|
| `codegen` | `src/middle/passes/codegen/` | 代码生成（字节码） |
| `monomorphize` | `src/middle/passes/monomorphize/` | 单态化处理 |
| `lifetime` | `src/middle/passes/lifetime/` | 生命周期分析 |

### 后端子模块

| 作用域 | 对应目录 | 说明 |
|--------|----------|------|
| `repl` | `src/backends/dev/repl/` | REPL 交互式命令行 |
| `shell` | `src/backends/dev/shell.rs` | Shell 命令处理 |
| `runtime` | `src/backends/runtime/` | 运行时执行引擎 |

### 文档作用域

| 作用域 | 说明 |
|--------|------|
| `docs` | 通用文档更新 |
| `design` | 语言设计规范（RFC） |
| `plan` | 实现计划文档 |

### 其他作用域

| 作用域 | 说明 |
|--------|------|
| `build` | 构建系统、Cargo 配置 |
| `ci` | CI/CD 配置（GitHub Actions） |
| `test` | 测试相关 |
| `release` | 发版相关 |
| `meta` | 项目元配置（.claude, .gitignore 等） |

---

## 消息规范

### 版本管理

版本号定义在项目根目录 `Cargo.toml` 的 `version` 字段：

```toml
[package]
version = "0.7.2"
```

采用语义化版本 `MAJOR.MINOR.PATCH`：

| 版本类型 | 说明 | 示例 |
|----------|------|------|
| **major** | 重大更新，不兼容的 API 变更 | 0.7.2 → 1.0.0 |
| **minor** | 新功能，向后兼容 | 0.7.2 → 0.8.0 |
| **patch** | 修复 bug，向后兼容 | 0.7.2 → 0.7.3 |

> ⚠️ 发版时 **在 dev 分支更新 `Cargo.toml` 版本号**，通过 PR 合并到 main 后由 CI 自动创建 tag 和 Release。**不要手动推 tag**，否则 CI 会跳过 release 流程。

---

## CI 发版流程

发版由 GitHub Actions (`release.yml`) 自动完成，流程如下：

```
1. 在 dev 分支上更新 Cargo.toml 的 version 字段
2. cargo build 更新 Cargo.lock
3. 按发版格式 commit（见下方 🔖 发版提交）
   - commit message 必须包含自上次发版以来的所有变更（即 PR 的完整内容）
4. 从 dev 创建 PR 到 main
5. 合并 PR 到 main
6. CI 自动检测：
   - 读取 Cargo.toml 版本号 → "v{version}"
   - 检查该 tag 是否已存在
   - 不存在 → 触发完整 release 流程
   - 已存在 → 跳过（不会重复发布）
7. CI 自动执行：
   - 并行：跨平台构建 (Linux/Windows/macOS) + 安全审计 + 测试
   - 全部通过后：创建 tag、打包产物、发布 GitHub Release
```

### 关键规则

| 规则 | 说明 |
|------|------|
| **不要手动推 tag** | CI 根据 tag 是否存在决定是否发布，手动推 tag 会导致 CI 跳过 |
| **版本在 dev 上 bump** | 发版 commit 在 dev 上完成，通过 PR 合并到 main |
| **发版 commit 包含完整 changelog** | commit message 需包含本次发版的所有变更内容，因为它是 PR 的描述来源 |
| **不要合并 main 回 dev** | PR 合并后 dev 会自动同步，无需反向合并 |

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
:sparkles: feat(parser): 添加闭包语法解析支持

实现闭包表达式解析：
- 支持 |args| body 简写语法
- 支持 move 语义捕获
- 添加闭包类型推断

关闭 #42
```

### 🐛 fix - 修复 bug

```
:bug: fix(repl): 修复多行输入时补全器失效的问题

SessionREPL 在多行模式下未正确注册补全器，
导致 Tab 补全无法触发。

修复 #128
```

### 📝 docs - 文档更新

```
:memo: docs(design): 更新所有权模型与类型系统规范

同步 RFC-009 和 RFC-011 的最新设计变更。
```

### ♻️ refactor - 重构

```
:recycle: refactor(typecheck): 分离原语值类型与 Dup 浅拷贝语义

将 MonoType 中的值类型和拷贝语义解耦，
消除 match 分支中的特殊情况。
```

### ⚡️ perf - 性能优化

```
:zap: perf(types): 优化 const generic 求值性能

为递归求值添加深度限制（默认 128），
避免恶意构造的类型表达式导致栈溢出。
```

### ✅ test - 测试

```
:white_check_mark: test(typecheck): 补充 scope VarInfo 可变性测试

覆盖场景：
- 不可变绑定的只读访问
- mut 绑定的可变性追踪
- 跨作用域的可变性传播
```

### 🔧 chore - 杂项

```
:wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap

升级 6 个生产依赖至最新稳定版本。
```

### 🚀 ci - CI 配置

```
:rocket: ci: 修复 nightly 构建 Rust 版本过低的问题

将 RUST_TOOLCHAIN 从 1.91.0 更新至 1.96.0，
匹配 Cargo.toml 中的 rust-version 要求。
```

### 💄 style - 格式调整

```
:lipstick: style(frontend): 应用 cargo fmt 格式化

统一函数签名的换行风格。
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
:bookmark: V0.7.2: REPL 重写与类型系统改进

## 📦 版本信息

**发布日期:** 2026-06-01

**版本号:** 0.7.1 → 0.7.2

---

## ✨ 新功能

- :sparkles: feat(typecheck): 实现泛型类型参数自动推断
- :sparkles: feat(typecheck): 添加 MonoType::Generic 结构化泛型表示
- feat: 接入 CLI REPL 命令到 SessionREPL

---

## ♻️ 重构优化

- :recycle: refactor(backends): 移除 tui_repl 模块，重写为 SessionREPL
- :recycle: refactor(typecheck): scope 变量存储引入 VarInfo 追踪可变性
- :recycle: refactor(typecheck): 分离原语值类型与 Dup 浅拷贝语义

---

## 🐛 Bug 修复

- :bug: fix(repl): 配置默认 REPL 历史记录，修复 shell evaluate_code
- :bug: fix(repl): 注册补全器并修复多行输入
- :bug: fix(repl): 移除 wrap_code 中多余的分号以保留表达式值

---

## ⚡ 性能优化

- :zap: perf(types): 为 const generic 求值添加递归深度限制

---

## 🔧 其他变更

- :wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap, owo-colors
- :white_check_mark: test(typecheck): 补充 scope VarInfo 可变性测试

---

## 📝 提交记录

| 提交 | 描述 |
|:---:|------|
| `f438aab` | :sparkles: feat(typecheck): 实现泛型类型参数自动推断 |
| `bf0c121` | :zap: perf(types): 递归深度限制 |
| `6edac15` | feat: 接入 CLI REPL 到 SessionREPL |
| `02cf54f` | :sparkles: feat(typecheck): MonoType::Generic |
| `3160a28` | :recycle: refactor(typecheck): VarInfo 追踪可变性 |
| `f00a2a4` | :recycle: refactor(backends): 移除 tui_repl 模块 |
| `afe3e0c` | :bug: fix(repl): REPL 历史记录和 shell 修复 |
| `c4d2242` | :wrench: chore(build): 依赖 bump |
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
# Scopes: frontend, parser, lexer, typecheck, types, middle, codegen,
#         monomorphize, lifetime, backends, repl, shell, runtime,
#         std, formatter, lsp, package, util, docs, design, plan,
#         build, ci, test, release, meta
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
