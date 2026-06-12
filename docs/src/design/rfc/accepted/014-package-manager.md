---
title: "RFC-014: 包管理系统设计"
status: "已接受"
author: "晨煦"
created: "2026-02-12"
updated: "2026-06-11"
group: "rfc-014"  # 本 RFC 是包管理系统的总纲，子 RFC：014a/014b/014c
---

# RFC-014: 包管理系统设计（总纲）

> **子 RFC：**
> - [RFC-014a: Registry 协议规范](../draft/014a-registry-protocol.md)
> - [RFC-014b: 构建系统与二进制分发](../draft/014b-build-system.md)
> - [RFC-014c: 工作空间支持](../draft/014c-workspace.md)

## 摘要

设计 YaoXiang 语言的包管理系统，支持语义化版本控制、本地与 GitHub 依赖、统一导入语法、`yaoxiang.toml` 配置文件和 `yaoxiang.lock` 锁定文件。

## 动机

### 为什么需要这个特性/变更？

包管理是现代编程语言生态的基础设施。当前 YaoXiang 语言缺少：
- 依赖声明机制
- 版本管理能力
- 标准分发渠道

### 当前的问题

```
my-project/
├── src/
│   └── main.yx          # 代码依赖其他模块
├── lib/                  # 手动复制的模块
│   ├── foo.yx
│   └── bar.yx
└── ???                   # 没有标准依赖管理
```

## 提案

### 核心设计

**分层架构**：
```
┌─────────────────────────────────────────────┐
│           Resolution Engine                  │ ← 依赖解析
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← 可扩展源
├──────────┬──────────┬──────────┬────────────┤
│  Local   │   Git    │ Registry │   GitHub   │
│  (本地)  │  (VCS)   │  (开放)  │ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**扩展机制**：新增 Source 类型只需实现 trait，无需修改解析引擎。

### 示例

```bash
# 1. 创建项目
yaoxiang init my-project

# 2. 编辑 yaoxiang.toml 添加依赖
[dependencies]
foo = "^1.0.0"
bar = { git = "https://github.com/user/bar", version = "0.5.0" }

# 3. 安装依赖
yaoxiang add foo

# 4. 代码中使用
use foo;
use bar.baz;
```

### 项目结构

```
my-project/
├── yaoxiang.toml        # 包配置
├── yaoxiang.lock        # 锁定文件（自动生成）
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/              # 本地依赖
        ├── foo-1.2.3/
        └── bar-0.5.0/
```

## 详细设计

### 配置文件格式

**yaoxiang.toml**：
```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"
license = "MIT"
authors = ["Your Name <you@example.com>"]
repository = "https://github.com/you/my-package"
keywords = ["cli", "utility"]

[dependencies]
foo = "1.2.3"           # 精确版本
bar = "^1.0.0"          # 兼容版本
baz = "~1.2.0"          # 补丁版本
qux = { git = "...", version = "0.5.0" }
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"

[build]
strategy = "none"       # none | cargo | cmake | custom

[binaries]
"linux-x86_64" = { url = "...", sha256 = "..." }

[workspace.members]     # 仅工作空间根
core = "packages/core/yaoxiang.toml"
```

**yaoxiang.lock**：
```toml
version = 1

[[package]]
name = "foo"
version = "1.2.3"
source = "git"
resolved = "https://github.com/user/foo?tag=v1.2.3"
integrity = "sha256-xxxx"
```

### 模块解析顺序

```
use foo.bar.baz;

查找顺序:
1. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/)
2. ./src/foo/bar/baz.yx                     (本地模块)
3. ~/.yaoxiang/cache/foo/<ver>/src/foo/bar/baz.yx  (全局缓存)
4. $YXPATH/foo/bar/baz.yx                   (全局路径，预留)
5. $YXLIB/std/foo/bar/baz.yx                (标准库)
```

### 核心数据结构

```rust
// 依赖来源（可扩展）
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> },
    GitHub { owner: String, repo: String, ref_: GitRef },  // GitHub 原生
}

enum GitRef {
    Tag(String),
    Branch(String),
    Rev(String),
    DefaultBranch,
}

// 依赖声明
enum DependencySpec {
    Version(VersionConstraint),
    Git { url: Url, version: Option<VersionConstraint> },
    Local { path: PathBuf },
    Workspace { member: String },  // 工作空间成员引用
}

// 解析后的依赖
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
    checksum: Option<String>,  // SHA-256
}

