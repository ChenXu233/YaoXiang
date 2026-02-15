---
title: RFC-015：YaoXiang 配置系统设计
---

# RFC-015: YaoXiang 配置系统设计

> **状态**: 审核中
> **作者**: 晨煦
> **创建日期**: 2026-02-12
> **最后更新**: 2026-02-15

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

### yaoxiang config 命令

提供 CLI 命令管理配置：

```bash
# 初始化用户级配置（按默认选项生成）
yaoxiang config init

# 编辑用户级配置（打开编辑器）
yaoxiang config edit

# 查看当前配置（合并后的有效配置）
yaoxiang config show

# 查看配置来源
yaoxiang config show --source

# 重置为默认配置
yaoxiang config reset
```

**首次运行**：用户首次运行任何 `yaoxiang` 命令时，自动检测用户级配置是否存在。若不存在，按默认选项自动生成。

**配置文件位置**：
- 项目级：`./yaoxiang.toml`（项目根目录）
- 用户级：`~/.config/yaoxiang/config.toml`

### 配置合并语义

不同层级的配置按以下规则合并：

| 类型 | 策略 | 说明 |
|------|------|------|
| 标量 (String/Number/Boolean) | 替换 | 项目级覆盖用户级 |
| 数组 (Array) | 替换 | 项目级完全替换用户级 |
| 对象 (Object) | 深度合并 | 逐字段合并，未定义字段继承下层 |

**示例 - 对象深度合并**：
```toml
# 用户级
[lint]
rules = ["recommended"]
severity = "warn"

# 项目级
[lint]
strict = true

# 合并结果
[lint]
rules = ["recommended"]    # 来自用户级
severity = "warn"          # 来自用户级
strict = true             # 来自项目级
```

### 向后兼容性

- ✅ 现有无配置文件模式继续支持（所有组件使用内置默认值）
- ✅ 新增配置项均有合理默认值
- ✅ 用户首次运行命令时自动按默认选项生成配置
- ✅ 配置解析失败时显示友好错误，提示具体行号和错误原因

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
| **Phase 1** | 基础配置解析器、toml 支持、项目级配置、`yaoxiang config init` |
| **Phase 2** | 用户级配置、配置合并逻辑、`yaoxiang config edit/show` |
| **Phase 3** | 命令行/环境变量覆盖、`platform` 平台约束、`[tool.*]` 扩展 |

### 依赖关系

- 依赖 RFC-014 包管理系统

### 风险

| 风险 | 缓解措施 |
|------|----------|
| 配置项过多 | 提供合理默认值，用户无感 |
| 解析器复杂 | 使用现有 toml 库 |

## 开放问题

- [x] `features` 条件编译语法？ → **移至单独 RFC**，依赖 RFC-011 泛型系统
- [x] `workspace` 工作空间设计？ → **移至单独 RFC**，复杂度高，需独立设计

### 已接受功能（第三阶段）

#### `platform` 平台约束

> **注意**：以下语法用于 `yaoxiang.toml` **配置文件**，**不是** YaoXiang 源代码 (`.yx` 文件) 中的语法。用户无需在代码中写 `cfg(...)`。

支持基于目标操作系统/架构的平台特定配置：

```toml
# yaoxiang.toml（配置文件）

[target.'cfg(windows)'.build]
output = "dist/win32"

[target.'cfg(unix)'.build]
output = "dist/unix"

[target.'cfg(target_arch = "x86_64")'.build]
rustflags = ["-C target-cpu=native"]
```

**语法**：`[target.'<条件>'.<配置节>]`

**说明**：
- 此语法仅出现在 `yaoxiang.toml` 配置文件中
- 构建时根据 `--target` 参数选择对应的配置
- 用户在 `.yx` 源代码中**无需**、**也不应该**写 `cfg(...)` 语法

**支持的条件**：
- `cfg(os = "windows")` - Windows 系统
- `cfg(os = "linux")` - Linux 系统
- `cfg(os = "macos")` - macOS 系统
- `cfg(target_arch = "x86_64")` - 64 位 x86 架构
- `cfg(target_arch = "aarch64")` - ARM 64 位架构

#### `[tool.*]` 第三方工具配置扩展

允许第三方工具在 `[tool.<名称>]` 下存储配置：

```toml
[tool.eslint]
extension = ["yx", "yxp"]
ignore = ["node_modules/", "dist/"]

[tool.prettier]
semi = false
singleQuote = true
```

**行为**：
- YaoXiang 忽略未知的 `[tool.*]` 节，但会保留在配置文件中
- 第三方工具可通过 `yaoxiang tool run <名称>` 集成或直接访问
- 工具特定配置不进行验证

---

## 参考文献

- [Cargo Manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [deno.json](https://deno.com/manual@v1.28.3/getting-started/configuration_file)
- [npm package.json](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)
