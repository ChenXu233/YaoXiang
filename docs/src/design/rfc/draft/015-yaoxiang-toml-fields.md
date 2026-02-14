---
title: RFC-015：YaoXiang 配置系统设计
---

# RFC-015: YaoXiang 配置系统设计

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2026-02-12
> **最后更新**: 2026-02-14

> **前置 RFC**: [RFC-014: 包管理系统设计](014-package-manager.md)

## 摘要

设计 YaoXiang 语言的统一配置系统，支持用户级和项目级两个层级，为包管理器、编译器、REPL、LSP 等组件提供共享的配置基础设施。

## 动机

### 为什么需要这个特性/变更？

YaoXiang 工具链包含多个组件：
- 包管理器（读取依赖配置）
- 编译器前端（读取 i18n 配置）
- REPL（读取交互配置）
- LSP（读取 fmt/lint/test 配置）
- 构建系统（读取构建配置）

各组件需要统一的配置基础设施。

### 当前的问题

- 各组件配置分散、无统一规范
- 用户无法统一管理偏好设置
- 项目配置与用户配置无明确层级

## 提案

### 核心设计

**分层架构**：
```
配置优先级（高 → 低）：
┌─────────────────────────────────────────────┐
│ 1. 项目级 yaoxiang.toml                      │ ← 项目团队控制
├─────────────────────────────────────────────┤
│ 2. 用户级 ~/.config/yaoxiang/config.toml     │ ← 用户偏好
├─────────────────────────────────────────────┤
│ 3. 编译器默认值                              │ ← 合理初始值
└─────────────────────────────────────────────┘
```

**规则**：上层覆盖下层，未配置的选项回退到下层。

### 配置层级限制

| 配置节 | 用户级 | 项目级 | 消费方 |
|--------|--------|--------|--------|
| `[package].*` | ❌ | ✅ | 包管理器 |
| `[yaoxiang]` | ❌ | ✅ | 编译器 |
| `[dependencies]` | ❌ | ✅ | 包管理器 |
| `[dev-dependencies]` | ❌ | ✅ | 包管理器 |
| `[bin]` | ❌ | ✅ | 包管理器 |
| `[lib]` | ❌ | ✅ | 包管理器 |
| `[build]` | ✅ | ✅ | 构建系统 |
| `[profile.*]` | ✅ | ✅ | 构建系统 |
| `[install]` | ✅ | ❌ | 包管理器 |
| `[i18n]` | ✅ | ✅ | 编译器 |
| `[repl]` | ✅ | ✅ | REPL |
| `[fmt]` | ✅ | ✅ | LSP |
| `[lint]` | ✅ | ✅ | LSP |
| `[test]` | ✅ | ✅ | LSP |
| `[tasks]` | ✅ | ✅ | CLI |

### 示例

**项目级配置**：
```toml
# yaoxiang.toml
[package]
name = "my-package"
version = "0.1.0"

[yaoxiang]
version = ">=0.1.0, <1.0.0"

[dependencies]
foo = "^1.0.0"

[build]
output = "dist/"

[tasks]
build = "yaoxiang build"
test = "yaoxiang test"
```

**用户级配置**：
```toml
# ~/.config/yaoxiang/config.toml
[install]
dir = "~/.local/share/yaoxiang"

[i18n]
lang = "zh"
fallback = "en"

[repl]
history-size = 1000
prompt = "yx> "
colors = true

[fmt]
line-width = 120
indent-width = 4

[lint]
rules = ["recommended"]
```

## 详细设计

### 项目级专属配置

```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"
authors = ["Alice <alice@example.com>"]
license = "MIT"
repository = "https://github.com/alice/my-project"

[yaoxiang]
version = ">=0.1.0, <1.0.0"

[dependencies]
foo = "^1.0.0"

[dev-dependencies]
test-utils = "0.1.0"

[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./foo" = "src/foo.yx"

[build]
script = "build.yx"
output = "dist/"

[profile.release]
optimize = true
lto = true

[run]
main = "src/main.yx"
args = ["--quiet"]

[tasks]
build = "yaoxiang build"
test = "yaoxiang test"
lint = "yaoxiang fmt && yaoxiang check"
```

### 用户级专属配置

```toml
[install]
dir = "~/.local/share/yaoxiang"
```

### 两者皆可的配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `[i18n].lang` | String | "en" | 语言 |
| `[i18n].fallback` | String | "en" | 回退语言 |
| `[repl].history-size` | Number | 1000 | 历史条数 |
| `[repl].history-file` | Path | ~ | 历史文件 |
| `[repl].prompt` | String | "yx> " | 提示符 |
| `[repl].colors` | Boolean | true | 语法高亮 |
| `[repl].auto-imports` | [String] | [] | 自动导入 |
| `[fmt].line-width` | Number | 120 | 行宽 |
| `[fmt].indent-width` | Number | 4 | 缩进 |
| `[fmt].use-tabs` | Boolean | false | Tab 缩进 |
| `[fmt].single-quote` | Boolean | false | 单引号 |
| `[lint].rules` | [String] | ["recommended"] | 规则集 |
| `[lint].strict` | Boolean | false | 严格模式 |
| `[test].report` | String | "console" | 测试报告 |
| `[build].output` | String | "dist/" | 输出目录 |

### 命令行与环境变量覆盖

```bash
# 命令行覆盖
yaoxiang run main.yx --lang zh
yaoxiang fmt --config-indent-width=2

# 环境变量
export YAOXIANG_LANG=zh
export YAOXIANG_FMT_INDENT_WIDTH=2
```

**优先级**：`命令行 > 环境变量 > 配置文件`

### 向后兼容性

- ✅ 现有无配置文件模式继续支持
- ✅ 新增配置项均有合理默认值

## 权衡

### 优点

- 统一配置基础设施，减少重复代码
- 用户偏好跨项目一致
- LSP/REPL/编译器共享同一套配置
- 渐进式配置，按需声明

### 缺点

- 配置项较多，学习成本略增
- 需要统一的配置解析器

## 替代方案

| 方案 | 为什么没选 |
|------|-----------|
| 各组件独立配置 | 重复代码，用户体验割裂 |
| 仅支持命令行参数 | 无法持久化用户偏好 |
| 仅支持环境变量 | 项目配置难以版本控制 |

## 实现策略

### 阶段划分

| 阶段 | 内容 |
|------|------|
| **Phase 1** | 基础配置解析器、toml 支持、项目级配置 |
| **Phase 2** | 用户级配置、配置合并逻辑 |
| **Phase 3** | 命令行/环境变量覆盖 |

### 依赖关系

- 依赖 RFC-014 包管理系统

### 风险

| 风险 | 缓解措施 |
|------|----------|
| 配置项过多 | 提供合理默认值，用户无感 |
| 解析器复杂 | 使用现有 toml 库 |

## 开放问题

- [ ] `features` 条件编译语法？
- [ ] `platform` 平台约束是否需要？
- [ ] `workspace` 工作空间设计？
- [ ] `[tool.*]` 第三方工具配置扩展？

---

## 参考文献

- [Cargo Manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [deno.json](https://deno.com/manual@v1.28.3/getting-started/configuration_file)
- [npm package.json](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)