// 构建策略
enum BuildStrategy {
    None,          // 纯 .yx 包
    Cargo,         // 调用 cargo build
    Cmake,         // 调用 cmake
    Custom,        // 执行 build.yx 脚本
    Precompiled,   // 直接用预编译产物
}
```

### CLI 命令设计

采用统一方案，将编译器、包管理器、REPL 整合为单一 CLI 工具：

#### 单文件模式 vs 项目模式

| 命令 | 单文件 | 项目模式 | 说明 |
|------|--------|---------|------|
| `yaoxiang run <file>` | ✅ | ✅ | 运行文件/项目入口 |
| `yaoxiang build` | ❌ | ✅ | 构建项目 |
| `yaoxiang build <file>` | ✅ | ✅ | 构建单个文件 |
| `yaoxiang init <name>` | ❌ | ✅ | 创建项目 |
| `yaoxiang add <dep>` | ❌ | ✅ | 添加依赖 |
| `yaoxiang update` | ❌ | ✅ | 更新依赖 |
| `yaoxiang fmt` | ✅ | ✅ | 格式化 |
| `yaoxiang check` | ✅ | ✅ | 类型检查 |
| `yaoxiang` (无参数) | ✅ | ✅ | 直接进入 REPL |

#### 命令详解

| 命令 | 功能 | 示例 |
|------|------|------|
| `yaoxiang` | 直接进入 REPL | `yaoxiang` |
| `yaoxiang run <file>` | 运行单文件/项目 | `yaoxiang run main.yx` |
| `yaoxiang init <name>` | 创建新项目 | `yaoxiang init my-app` |
| `yaoxiang build` | 构建项目 | `yaoxiang build` |
| `yaoxiang build <file>` | 构建单个文件 | `yaoxiang build foo.yx` |
| `yaoxiang add <dep>` | 添加依赖 | `yaoxiang add foo` |
| `yaoxiang add -D <dep>` | 添加开发依赖 | `yaoxiang add -D test` |
| `yaoxiang rm <dep>` | 移除依赖 | `yaoxiang rm foo` |
| `yaoxiang update` | 更新所有依赖 | `yaoxiang update` |
| `yaoxiang update foo` | 更新指定依赖 | `yaoxiang update foo` |
| `yaoxiang install` | 安装所有依赖 | `yaoxiang install` |
| `yaoxiang list` | 列出依赖 | `yaoxiang list` |
| `yaoxiang outdated` | 检查过时依赖 | `yaoxiang outdated` |
| `yaoxiang fmt` | 格式化代码 | `yaoxiang fmt` |
| `yaoxiang check` | 类型检查 | `yaoxiang check` |
| `yaoxiang clean` | 清理构建产物 | `yaoxiang clean` |
| `yaoxiang task <name>` | 运行自定义任务 | `yaoxiang task lint` |
| `yaoxiang publish` | 发布包到 Registry | `yaoxiang publish` |
| `yaoxiang publish --github` | 发布并创建 GitHub Release | `yaoxiang publish --github` |
| `yaoxiang yank <pkg>@<ver>` | 删除已发布版本（不可恢复） | `yaoxiang yank foo@1.2.3` |
| `yaoxiang login --registry <url>` | Registry 认证 | `yaoxiang login --registry https://reg.example.com` |
| `yaoxiang login --github` | GitHub 认证 | `yaoxiang login --github` |
| `yaoxiang logout --registry <url>` | 登出 | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean` | 清理全局缓存 | `yaoxiang cache clean` |
| `yaoxiang workspace <cmd>` | 工作空间操作 | `yaoxiang workspace list` |

#### 命令约束说明

```bash
# 单文件模式：不需要 yaoxiang.toml
yaoxiang run hello.yx   # ✅ 正常工作
yaoxiang add foo        # ❌ 报错：不是项目目录

