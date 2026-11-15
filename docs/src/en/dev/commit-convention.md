# Commit Guidelines

This document defines the Git commit conventions for the YaoXiang project, with the goal of keeping
the commit history clear, readable, and easy to understand.

---

## Table of Contents

- [Commit Format](#提交格式)
- [Commit Types](#提交类型)
- [Commit Granularity](#提交粒度)
- [Complete Emoji Reference](#完整-emoji-参考)
- [Scope](#作用域)
- [Version Management](#版本管理)
- [Message Guidelines](#消息规范)
- [Language Guidelines](#语言规范)
- [🔖 Release Commit](#-发版提交)
- [Examples](#示例)
- [Using a Commit Template](#使用-commit-template)
- [FAQ](#常见问题)

---

## Commit Format

**EXTREMELY IMPORTANT!!!!!! DO NOT FORGET!!!** All commit messages must follow this format:

```
:emoji代码: type(scope): 主题（中文）

[可选的主体内容]

[可选的页脚]
```

> ⚠️ **Important**: You **must use emoji codes** (such as `:sparkles:`) rather than entering emoji
> characters directly.
>
> **Using Chinese commit messages is recommended** to keep team communication consistent.

### Components

| Component  | Description                                           | Required |
| ---------- | ----------------------------------------------------- | -------- |
| emoji code | Identifies the commit type with an emoji              | ✅       |
| type       | Commit type                                           | ✅       |
| scope      | Area affected by the change                           | ✅       |
| subject    | Brief description (in Chinese, 50 characters maximum) | ✅       |
| body       | Detailed explanation (optional)                       | ❌       |
| footer     | Breaking changes or issue closures (optional)         | ❌       |

---

## Commit Types

| Emoji Code              | type     | Description                            |
| ----------------------- | -------- | -------------------------------------- |
| :sparkles:              | feat     | New feature                            |
| :bug:                   | fix      | Fix a bug                              |
| :memo:                  | docs     | Documentation changes only             |
| :lipstick:              | style    | Code formatting (no functional impact) |
| :recycle:               | refactor | Refactor code                          |
| :zap:                   | perf     | Performance optimization               |
| :white_check_mark:      | test     | Add or modify tests                    |
| :wrench:                | chore    | Build tooling or utility changes       |
| :building_construction: | build    | Build system changes                   |
| :rocket:                | ci       | CI configuration changes               |

---

## Complete Emoji Reference

The following is the complete emoji list from the gitmoji project. Choose the appropriate emoji
based on the commit content:

| Emoji | Emoji Code                    | Commit Description                      |
| :---- | :---------------------------- | :-------------------------------------- |
| 🎨    | `:art:`                       | Improve code structure or formatting    |
| ⚡️    | `:zap:` / `:racehorse:`       | Improve performance                     |
| 🔥    | `:fire:`                      | Remove code or files                    |
| 🐛    | `:bug:`                       | Fix a bug                               |
| 🚑    | `:ambulance:`                 | Critical patch                          |
| ✨    | `:sparkles:`                  | Introduce a new feature                 |
| 📝    | `:memo:`                      | Write documentation                     |
| 🚀    | `:rocket:`                    | Deploy functionality                    |
| 💄    | `:lipstick:`                  | Update the UI and style files           |
| 🎉    | `:tada:`                      | Initial commit                          |
| ✅    | `:white_check_mark:`          | Add tests                               |
| 🔒    | `:lock:`                      | Fix a security issue                    |
| 🍎    | `:apple:`                     | Fix content on macOS                    |
| 🐧    | `:penguin:`                   | Fix content on Linux                    |
| 🏁    | `:checkered_flag:`            | Fix content on Windows                  |
| 🤖    | `:robot:`                     | Fix certain content on Android          |
| 🍏    | `:green_apple:`               | Solve certain issues on iOS             |
| 🔖    | `:bookmark:`                  | Release or version tag                  |
| 🚨    | `:rotating_light:`            | Remove linter warnings                  |
| 🚧    | `:construction:`              | Work in progress                        |
| 💚    | `:green_heart:`               | Fix CI build issues                     |
| ⬇️    | `:arrow_down:`                | Downgrade dependencies                  |
| ⬆️    | `:arrow_up:`                  | Upgrade dependencies                    |
| 📌    | `:pushpin:`                   | Pin dependencies to a specific version  |
| 👷    | `:construction_worker:`       | Add a CI build system                   |
| 📈    | `:chart_with_upwards_trend:`  | Add analytics or tracking code          |
| ♻️    | `:recycle:`                   | Refactor code                           |
| 🔨    | `:hammer:`                    | Major refactoring                       |
| ➖    | `:heavy_minus_sign:`          | Remove a dependency                     |
| 🐳    | `:whale:`                     | Docker-related work                     |
| ➕    | `:heavy_plus_sign:`           | Add a dependency                        |
| 🔧    | `:wrench:`                    | Modify configuration files              |
| 🌐    | `:globe_with_meridians:`      | Internationalization and localization   |
| ✏️    | `:pencil2:`                   | Fix a typo                              |
| 💩    | `:hankey:`                    | Write error code that needs improvement |
| ⏪️    | `:rewind:`                    | Revert changes                          |
| 🔀    | `:twisted_rightwards_arrows:` | Merge branches                          |
| 📦    | `:package:`                   | Update compiled files or packages       |
| 👽    | `:alien:`                     | Update code due to external API changes |
| 🚚    | `:truck:`                     | Move or rename files                    |
| 📄    | `:page_facing_up:`            | Add or update licenses                  |
| 💥    | `:boom:`                      | Introduce breaking changes              |
| 🍱    | `:bento:`                     | Add or update assets                    |
| 👌    | `:ok_hand:`                   | Update code due to code review changes  |
| ♿️    | `:wheelchair:`                | Improve accessibility                   |
| 💡    | `:bulb:`                      | Document source code                    |
| 🍻    | `:beers:`                     | Write code in a drunken stupor          |
| 💬    | `:speech_balloon:`            | Update text and copy                    |
| 🗃️    | `:card_file_box:`             | Make database-related changes           |
| 🔊    | `:loud_sound:`                | Add logs                                |
| 🔇    | `:mute:`                      | Remove logs                             |
| 👥    | `:busts_in_silhouette:`       | Add contributors                        |
| 🚸    | `:children_crossing:`         | Improve user experience or usability    |
| 🏗️    | `:building_construction:`     | Make architectural changes              |
| 📱    | `:iphone:`                    | Work on responsive design               |
| 🤡    | `:clown_face:`                | Mock things                             |
| 🥚    | `:egg:`                       | Add an Easter egg                       |
| 🙈    | `:see_no_evil:`               | Add or update the .gitignore file       |
| 📸    | `:camera_flash:`              | Add or update snapshots                 |

---

## Commit Granularity

One commit = one understandable reason + one independently revertible unit.

- **No bookkeeping-only commits**: Changes with no substantive information, such as checking a box,
  adjusting a number or count, or fixing a typo, should be folded into an adjacent feature commit or
  wrap-up commit rather than committed separately.
- **Substantive documentation follows the change**: Corrections to an RFC, specification, or
  contract description must be committed together with their corresponding code changes.
- **Generated artifacts follow the code**: Documentation regenerated by a generator should be folded
  into the feature commit to which it belongs; documentation tables reconciled against gates from
  multiple sources must be committed together with their code.
- **Wrap-up documentation lands last**: WBS check-offs, records (`09a`), and registration pointers
  should be accumulated into the final commit of that push. Do not create a separate commit for “a
  single check-off” or “a single registration.”
- **WBS records plans only**: The close-out entry for a WBS line should say only “Completed + `09a`
  pointer”; all measured values, probe results, and acceptance-criteria details belong in `09a`.
- **Obtain the commit hash after pushing**: `pull --rebase` rewrites local hashes, and any commit
  referenced in documentation must come from the remote after the push.

### Commit Window

`pre-commit` stashes all unstaged changes before running the hooks and restores them after the hooks
finish (logs: `Stashing unstaged files to …` / `Restored changes from …`). While a commit is in
progress, others reading the working tree will see the misleading state where “only staged files
remain”: during this period, measure only against the frozen sha and do not read the working tree;
after each commit completes, use `git status` to verify that the remaining changes are intact.

---

## Scope

Scopes are based on the repository directory structure. **You must use one of the following defined
scopes**:

### Top-Level Modules

| Scope       | Corresponding Directory              | Description                                                                                   |
| ----------- | ------------------------------------ | --------------------------------------------------------------------------------------------- |
| `frontend`  | `src/frontend/`                      | Frontend: lexing, parsing, and type checking                                                  |
| `middle`    | `src/middle/`                        | Middle layer: IR, optimization, and monomorphization                                          |
| `backends`  | `src/backends/`                      | Backends: interpreter, runtime, and REPL                                                      |
| `std`       | `src/std/`                           | Standard library                                                                              |
| `formatter` | `src/formatter/`                     | Code formatter                                                                                |
| `lsp`       | `src/lsp/`                           | Language Server Protocol                                                                      |
| `package`   | `src/package/`                       | Package manager                                                                               |
| `util`      | `src/util/`                          | Utility libraries: diagnostics, caching, and i18n                                             |
| `proof`     | `src/frontend/core/typecheck/proof/` | Proof and verification support (promoted to a top-level domain in the final state of RFC-039) |
| `driver`    | `src/driver/`                        | L1 orchestration layer: unified Driver and phase contracts (established in RFC-039 P4)        |

### Front-End Submodules

| Scope       | Corresponding Directory        | Description             |
| ----------- | ------------------------------ | ----------------------- |
| `parser`    | `src/frontend/core/parser/`    | Syntax parser           |
| `lexer`     | `src/frontend/core/lexer/`     | Lexer                   |
| `typecheck` | `src/frontend/core/typecheck/` | Type checking           |
| `types`     | `src/frontend/core/types/`     | Type system definitions |

### Middle-Layer Submodules

| Scope          | Corresponding Directory       | Description                                                                        |
| -------------- | ----------------------------- | ---------------------------------------------------------------------------------- |
| `codegen`      | `src/middle/passes/codegen/`  | Code generation (bytecode)                                                         |
| `monomorphize` | `src/middle/passes/mono/`     | Monomorphization processing (the directory rename follows the RFC-039 P6 decision) |
| `lifetime`     | `src/middle/passes/lifetime/` | Lifetime analysis                                                                  |

### Backend Submodules

| Scope     | Corresponding Directory | Description                   |
| --------- | ----------------------- | ----------------------------- |
| `repl`    | `src/repl/`             | Interactive REPL command line |
| `runtime` | `src/backends/runtime/` | Runtime execution engine      |

### Documentation Scopes

| Scope    | Description                                           |
| -------- | ----------------------------------------------------- |
| `docs`   | General documentation updates                         |
| `design` | Language design specifications (docs/src/dev/design/) |
| `plan`   | Implementation plan documentation                     |
| `rfc`    | RFC documentation (docs/src/rfc/)                     |

### Other Scopes

| Scope      | Description                                                    |
| ---------- | -------------------------------------------------------------- |
| `build`    | Build system and Cargo configuration                           |
| `ci`       | CI/CD configuration (GitHub Actions)                           |
| `test`     | Test-related changes                                           |
| `release`  | Release-related changes                                        |
| `meta`     | Project meta-configuration (.claude, .gitignore, etc.)         |
| `deps`     | Dependency upgrades (the same naming convention as dependabot) |
| `examples` | Example directory `examples/`                                  |
| `vscode`   | VS Code extension `vscode-extension/`                          |
| `benches`  | Benchmarks `benches/`                                          |

---

## Message Guidelines

### Version Management

The version number is defined in the `version` field of `Cargo.toml` at the project root:

```toml
[package]
version = "0.7.2"
```

Semantic versioning `MAJOR.MINOR.PATCH` is used:

| Version Type | Description                                | Example       |
| ------------ | ------------------------------------------ | ------------- |
| **major**    | Major update with incompatible API changes | 0.7.2 → 1.0.0 |
| **minor**    | New feature with backward compatibility    | 0.7.2 → 0.8.0 |
| **patch**    | Bug fix with backward compatibility        | 0.7.2 → 0.7.3 |

> ⚠️ When releasing, **update the version number in `Cargo.toml` on the dev branch**, merge the PR
> into main, and CI will automatically create the tag and Release. **Do not push the tag manually**,
> or CI will skip the release process.

---

## CI Release Process

Releases are completed automatically by GitHub Actions (`dist-release.yml`). The process is as
follows:

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

### Key Rules

| Rule                                                   | Description                                                                                                                                                                                                    |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Do not push tags manually**                          | A release relies only on the gate decision made after pushing to main; a manually pushed tag will not trigger a release (to re-release, use the `workflow_dispatch` of `dist-release.yml` and specify the tag) |
| **Bump the version on dev**                            | The release commit is completed on dev and merged into main through a PR                                                                                                                                       |
| **The release commit contains the complete changelog** | The commit message must include all changes in the release because it is the source of the PR description                                                                                                      |
| **Do not merge main back into dev**                    | Dev is synchronized automatically after the PR is merged, so no reverse merge is needed                                                                                                                        |

---

## Message Guidelines

### Language Guidelines

**Using Chinese commit messages is recommended** to keep team communication consistent.

- Use Chinese for the Subject; keep it concise and clear
- The Body may provide detailed explanations in Chinese
- Special technical terms may remain in English

### Subject

- Use Chinese and keep it concise and clear
- No more than 50 characters
- Do not end with a period

### Body

- Explain the reasons and methods for the changes in detail
- No more than 72 characters per line
- Use `-` or `*` to list key points

### Footer

- **Breaking changes**: Begin with `BREAKING CHANGE:`
- **Close an issue**: Use `关闭 #123` or `修复 #456`

---

## Examples

### ✨ feat - New Feature

```
:sparkles: feat(parser): 添加闭包语法解析支持

实现闭包表达式解析：
- 支持 |args| body 简写语法
- 支持 move 语义捕获
- 添加闭包类型推断

关闭 #42
```

### 🐛 fix - Bug Fix

```
:bug: fix(repl): 修复多行输入时补全器失效的问题

SessionREPL 在多行模式下未正确注册补全器，
导致 Tab 补全无法触发。

修复 #128
```

### 📝 docs - Documentation Update

```
:memo: docs(design): 更新所有权模型与类型系统规范

同步 RFC-009 和 RFC-011 的最新设计变更。
```

### ♻️ refactor - Refactoring

```
:recycle: refactor(typecheck): 分离原语值类型与 Dup 浅拷贝语义

将 MonoType 中的值类型和拷贝语义解耦，
消除 match 分支中的特殊情况。
```

### ⚡️ perf - Performance Optimization

```
:zap: perf(types): 优化 const generic 求值性能

为递归求值添加深度限制（默认 128），
避免恶意构造的类型表达式导致栈溢出。
```

### ✅ test - Testing

```
:white_check_mark: test(typecheck): 补充 scope VarInfo 可变性测试

覆盖场景：
- 不可变绑定的只读访问
- mut 绑定的可变性追踪
- 跨作用域的可变性传播
```

### 🔧 chore - Miscellaneous

```
:wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap

升级 6 个生产依赖至最新稳定版本。
```

### 🚀 ci - CI Configuration

```
:rocket: ci: 修复 nightly 构建 Rust 版本过低的问题

将 RUST_TOOLCHAIN 从 1.91.0 更新至 1.96.0，
匹配 Cargo.toml 中的 rust-version 要求。
```

### 💄 style - Formatting Adjustments

```
:lipstick: style(frontend): 应用 cargo fmt 格式化

统一函数签名的换行风格。
```

---

---

## 🔖 Release Commit

When the current commit is a **release**, it must follow the conventions below:

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

1. **Message header**: Must use the `:bookmark:` + `V<版本号>` format
2. **Version number**: Must follow semantic versioning conventions
3. **Content completeness**: Must include an introduction to **all commit** content since the previous release
4. **Categorization by type**: Organize the content by types such as `feat`, `fix`, `refactor`, and `chore`

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

Refer to the format of the [`release.md`](release.md) template when writing a release document.

---

### 1. Set Up the Commit Template

```bash
# Run at the project root
git config commit.template .gitmessage.txt
```

### 2. Template File

The `.gitmessage.txt` file at the project root should have the following format:

```
# emoji code type(scope): Subject (Chinese)
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
# Example:
# ✨ feat(db): Add batch deletion for to-do items
# 🐛 fix(provider): Fix timer resumption in the background
#
# Release format: 🔖 V1.0.0: Release title
```

---

## FAQ

### Q: How should I choose a commit type?

- **feat**: User-visible feature changes
- **fix**: Fixes for issues reported by users
- **docs**: Documentation such as READMEs and comments
- **chore**: Dependency updates and configuration files
- **refactor**: Code optimizations that do not change behavior

### Q: When should I split a commit?

- Each commit should do only **one thing**
- Commit related functionality together and unrelated changes separately
- Follow the Atomic Commits principle

---

## References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [Complete Emoji List](#完整-emoji-参考)
- [release.md](release.md) - Release template

---

> 💡 **Tip**: Keep commits atomic and descriptions clear to make code review and retrospectives more
> efficient!
