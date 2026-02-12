# RFC-014: 包管理系统设计

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2026-02-12
> **最后更新**: 2026-02-12

## 摘要

设计 YaoXiang 语言的包管理系统，支持语义化版本 (SemVer)、本地模块与 GitHub 依赖（下载到 vendor/ 目录）、统一 `use <name>` 导入语法，以及 `yaoxiang.toml` 配置文件和 `yaoxiang.lock` 锁定文件。

## 动机

### 为什么需要这个特性/变更？

现代编程语言生态中，包管理是基础设施的核心组成部分。目前 YaoXiang 语言存在以下问题：

1. **依赖管理缺失**：用户无法声明项目依赖，只能手动复制代码
2. **版本混乱**：无法管理依赖的版本，容易出现兼容性问题
3. **生态建设困难**：第三方库没有标准的发布和分发机制

### 当前的问题

```
my-project/
├── src/
│   └── main.yx          # 代码中需要其他模块的函数
├── lib/                 # 手动复制来的模块
│   ├── foo.yx
│   └── bar.yx
└── ???                 # 没有标准的依赖声明方式
```

用户期望的工作流：
```bash
# 声明依赖
[dependencies]
foo = "1.2.3"

# 一键安装
yaoxiang add foo

# 直接使用
use foo;    # 无需关心来源
```

## 提案

### 核心设计

采用分层架构，将「依赖来源」抽象为 `Source` trait：

```
┌─────────────────────────────────────────┐
│           Resolution Engine             │
│    (依赖解析、版本约束匹配、冲突检测)      │
└─────────────┬───────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│              Source Trait               │
├─────────────┬─────────────┬───────────────┤
│   Local     │    Git      │   Registry   │
│   Source    │   Source    │    (预留)     │
│  (本地路径)  │  (GitHub)   │ (crates.io)   │
└─────────────┴─────────────┴───────────────┘
```

### 项目结构

```
my-project/
├── yaoxiang.toml        # 包配置
├── yaoxiang.lock        # 锁定文件（自动生成）
├── src/                 # 源码
│   └── main.yx
└── vendor/              # 下载的依赖
    ├── foo-1.2.3/
    │   ├── yaoxiang.toml
    │   └── src/
    └── bar-0.5.0/
```

### 用户工作流

```bash
# 1. 创建项目
yaoxiang init my-project

# 2. 编辑 yaoxiang.toml 添加依赖
[dependencies]
foo = { git = "https://github.com/user/foo", version = "1.2.3" }

# 3. 下载依赖
yaoxiang add foo   # 或 yaoxiang install

# 4. 代码中使用（统一为本地路径）
use foo;
use foo.bar;
```

### yaoxiang.toml 配置规范

```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"
authors = ["Author Name <email@example.com>"]

[dependencies]
# 语义化版本约束
foo = "1.2.3"           # 精确版本
bar = "^1.0.0"          # 兼容版本 (>=1.0.0, <2.0.0)
baz = "~1.2.0"          # 补丁版本 (>=1.2.0, <1.3.0)

# GitHub 依赖
qux = { git = "https://github.com/user/qux", version = "0.5.0" }

# 本地路径依赖
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"

[build]
script = "build.yx"
```

### 版本约束语法

| 约束 | 含义 | 示例 |
|------|------|------|
| `1.2.3` | 精确版本 | `1.2.3` |
| `^1.2.3` | 兼容版本 | `>=1.2.3, <2.0.0` |
| `~1.2.3` | 补丁版本 | `>=1.2.3, <1.3.0` |
| `>=1.0` | 最低版本 | `>=1.0.0` |
| `1.x` | 主版本通配 | `>=1.0.0, <2.0.0` |
| `*` | 任意版本 | 任意 |

### yaoxiang.lock 格式

```toml
# 此文件自动生成，请勿手动修改
version = 1

[[package]]
name = "foo"
version = "1.2.3"
source = "git"
resolved = "https://github.com/user/foo?tag=v1.2.3"
integrity = "sha256-xxxx"

[[package]]
name = "bar"
version = "0.5.0"
source = "local"
resolved = "./vendor/bar-0.5.0"

[[package]]
name = "my-package"
version = "0.1.0"
dependencies = [
    "foo 1.2.3",
    "bar 0.5.0",
]
```

### 模块解析顺序

```
use foo.bar.baz;

查找顺序:
1. ./vendor/*/src/foo/bar/baz.yx  (vendor/ 中匹配版本)
2. ./src/foo/bar/baz.yx           (本地模块)
3. $YXPATH/foo/bar/baz.yx         (全局路径，预留)
4. $YXLIB/std/foo/bar/baz.yx      (标准库)
```

## 详细设计

### 核心数据结构

```rust
// src/package/manifest.rs

/// 包清单（从 yaoxiang.toml 解析）
#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub package: PackageConfig,
    pub dependencies: HashMap<String, DependencySpec>,
    pub dev_dependencies: HashMap<String, DependencySpec>,
    pub build: Option<BuildConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageConfig {
    pub name: String,
    pub version: Version,
    pub description: Option<String>,
    pub authors: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum DependencySpec {
    /// 简单版本约束: "1.2.3"
    Version(VersionConstraint),
    /// Git 依赖
    Git {
        url: Url,
        version: Option<VersionConstraint>,
        tag: Option<String>,
        branch: Option<String>,
        rev: Option<String>,
    },
    /// 本地路径
    Local { path: PathBuf },
}
```

