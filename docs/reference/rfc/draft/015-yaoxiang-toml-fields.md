# RFC-015: yaoxiang.toml 配置字段研究

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2026-02-12
> **最后更新**: 2026-02-12

> **前置 RFC**: [RFC-014: 包管理系统设计](014-package-manager.md)

## 摘要

深入研究 `yaoxiang.toml` 配置文件的字段设计，对比主流语言生态的配置文件规范，制定符合 YaoXiang 语言特性的配置字段集。

### 分层配置架构

```
优先级（高 → 低）：
1. 项目级 yaoxiang.toml
2. 用户级 ~/.config/yaoxiang/config.toml
3. 编译器默认值
```

### i18n 支持

```toml
[i18n]
lang = "zh"        # en / zh / zh-x-miao
fallback = "en"    # 翻译缺失时的回退语言
```

## 1. 字段研究

### 1.1 Cargo.toml 分析

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
authors = ["Alice <alice@example.com>"]
description = "A short description"
documentation = "https://docs.rs/my-crate"
homepage = "https://github.com/alice/my-crate"
repository = "https://github.com/alice/my-crate"
license = "MIT OR Apache-2.0"
license-file = "LICENSE"
readme = "README.md"
keywords = ["cli", "parsing"]
categories = ["command-line-utilities", "parser-implementations"]

[features]
default = ["std"]
std = []
docsrs = []

[dependencies]
foo = "1.0"

[dev-dependencies]
test-utils = "1.0"

[build-dependencies]
cc = "1.0"

[workspace]
members = ["crate-a", "crate-b"]
exclude = ["temp/"]

[profile.release]
opt-level = 3
lto = true
```

**关键字段**：
- `edition`: Rust 版本特性集
- `features`: 条件编译特性
- `profile.*`: 构建优化配置

### 1.2 package.json 分析

```json
{
  "name": "my-project",
  "version": "1.0.0",
  "description": "A short description",
  "main": "dist/index.js",
  "module": "dist/index.mjs",
  "types": "dist/index.d.ts",
  "exports": {
    ".": {
      "import": "./dist/index.mjs",
      "require": "./dist/index.js"
    },
    "./foo": "./dist/foo.js"
  },
  "scripts": {
    "build": "tsc",
    "test": "jest"
  },
  "keywords": ["cli", "parser"],
  "author": "Alice <alice@example.com>",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/alice/my-project.git"
  },
  "bugs": {
    "url": "https://github.com/alice/my-project/issues"
  },
  "homepage": "https://github.com/alice/my-project#readme",
  "engines": {
    "node": ">=16.0.0"
  },
  "os": ["darwin", "linux"],
  "cpu": ["x64", "arm64"],
  "dependencies": {
    "foo": "^1.0.0"
  },
  "devDependencies": {
    "test-utils": "^1.0.0"
  },
  "peerDependencies": {
    "react": ">=17.0.0"
  },
  "bundledDependencies": [],
  "optionalDependencies": {},
  "publishConfig": {
    "registry": "https://npm.pkg.github.com/"
  }
}
```

**关键字段**：
- `exports`: 条件导出（ESM/CommonJS）
- `engines`: 运行时版本要求
- `os`/`cpu`: 平台限制
- `peerDependencies`: 同位依赖
- `publishConfig`: 发布配置

### 1.3 pyproject.toml 分析

```toml
[project]
name = "my-package"
version = "0.1.0"
description = "A short description"
authors = [
    { name = "Alice", email = "alice@example.com" }
]
license = { text = "MIT" }
readme = "README.md"
requires-python = ">=3.8"
keywords = ["cli", "parser"]
classifiers = [
    "Development Status :: 4 - Beta",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: MIT License",
    "Programming Language :: Python :: 3"
]
dependencies = [
    "requests>=2.25.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0.0",
    "black>=22.0.0",
]

[project.scripts]
my-script = "my_package.cli:main"

[tool.black]
line-length = 88
target-version = ['py38']

[tool.pytest.ini_options]
testpaths = ["tests"]
```

**关键字段**：
- `classifiers`: PyPI 分类标签
- `requires-python`: Python 版本要求
- `project.scripts`: 入口脚本定义
- `[tool.*]`: 工具配置节

### 1.4 go.mod 分析

```mod
module github.com/alice/my-project

go 1.21

require (
    github.com/foo/bar v1.2.3
    github.com/baz/qux v0.5.0
)

require github.com/example/pkg v1.0.0 // indirect

