# Commit Submission Guide

This document defines the Git commit conventions for the YaoXiang project, aiming to keep the commit
history clear, readable, and easy to understand.

---

## Table of Contents

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

## Commit Format

**Very important!!!!!! Don't forget!!!** All commit messages follow this format:

```
:emoji代码: type(scope): 主题（中文）

[可选的主体内容]

[可选的页脚]
```

> ⚠️ **Important**: You MUST use **emoji codes** (such as `:sparkles:`) rather than typing the
> actual emoji character.
>
> **Chinese commit messages are recommended** to maintain team communication consistency.

### Components

| Component | Description                                        | Required |
| --------- | -------------------------------------------------- | -------- |
| emoji代码 | Emoji that identifies the commit type              | ✅       |
| type      | Commit type                                        | ✅       |
| scope     | Affected area                                      | ✅       |
| subject   | Brief description (Chinese, no more than 50 chars) | ✅       |
| body      | Detailed explanation (optional)                    | ❌       |
| footer    | Breaking changes or issue references (optional)    | ❌       |

---

## Commit Types

| emoji code              | type     | Description                          |
| ----------------------- | -------- | ------------------------------------ |
| :sparkles:              | feat     | New feature                          |
| :bug:                   | fix      | Bug fix                              |
| :memo:                  | docs     | Documentation changes only           |
| :lipstick:              | style    | Code formatting (no behavior change) |
| :recycle:               | refactor | Code refactoring                     |
| :zap:                   | perf     | Performance improvement              |
| :white_check_mark:      | test     | Add or modify tests                  |
| :wrench:                | chore    | Build tools, auxiliary tool changes  |
| :building_construction: | build    | Build system changes                 |
| :rocket:                | ci       | CI configuration changes             |

---

## Complete Emoji Reference

Below is the full emoji list aligned with the gitmoji project; pick the one that fits the commit
content:

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
| 🔨    | `:hammer:`                    | Major refactor                          |
| ➖    | `:heavy_minus_sign:`          | Remove a dependency                     |
| 🐳    | `:whale:`                     | Docker-related work                     |
| ➕    | `:heavy_plus_sign:`           | Add a dependency                        |
| 🔧    | `:wrench:`                    | Modify configuration files              |
| 🌐    | `:globe_with_meridians:`      | Internationalization & localization     |
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
| 💡    | `:bulb:`                      | Add or update source comments           |
| 🍻    | `:beers:`                     | Drunken coding                          |
| 💬    | `:speech_balloon:`            | Update text and literals                |
| 🗃️    | `:card_file_box:`             | Database-related changes                |
| 🔊    | `:loud_sound:`                | Add logs                                |
| 🔇    | `:mute:`                      | Remove logs                             |
| 👥    | `:busts_in_silhouette:`       | Add contributors                        |
| 🚸    | `:children_crossing:`         | Improve UX/usability                    |
| 🏗️    | `:building_construction:`     | Architectural changes                   |
| 📱    | `:iphone:`                    | Work on responsive design               |
| 🤡    | `:clown_face:`                | Mock things                             |
| 🥚    | `:egg:`                       | Add an easter egg                       |
| 🙈    | `:see_no_evil:`               | Add or update .gitignore                |
| 📸    | `:camera_flash:`              | Add or update snapshots                 |

---

## Scope

The scope is based on the repository directory structure. **You MUST use one of the following
defined scopes**:

### Top-Level Modules

| Scope       | Directory                            | Description                                                                    |
| ----------- | ------------------------------------ | ------------------------------------------------------------------------------ |
| `frontend`  | `src/frontend/`                      | Frontend: lexer, parser, type checker                                          |
| `middle`    | `src/middle/`                        | Middle layer: IR, optimization, monomorphization                               |
| `backends`  | `src/backends/`                      | Backend: interpreter, runtime, REPL                                            |
| `std`       | `src/std/`                           | Standard library                                                               |
| `formatter` | `src/formatter/`                     | Code formatter                                                                 |
| `lsp`       | `src/lsp/`                           | Language Server Protocol                                                       |
| `package`   | `src/package/`                       | Package manager                                                                |
| `util`      | `src/util/`                          | Utilities: diagnostics, cache, i18n                                            |
| `proof`     | `src/frontend/core/typecheck/proof/` | Proof and verification support (promoted to top-level per RFC-039 final state) |

### Frontend Sub-Modules