# 项目模式：需要 yaoxiang.toml
cd my-project
yaoxiang run main.yx    # ✅ 运行入口文件
yaoxiang build          # ✅ 构建项目
yaoxiang add foo        # ✅ 添加依赖
```

### 向后兼容性

- ✅ 现有 `use` 语法完全保留
- ✅ 现有模块解析逻辑不变
- ✅ 新增 .yaoxiang/vendor 目录不影响现有项目

### 全局缓存

所有下载的依赖缓存到 `~/.yaoxiang/cache/`，项目 vendor 目录从缓存复制。

```
~/.yaoxiang/
├── cache/
│   ├── registry/
│   │   └── foo-1.2.3/
│   ├── git/
│   │   └── github.com-user-bar-abc123/
│   └── binaries/
│       └── foo-1.2.3-linux-x86_64.tar.gz
├── credentials.toml
└── config.toml
```

```toml
# ~/.yaoxiang/config.toml
[cache]
dir = "~/.yaoxiang/cache"
max_size = "2GB"
ttl = "30d"
```

缓存失效规则：
- Registry 包：版本号不可变，永不失效
- Git 依赖：按 tag/rev 缓存，tag 不变则不失效
- `yaoxiang cache clean` 手动清理

### 认证

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

- 环境变量优先：`$YX_GITHUB_TOKEN`、`$YX_REGISTRY_TOKEN`
- Token 永远不写入 `yaoxiang.toml` 或 `yaoxiang.lock`
- 文件权限 600

### yank 语义

`yaoxiang yank foo@1.2.3` 执行**删除 + 版本号锁死**：

- 包被彻底删除，不可恢复
- 版本号永久占用，不能重新发布同版本号
- 已有 lockfile 引用该版本的项目会报错，需要升级
- **安全目的**：防止 npm 式供应链攻击（攻击者抢注被删除的版本号注入恶意代码）

### Registry 协议

详见 [RFC-014a: Registry 协议规范](../draft/014a-registry-protocol.md)。

核心设计：开放协议 + 适配层。官方 Registry 为主，GitHub Release/main 分支为辅，支持自定义 Registry。

### 构建系统

详见 [RFC-014b: 构建系统与二进制分发](../draft/014b-build-system.md)。

核心设计：声明式 `[build]` 配置，预编译优先/源码兜底，支持 cargo/cmake/custom 策略。

### 工作空间

详见 [RFC-014c: 工作空间支持](../draft/014c-workspace.md)。

核心设计：字典形式 members 声明，共享 lockfile，路径依赖，Cargo workspace 集成。

## 权衡

### 优点

- 统一导入语法，用户无需关心依赖来源
- 确定性构建，lock 文件保证构建一致性
- 离线支持，下载到本地后可离线开发
- Source trait 便于后续扩展

### 缺点

- 需要额外存储空间（.yaoxiang/vendor 目录）
- 版本冲突需要用户手动解决

## 替代方案

| 方案 | 为什么没选 |
|------|-----------|
| 实时 GitHub 访问 | 安全性和缓存复用难以保证 |
| 全局缓存 ($HOME/.yaoxiang) | 隔离性差，版本冲突复杂 |
| 仅支持注册表 | GitHub 是当前主流代码托管平台 |

## 实现策略

### 阶段划分

| 阶段 | 内容 | 状态 |
|------|------|------|
| **Phase 1** | toml 解析、本地依赖、lock 生成、基础算法 | ✅ 已完成 |
| **Phase 2** | GitHub 支持、.yaoxiang/vendor 管理、下载工具 | ✅ 已完成 |
| **Phase 3** | 全局缓存、semver crate 替换、CLI 完善 | 待开始 |
| **Phase 3.5** | Source trait 改 async、async-trait 集成 | 待开始 |
| **Phase 4** | Registry 协议、publish、auth（RFC-014a） | 待开始 |
| **Phase 5** | 构建系统、预编译二进制（RFC-014b） | 待开始 |
| **Phase 6** | 工作空间支持（RFC-014c） | 待开始 |

### 依赖关系

- 无前置依赖
- 需与 `ModuleGraph`（`middle/passes/module/`）集成

### 风险

| 风险 | 缓解措施 |
|------|----------|
| 依赖解析算法复杂 | 先实现简单版本，后加冲突检测 |
| Git 下载不稳定 | 重试和缓存机制 |
| 性能问题 | 惰性加载、增量解析 |

## 开放问题

- [x] `dev-dependencies` 条件编译语法？→ 由 RFC-014b 构建系统统一处理
- [x] 完整性校验算法（SHA-256 / BLAKE3）？→ SHA-256
- [ ] `excludes` 排除特定文件不下载？
- [ ] 包命名规范（是否支持 namespace，如 `@org/pkg`）？
- [ ] Registry API 版本化策略？

---

## 依赖项（Cargo.toml 需新增）

| 用途 | crate | 说明 |
|------|-------|------|
| 语义化版本 | `semver` | 替换手写解析器 |
| HTTP 客户端 | `reqwest` | Registry 通信 |
| SHA-256 | `sha2` | 完整性校验 |
| 压缩 | `flate2` + `tar` | 包格式处理 |

---

## 参考文献

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