replace github.com/foo/bar => ./local-bar

exclude github.com/foo/bad v1.0.0
```

**关键字段**：
- `replace`: 本地路径替换
- `exclude`: 排除特定版本
- `// indirect`: 间接依赖标记

### 1.5 deno.json 分析

```json
{
  "name": "@alice/my-project",
  "version": "0.1.0",
  "exports": "./mod.ts",
  "imports": {
    "foo": "https://deno.land/x/foo@v1.0.0/mod.ts"
  },
  "tasks": {
    "build": "deno run --allow-all build.ts",
    "test": "deno test"
  },
  "fmt": {
    "lineWidth": 80,
    "indentWidth": 2,
    "useTabs": false,
    "singleQuote": true
  },
  "lint": {
    "rules": {
      "tags": ["recommended"]
    }
  },
  "test": {
    "files": {
      "include": ["test/"]
    }
  },
  "nodeModulesDir": "none",
  "compilerOptions": {
    "strict": true,
    "jsx": "react-jsx"
  }
}
```

**关键字段**：
- `imports`: URL 导入映射
- `tasks`: 自定义任务
- `fmt`/`lint`/`test`: 工具配置
- `nodeModulesDir`: 节点模块目录控制

### 1.6 对比总结

| 字段 | Cargo | package.json | pyproject.toml | go.mod | deno.json | 建议 |
|------|-------|--------------|----------------|--------|-----------|------|
| name | ✅ | ✅ | ✅ | ✅ (module) | ✅ | ✅ 必需 |
| version | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ 必需 |
| description | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ 推荐 |
| authors | ✅ | ✅ (author) | ✅ | ❌ | ❌ | ✅ 推荐 |
| license | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ 推荐 |
| repository | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ 推荐 |
| homepage | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ 推荐 |
| keywords | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ 可选 |
| readme | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ 可选 |
| edition | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ YaoXiang 版本 |
| features | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ 可选 |
| exports | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ 可选 |
| dependencies | ✅ | ✅ | ✅ | ✅ | (imports) | ✅ 必需 |
| dev-dependencies | ✅ | ✅ | ✅ | ❌ (// indirect) | ❌ | ✅ 可选 |
| workspace | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ Phase 4 |
| engines | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ 可选 |

## 2. 字段设计建议

### 2.0 分层配置架构

yaoxiang.toml 支持**用户级**和**项目级**两个配置层级：

```
┌─────────────────────────────────────────────────────────┐
│                    配置优先级（高→低）                    │
├─────────────────────────────────────────────────────────┤
│  1. 项目级 yaoxiang.toml                                 │
│  2. 用户级 ~/.config/yaoxiang/config.toml               │
│  3. 编译器默认值                                         │
└─────────────────────────────────────────────────────────┘
```

**配置回退规则**：
- 项目级配置**覆盖**用户级配置
- 项目级未配置的选项，使用用户级配置
- 用户级未配置的选项，使用编译器默认值

#### 2.0.1 用户级配置示例

```toml
# ~/.config/yaoxiang/config.toml

[i18n]
lang = "zh"                    # 默认语言

[repl]
history-size = 1000            # REPL 历史条数
history-file = "~/.yaoxiang_history"
prompt = "yx> "                # 提示符
colors = true                  # 语法高亮
auto-imports = ["std"]        # 自动导入模块

[fmt]
line-width = 120
indent-width = 4
use-tabs = false
single-quote = true

[lint]
rules = ["recommended"]
strict = false

[build]
output = "dist/"              # 构建输出目录

[install]
dir = "~/.local/share/yaoxiang"  # 全局安装目录
```

#### 2.0.2 项目级配置示例

```toml
# yaoxiang.toml

[package]
name = "my-project"
version = "0.1.0"

[yaoxiang]
version = ">=0.1.0, <1.0.0"

[i18n]
# 继承用户级配置，如不配置则使用 zh
lang = "zh"                    # 覆盖用户级设置

[repl]
# 使用用户级的 history-size，但覆盖 welcome-message
welcome-message = "Welcome to My Project!"

[dependencies]
std = "0.1.0"
```

#### 2.0.3 字段层级限制

| 字段 | 用户级 | 项目级 | 说明 |
|------|--------|--------|------|
| `[package].*` | ❌ | ✅ | 包元信息只能在项目级 |
| `[yaoxiang]` | ❌ | ✅ | 语言版本只能在项目级 |
| `[dependencies]` | ❌ | ✅ | 依赖声明只能在项目级 |
| `[dev-dependencies]` | ❌ | ✅ | 开发依赖只能在项目级 |
| `[bin]` | ❌ | ✅ | 二进制配置只能在项目级 |
| `[lib]` | ❌ | ✅ | 库配置只能在项目级 |
| `[build]` | ✅ | ✅ | 构建配置（项目覆盖用户） |
| `[install]` | ✅ | ❌ | 全局安装配置只能在用户级 |
| `[repl]` | ✅ | ✅ | REPL 配置（项目覆盖用户） |
| `[i18n]` | ✅ | ✅ | 国际化配置（项目覆盖用户） |
| `[fmt]` | ✅ | ✅ | 格式化配置（项目覆盖用户） |
| `[lint]` | ✅ | ✅ | Lint 配置（项目覆盖用户） |
| `[test]` | ✅ | ✅ | 测试配置（项目覆盖用户） |
| `[tasks]` | ✅ | ✅ | 任务配置（项目覆盖用户） |

#### 2.0.4 【创新】i18n 国际化配置

```toml
# 项目级或用户级

[i18n]
lang = "zh"                    # 当前语言：en / zh / zh-x-miao
fallback = "en"               # 翻译缺失时的回退语言
```

**使用方式**：
```bash
# 命令行覆盖
yaoxiang run main.yx --lang zh
yaoxiang -L zh run main.yx

# 环境变量覆盖
export YAOXIANG_LANG=zh

# 配置文件（优先级：命令行 > 环境变量 > 配置文件）
```

### 2.1 必需字段

```toml
[package]
name = "my-package"       # 包名（小写，推荐 kebab-case）
version = "0.1.0"        # 语义化版本
```

### 2.2 推荐字段

```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"
authors = ["Alice <alice@example.com>"]
license = "MIT"
repository = "https://github.com/alice/my-project"
homepage = "https://alice.github.io/my-project"
documentation = "https://docs.alice.github.io/my-project"
keywords = ["cli", "parser"]
readme = "README.md"

[yaoxiang]
# edition 使用 SemVer，支持版本范围
version = ">=0.1.0, <1.0.0"    # 要求语言版本 >= 0.1.0 且 < 1.0.0
# 或使用通配符
version = "0.x"                # 要求 0.x 版本
```

### 2.3 依赖声明

```toml
[dependencies]
# 简单版本约束
foo = "1.2.3"
bar = "^1.0.0"
baz = "~1.2.0"

# Git 依赖
qux = { git = "https://github.com/user/qux", version = "0.5.0" }
qux = { git = "https://github.com/user/qux", tag = "v0.5.0" }
qux = { git = "https://github.com/user/qux", branch = "main" }

# 本地路径
local = { path = "./local-module" }

# 注册表依赖（Phase 3）
registry-pkg = "1.0.0"

[dev-dependencies]
test-utils = "0.1.0"

[build-dependencies]
build-script = { path = "./build" }
```

### 2.4 构建配置

```toml
[build]
script = "build.yx"           # 构建脚本（可选）
output = "dist/"              # 构建输出目录

[profile.release]
optimize = true                # 优化级别
lto = true                     # 链接时优化
debug = false                  # 调试信息
```

### 2.5 工具配置

```toml
[fmt]
line-width = 120
indent-width = 4
use-tabs = false
single-quote = true

[lint]
rules = ["recommended"]
strict = false

[test]
files = ["tests/"]
report = "junit"
```

### 2.5 内建任务配置

借鉴 package.json scripts 设计，将常用命令内建到 yaoxiang.toml：

```toml
[tasks]
# 内建任务，可覆盖
build = "yaoxiang build"
test = "yaoxiang test"
bench = "yaoxiang bench"
clean = "yaoxiang clean"

# 自定义任务
lint = "yaoxiang fmt && yaoxiang check"
release = "yaoxiang build --release"
deploy = "yaoxiang build && cp dist/* /srv"
docs = "yaoxiang doc"

# 任务依赖
[dependencies.benchmark]
run = "yaoxiang bench"
depends = ["build"]

# 条件任务
[dependencies.docs]
run = "yaoxiang doc"
only = ["features:docs"]
```

**运行任务**：
```bash
yaoxiang task build    # 运行 build 任务
yaoxiang task lint     # 运行 lint 任务
yaoxiang task release  # 运行 release 任务
```

### 2.6 REPL 集成配置

YaoXiang CLI 是统一的（编译器 + 包管理器 + REPL），支持在 yaoxiang.toml 中配置 REPL 行为：

```toml
[repl]
history-size = 1000       # 历史记录条数（默认 1000）
history-file = "~/.yaoxiang_history"  # 历史文件路径
auto-imports = ["std"]   # REPL 启动时自动导入的模块
welcome-message = "Welcome to YaoXiang v0.1.0!"  # 欢迎语
prompt = "yx> "          # 提示符
multi-line = true        # 支持多行输入
colors = true            # 语法高亮
editor = "vim"           # 外部编辑器

[run]
main = "src/main.yx"     # 项目默认入口文件
args = ["--quiet"]       # 默认运行参数

[build]
# 构建时自动执行
pre-build = "yaoxiang fmt"   # 构建前格式化
post-build = "echo Build done!"  # 构建后提示

# 构建产物
output = "dist/"         # 输出目录
```

### 2.7 入口与导出

```toml
[package]
name = "my-library"
version = "0.1.0"

# 库入口
[lib]
path = "src/lib.yx"            # 库主文件

# 可执行程序
[[bin]]
name = "my-cli"
path = "src/cli.yx"

[[bin]]
name = "my-tool"
path = "src/tool.yx"

# 条件导出
[exports]
"." = "src/lib.yx"
"./foo" = "src/foo.yx"
"./bar" = { path = "src/bar.yx", yaoxiang = ">=0.2.0" }
```

### 2.8 平台约束

```toml
[platform]
os = ["windows", "linux", "macos"]
cpu = ["x86_64", "aarch64"]
yaoxiang = ">=0.1.0"

[target.x86_64-unknown-linux-gnu]
# 平台特定配置
optimize = true

[target.x86_64-pc-windows-msvc]
# Windows 特定配置
```

### 2.9 发布配置

```toml
[publish]
registry = "https://packages.yaoxiang.dev"
access = "public"              # public | private
```

## 3. 完整示例

### 3.1 应用程序

```toml
# yaoxiang.toml
[package]
name = "my-cli-tool"
version = "0.1.0"
description = "A command-line tool for processing files"
authors = ["Alice <alice@example.com>"]
license = "MIT"
repository = "https://github.com/alice/my-cli-tool"
homepage = "https://github.com/alice/my-cli-tool"
readme = "README.md"
keywords = ["cli", "tool", "file-processing"]

[yaoxiang]
# edition 使用 SemVer
version = ">=0.1.0, <1.0.0"

[dependencies]
# 标准库
std = "0.1.0"

# 第三方依赖
toml = { git = "https://github.com/yaoxiang/toml", version = "^1.0.0" }

[dev-dependencies]
test-utils = { path = "./crates/test-utils" }

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[build]
script = "scripts/build.yx"

[profile.release]
optimize = true

# 内建任务配置
[tasks]
build = "yaoxiang build"
test = "yaoxiang test"
lint = "yaoxiang fmt && yaoxiang check"
release = "yaoxiang build --release"

# REPL 配置
[repl]
history-size = 1000
auto-imports = ["std"]
welcome-message = "Welcome to My CLI Tool!"

[run]
main = "src/cli.yx"
```

### 3.2 库

```toml
# yaoxiang.toml
[package]
name = "my-parser-lib"
version = "0.1.0"
description = "A JSON parser library for YaoXiang"
authors = ["Alice <alice@example.com>"]
license = "MIT"
repository = "https://github.com/alice/my-parser-lib"

[yaoxiang]
# edition 使用 SemVer
version = ">=0.1.0, <1.0.0"

[dependencies]
std = "0.1.0"

[dev-dependencies]
json-fuzz = { path = "./crates/json-fuzz" }

[lib]
path = "src/lib.yx"

[exports]
"." = "src/lib.yx"
"./error" = "src/error.yx"

# REPL 配置
[repl]
auto-imports = ["std", "./error"]
```

### 3.3 工作空间

```toml
# yaoxiang.toml (workspace root)
[workspace]
members = [
    "crates/cli",
    "crates/lib",
    "crates/utils",
]
exclude = ["temp/", "experiments/"]

[workspace.dependencies]
version = "1.0"
shared-utils = { path = "crates/utils" }
```

## 4. 字段优先级

| 阶段 | 字段 | 优先级 |
|------|------|--------|
| **P0** | `package.name`, `package.version` | 必需 |
| **P0** | `dependencies` | 必需 |
| **P1** | `description`, `authors`, `license` | 推荐 |
| **P1** | `yaoxiang.edition` | 推荐 |
| **P1** | `bin`, `lib` | 常用 |
| **P2** | `dev-dependencies`, `build-dependencies` | 可选 |
| **P2** | `profile.*` | 可选 |
| **P3** | `features`, `platform`, `target.*` | 高级 |
| **P3** | `workspace` | Phase 4 |

## 5. 开放问题

- [ ] 是否需要 `bundled-dependencies`？
- [ ] 是否需要 `peer-dependencies`？
- [ ] `features` 条件编译的语法设计？
- [x] 分层配置（用户级 + 项目级）已确定
- [x] i18n 配置已确定

## 附录 A：字段索引

| 字段 | 位置 | 类型 | 必需 | 说明 |
|------|------|------|------|------|
| `name` | `[package]` | String | ✅ | 包名 |
| `version` | `[package]` | Version | ✅ | 版本号 |
| `description` | `[package]` | String | ❌ | 描述 |
| `authors` | `[package]` | [String] | ❌ | 作者列表 |
| `license` | `[package]` | String | ❌ | 许可证 |
| `repository` | `[package]` | Url | ❌ | 代码仓库 |
| `homepage` | `[package]` | Url | ❌ | 主页 |
| `documentation` | `[package]` | Url | ❌ | 文档地址 |
| `keywords` | `[package]` | [String] | ❌ | 关键词 |
| `readme` | `[package]` | Path | ❌ | README 文件 |
| `edition` | `[yaoxiang]` | String | ❌ | 语言版本 |
| `dependencies` | Top-level | Table | ✅ | 依赖声明 |
| `dev-dependencies` | Top-level | Table | ❌ | 开发依赖 |
| `build-dependencies` | Top-level | Table | ❌ | 构建依赖 |
| `lib` | `[package]` | Table | ❌ | 库配置 |
| `bin` | `[package]` | [Table] | ❌ | 可执行程序 |
| `build` | Top-level | Table | ❌ | 构建配置 |
| `profile.*` | Top-level | Table | ❌ | 构建优化 |
| `fmt` | Top-level | Table | ❌ | 格式化配置 |
| `lint` | Top-level | Table | ❌ |  lint 配置 |
| `test` | Top-level | Table | ❌ | 测试配置 |
| `exports` | `[package]` | Table | ❌ | 导出映射 |
| `platform` | Top-level | Table | ❌ | 平台约束 |
| `target.*` | Top-level | Table | ❌ | 平台特定配置 |
| `publish` | Top-level | Table | ❌ | 发布配置 |
| `workspace` | Top-level | Table | ❌ | 工作空间 |
| `i18n` | Top-level | Table | ❌ | 【创新】国际化配置 |
| `tasks` | Top-level | Table | ❌ | 【创新】内建任务配置 |
| `repl` | Top-level | Table | ❌ | 【创新】REPL 配置 |
| `install` | Top-level | Table | ❌ | 全局安装配置（仅用户级） |

## 参考文献

- [Cargo Manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [npm package.json](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)
- [PEP 621: pyproject.toml](https://peps.python.org/pep-0621/)
- [go.mod reference](https://go.dev/ref/mod#go-mod)
- [deno.json](https://deno.com/manual@v1.28.3/getting-started/configuration_file)

---

## 附录：字段层级速查表

### 项目级专属（不能在用户级配置）

| 字段 | 说明 |
|------|------|
| `[package].*` | 包名、版本、作者等元信息 |
| `[yaoxiang]` | 语言版本约束 |
| `[dependencies]` | 依赖声明 |
| `[dev-dependencies]` | 开发依赖 |
| `[bin]` | 可执行程序配置 |
| `[lib]` | 库配置 |
| `[build]` | 构建脚本 |

### 用户级专属（不能在项目级配置）

| 字段 | 说明 |
|------|------|
| `[install]` | 全局安装目录等 |
| `[install].dir` | 全局安装路径 |

### 两者皆可（项目覆盖用户）

| 字段 | 说明 |
|------|------|
| `[i18n]` | 国际化配置 |
| `[repl]` | REPL 配置 |
| `[fmt]` | 格式化配置 |
| `[lint]` | Lint 配置 |
| `[test]` | 测试配置 |
| `[build].output` | 构建输出目录 |
| `[tasks]` | 自定义任务 |
| `[profile.*]` | 构建优化配置 |