```rust
// src/package/version.rs

/// 语义化版本
#[derive(Debug, Clone, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
    pub build: Option<String>,
}

/// 版本约束表达式
#[derive(Debug, Clone)]
pub enum VersionConstraint {
    Exact(Version),
    Compatible(Version),  // ^1.2.3
    Patch(Version),       // ~1.2.3
    Range { min: Option<Version>, max: Option<Version> },
    Wildcard(MatchLevel),
}

enum MatchLevel { Major, Minor }
```

```rust
// src/package/resolution.rs

/// 解析后的依赖项
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: Version,
    pub source: SourceInfo,
    pub integrity: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SourceInfo {
    Local { path: PathBuf },
    Git { url: Url, tag: Option<String>, commit: Option<String> },
    Registry { registry: String, namespace: Option<String> },
}

/// 包标识符：名称 + 版本（确保同一包的不同版本可共存）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    pub name: String,
    pub version: Version,
}
```

### 组件架构

```
src/package/
├── mod.rs              # 入口，导出公共 API
├── manifest.rs         # yaoxiang.toml 解析
├── version.rs          # SemVer 解析与约束
├── resolution.rs       # 依赖解析算法
├── lock.rs             # yaoxiang.lock 读写
├── fetch.rs            # 下载依赖到 vendor/
├── storage.rs          # 本地缓存管理
└── id.rs               # PackageId 定义
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

- **现有 `use` 语法完全保留**：`use std.io` 继续正常工作
- **现有模块解析不受影响**：`src/` 目录下的模块查找逻辑不变
- **新增 vendor/ 目录不影响现有项目**

## 权衡

### 优点

- **统一导入语法**：用户无需关心依赖来源，统一 `use <name>`
- **确定性构建**：`yaoxiang.lock` 保证不同机器构建结果一致
- **离线支持**：下载到本地后可离线开发
- **渐进式扩展**：源抽象为 trait，便于后续添加注册表支持
- **统一 CLI 体验**：编译器、包管理器、REPL 合为一体
- **可配置 REPL**：在 yaoxiang.toml 中配置 REPL 行为，开箱即用

### 缺点

- **需要下载依赖**：相比实时 GitHub，需要额外存储空间
- **版本冲突处理**：需要用户手动解决不兼容的版本依赖

## 替代方案

| 方案 | 描述 | 为什么没选 |
|------|------|-----------|
| 实时 GitHub 访问 | `use github.com/user/repo` 直接拉取 | 安全性和缓存复用难以保证 |
| 全局缓存 | 依赖存储在全局目录 ($HOME/.yaoxiang) | 隔离性差，版本冲突更复杂 |
| 仅语义化版本 | 不支持 Git 源，只支持注册表 | GitHub 是当前主流的代码托管平台 |

## 实现策略

### 阶段划分

**Phase 1: 基础依赖管理**
- yaoxiang.toml 解析
- 版本约束解析与匹配
- 依赖图构建与冲突检测
- yaoxiang.lock 生成
- 本地路径依赖支持

**Phase 2: GitHub 支持**
- Git 源实现
- 下载工具
- vendor/ 目录管理
- Git 标签/分支解析

**Phase 3: 注册表支持（预留）**
- Registry Source 实现
- 搜索功能
- 私有注册表支持

**Phase 4: 高级功能**
- 工作空间 (Workspace)
- 依赖覆盖
- 依赖完整性校验

### 依赖关系

- 此 RFC 无前置依赖
- 实现时需与 `ModuleGraph`（`middle/passes/module/`）集成

### 风险

| 风险 | 缓解措施 |
|------|----------|
| 依赖解析算法复杂 | 先实现简单版本，后加冲突检测 |
| Git 下载不稳定 | 实现重试和缓存机制 |
| 性能问题 | 惰性加载、增量解析 |

## 开放问题

- [ ] 是否需要支持 `dev-dependencies` 的条件编译？
- [ ] 依赖的 `Integrity` 校验使用什么算法？（SHA-256 / BLAKE3）
- [ ] 是否需要支持 `excludes` 排除特定文件不下载？

## 附录

### 附录 A：术语表

| 术语 | 定义 |
|------|------|
| Manifest | 包清单文件 (yaoxiang.toml) |
| Lockfile | 锁定文件 (yaoxiang.lock)，记录精确版本 |
| Vendor | 本地依赖存储目录 |
| Source | 依赖来源（本地/Git/注册表） |
| Resolution | 依赖解析过程 |

### 附录 B：参考实现

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [npm Package Manifest](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)
- [Go Modules](https://go.dev/ref/mod)

---

## 参考文献

- [RFC-011: 泛型系统设计](011-generic-type-system.md) - 类型系统基础
- [Yarn Berry: Plug'n'Play](https://yarnpkg.com/features/pnp) - 替代依赖管理方案
- [Python PEP 440: Version Identification](https://peps.python.org/pep-0440/) - 版本标识标准