| Scope       | Directory                      | Description             |
| ----------- | ------------------------------ | ----------------------- |
| `parser`    | `src/frontend/core/parser/`    | Syntax parser           |
| `lexer`     | `src/frontend/core/lexer/`     | Lexical analyzer        |
| `typecheck` | `src/frontend/core/typecheck/` | Type checking           |
| `types`     | `src/frontend/core/types/`     | Type system definitions |

### Middle Layer Sub-Modules

| Scope          | Directory                     | Description                                                |
| -------------- | ----------------------------- | ---------------------------------------------------------- |
| `codegen`      | `src/middle/passes/codegen/`  | Code generation (bytecode)                                 |
| `monomorphize` | `src/middle/passes/mono/`     | Monomorphization (directory renamed per RFC-039 P6 ruling) |
| `lifetime`     | `src/middle/passes/lifetime/` | Lifetime analysis                                          |

### Backend Sub-Modules

| Scope     | Directory               | Description              |
| --------- | ----------------------- | ------------------------ |
| `repl`    | `src/repl/`             | REPL interactive CLI     |
| `runtime` | `src/backends/runtime/` | Runtime execution engine |

### Documentation Scopes

| Scope    | Description                                           |
| -------- | ----------------------------------------------------- |
| `docs`   | General documentation updates                         |
| `design` | Language design specifications (docs/src/dev/design/) |
| `plan`   | Implementation plan documents                         |
| `rfc`    | RFC documents (docs/src/rfc/)                         |

### Other Scopes

| Scope      | Description                                     |
| ---------- | ----------------------------------------------- |
| `build`    | Build system, Cargo configuration               |
| `ci`       | CI/CD configuration (GitHub Actions)            |
| `test`     | Test-related                                    |
| `release`  | Release-related                                 |
| `meta`     | Project meta-config (.claude, .gitignore, etc.) |
| `deps`     | Dependency upgrades (dependabot convention)     |
| `examples` | Examples directory `examples/`                  |
| `vscode`   | VS Code extension `vscode-extension/`           |
| `benches`  | Benchmarks `benches/`                           |

---

## Message Conventions

### Version Management

The version number is defined in the `version` field of `Cargo.toml` at the project root:

```toml
[package]
version = "0.7.2"
```

We follow semantic versioning `MAJOR.MINOR.PATCH`:

| Version Type | Description                            | Example       |
| ------------ | -------------------------------------- | ------------- |
| **major**    | Major update, incompatible API changes | 0.7.2 → 1.0.0 |
| **minor**    | New feature, backward-compatible       | 0.8.2 → 0.8.0 |
| **patch**    | Bug fix, backward-compatible           | 0.7.2 → 0.7.3 |

> ⚠️ When releasing, **update the `Cargo.toml` version on the `dev` branch** and merge to `main` via
> PR. CI will then automatically create the tag and Release. **Do not push the tag manually**,
> otherwise CI will skip the release workflow.

---

### CI Release Workflow

Release is performed automatically by GitHub Actions (`dist-release.yml`):

```
1. 在 dev 分支上更新 Cargo.toml 的 version 字段
2. cargo build 更新 Cargo.lock
3. 按发版格式 commit（见下方 🔖 发版提交）
   - commit message 必须包含自上次发版以来的所有变更（即 PR 的完整内容）
4. 从 dev 创建 PR 到 main
5. 合并 PR 到 main
6. gate job 自动检测：
   - 读取 Cargo.toml 版本号 → "v{version}"
   - 检查该 tag 是否已存在
   - 不存在 → 触发完整 release 流程
   - 已存在 → 跳过（不会重复发布）
7. 门禁：安全审计 + fmt/clippy/单测/doc test 全过
8. 全过之后：创建并推送 tag
9. tag 后构建与发布：
   - cargo-dist 跨平台构建（Linux/Windows/macOS 五平台）+ 重组包（bin/ + lib/）+ wasm + Inno 向导
   - 发布 GitHub Release（body 由 generate-commit-list.ts 生成）；apt 仓库另发 GitHub Pages
```

#### Key Rules

| Rule                                           | Description                                                                                                                                                                       |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Do not push the tag manually**               | Release only triggers from the gate on push to main; manually pushed tags won't trigger a release (to re-publish, use `dist-release.yml`'s workflow_dispatch and specify the tag) |
| **Bump version on dev**                        | The release commit is completed on dev, then merged to main via PR                                                                                                                |
| **Release commit includes the full changelog** | The commit message must contain all changes for this release, since it becomes the PR description source                                                                          |
| **Do not merge main back into dev**            | After the PR is merged, dev auto-syncs; no need to reverse merge                                                                                                                  |

---

## Message Conventions

### Language Conventions

**Chinese commit messages are recommended** to maintain team communication consistency.

- Use Chinese for the Subject; keep it concise and clear
- Use Chinese for the Body to provide detail
- Keep English for special technical terms when needed

### Subject

- Use Chinese, concise and clear
- No more than 50 characters
- No period at the end

### Body

- Explain why and how the change was made
- No more than 72 characters per line
- Use `-` or `*` for bullet points

### Footer

- **Breaking change**: Start with `BREAKING CHANGE:`
- **Close an issue**: Use `Closes #123` or `Fixes #456`

---

## Examples

### ✨ feat - New feature

```
:sparkles: feat(parser): 添加闭包语法解析支持

实现闭包表达式解析：
- 支持 |args| body 简写语法
- 支持 move 语义捕获
- 添加闭包类型推断

关闭 #42
```

### 🐛 fix - Bug fix

```
:bug: fix(repl): 修复多行输入时补全器失效的问题

SessionREPL 在多行模式下未正确注册补全器，
导致 Tab 补全无法触发。

修复 #128
```

### 📝 docs - Documentation update

```
:memo: docs(design): 更新所有权模型与类型系统规范

同步 RFC-009 和 RFC-011 的最新设计变更。
```

### ♻️ refactor - Refactor

```
:recycle: refactor(typecheck): 分离原语值类型与 Dup 浅拷贝语义

将 MonoType 中的值类型和拷贝语义解耦，
消除 match 分支中的特殊情况。
```

### ⚡️ perf - Performance optimization

```
:zap: perf(types): 优化 const generic 求值性能

为递归求值添加深度限制（默认 128），
避免恶意构造的类型表达式导致栈溢出。
```

### ✅ test - Tests

```
:white_check_mark: test(typecheck): 补充 scope VarInfo 可变性测试

覆盖场景：
- 不可变绑定的只读访问
- mut 绑定的可变性追踪
- 跨作用域的可变性传播
```

### 🔧 chore - Misc

```
:wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap

升级 6 个生产依赖至最新稳定版本。
```

### 🚀 ci - CI configuration

```
:rocket: ci: 修复 nightly 构建 Rust 版本过低的问题

将 RUST_TOOLCHAIN 从 1.91.0 更新至 1.96.0，
匹配 Cargo.toml 中的 rust-version 要求。
```

### 💄 style - Formatting

```
:lipstick: style(frontend): 应用 cargo fmt 格式化

统一函数签名的换行风格。
```

---

---

## 🔖 Release Commits

When the commit is a **release**, the following rules MUST be followed:

### Release Commit Format

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



### Release Requirements

1. **Message header**: MUST use the `:bookmark:` + `V<version>` format
2. **Version**: Follow semantic versioning
3. **Content completeness**: MUST include **all commits** since the last release
4. **Categorize by type**: Organize by `feat`, `fix`, `refactor`, `chore`, etc.

### Release Example

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

### Reference Template

Refer to the [`release.md`](release.md) template for the format when writing release notes.

---

### 1. Set Up a Commit Template

```bash
# Run in the project root
git config commit.template .gitmessage.txt
```

### 2. Template File

The format of `.gitmessage.txt` at the project root is as follows:

```
# emoji_code type(scope): Subject (Chinese)
#
# Body (optional)
#
# Footer (optional)
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: frontend, parser, lexer, typecheck, types, middle, codegen,
#         monomorphize, lifetime, backends, repl, runtime, proof,
#         std, formatter, lsp, package, util, docs, design, plan, rfc,
#         build, ci, test, release, meta, deps, examples, vscode, benches
#
# Examples:
# ✨ feat(db): Add bulk delete todo feature
# 🐛 fix(provider): Fix timer background recovery issue
#
# Release format: 🔖 V1.0.0: Release title
```

---

## FAQ

### Q: How to choose a commit type?

- **feat**: A change that the user can see
- **fix**: Fix an issue reported by the user
- **docs**: README, comments, etc.
- **chore**: Dependency updates, configuration files
- **refactor**: Code optimization that doesn't change behavior

### Q: When should a commit be split?

- Each commit should do **one thing**
- Commit related features together, separate unrelated ones
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [Emoji 完整列表](#完整-emoji-参考)
- [release.md](release.md) - Release template

---

> 💡 **Tip**: Keep commits atomic and clearly described, making code review and history navigation
> more efficient!
