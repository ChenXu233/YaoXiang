---
title: "RFC-014c: 工作空间支持"
status: "审核中"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
group: "rfc-014"
issue: "#113"
---

# RFC-014c: 工作空间支持

> 本 RFC 是 [RFC-014: 包管理系统设计](../accepted/014-package-manager.md) 的子 RFC。

## 摘要

定义 YaoXiang 的工作空间（workspace）机制：多个相关包一起开发时的依赖共享、路径引用、lockfile 统一、与 Cargo workspace 的集成。

## 动机

当项目规模增长，代码需要拆成多个包。这些包需要：
- 互相引用（路径依赖）
- 共享外部依赖版本（避免版本漂移）
- 统一 lockfile（保证构建一致性）
- 与 Cargo workspace 协作（FFI 部分）

### 当前的问题

- 每个项目独立管理依赖，无法共享
- 没有路径依赖的发布时自动替换机制
- 没有与 Cargo workspace 的集成

## 提案

### 核心设计：协调层 + 自包含成员

根 workspace 只做协调，每个成员完全自包含。

### 根 yaoxiang.toml

```toml
# 根 yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**根 toml 只做三件事：**
1. 声明成员列表（字典形式，key 为成员名，value 为 toml 路径）
2. 提供共享 lockfile（`yaoxiang.lock`）
3. 提供共享 vendor 目录（`.yaoxiang/vendor/`）

**根 toml 不定义 dependencies。** 每个成员的依赖写在自己的 `yaoxiang.toml` 里。

### 成员 yaoxiang.toml

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # 引用工作空间成员
regex = "^1.0.0"
```

```toml
# packages/utils/yaoxiang.toml
[package]
name = "utils"
version = "0.2.0"

[dependencies]
regex = "^1.0.0"
```

### 工作空间结构

```
my-workspace/
├── yaoxiang.toml              # 工作空间根配置
├── yaoxiang.lock              # 共享 lockfile
├── .yaoxiang/
│   └── vendor/                # 共享 vendor 目录
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # 成员包配置
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # 可选：共享 Cargo workspace（FFI）
```

### 依赖解析

- 每个成员读自己的 `[dependencies]`
- 解析时合并所有成员的依赖，生成一个共享 lockfile
- 版本冲突在 lockfile 生成时报错
- 同一个包在不同成员中必须解析到相同版本

### workspace 依赖引用

`{ workspace = "member-name" }` 引用 `[workspace.members]` 的 **key**（不是成员的 `[package].name`）。

```toml
# 根 yaoxiang.toml
[workspace.members]
utils = "packages/utils/yaoxiang.toml"    # key = "utils"
```

```toml
# packages/app/yaoxiang.toml
[package]
name = "app"

[dependencies]
utils = { workspace = "utils" }   # ✅ 引用 key "utils"
# 即使 packages/utils/yaoxiang.toml 里写的是 name = "my-utils"
```

**为什么用 key 而不是 name：**
- key 由工作空间控制，稳定唯一
- `[package].name` 是公开名，发布时可能变
- key 是 BTreeMap 的 key，天然唯一
- 发布时 workspace 引用被替换为版本依赖，key 不泄露到公开 API

### 路径依赖与发布

开发时用工作空间引用：

```toml
[dependencies]
utils = { workspace = "utils" }
```

发布时自动替换为版本依赖：

```toml
[dependencies]
utils = "^0.2.0"
```

**版本来源：** 读被依赖成员的 `[package].version`，加 `^` 前缀。不检查 Registry——版本的权威来源是成员的 `yaoxiang.toml`，Registry 只是分发渠道。

包管理器在 `yaoxiang publish` 时自动完成这个替换。

### 与 Cargo Workspace 集成

如果工作空间中有 FFI 包，可以同时定义 Cargo workspace：

```toml
# 根 Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace（FFI 部分）
├── packages/
│   ├── core/
│   │   ├── src/lib.yx     # YaoXiang 代码
│   │   └── native/
│   │       ├── Cargo.toml # Rust FFI 代码
│   │       └── src/lib.rs
│   └── utils/
│       ├── src/lib.yx
│       └── native/
│           ├── Cargo.toml
│           └── src/lib.rs
```

`yaoxiang build` 自动检测并调用 `cargo build` 编译 native 部分。

### CLI 命令

| 命令 | 功能 |
|------|------|
| `yaoxiang workspace list` | 列出工作空间成员 |
| `yaoxiang workspace add <path>` | 添加成员 |
| `yaoxiang workspace remove <name>` | 移除成员 |
| `yaoxiang build` | 构建所有成员（按依赖拓扑排序） |
| `yaoxiang build core` | 构建指定成员 |
| `yaoxiang test` | 运行所有成员的测试 |

**`yaoxiang build` 行为：** 构建所有成员，按依赖拓扑排序。如果 core → utils → app，构建顺序为 core → utils → app。

## 详细设计

### WorkspaceManifest 结构

根 toml 使用独立的 `WorkspaceManifest` 类型，不复用 `PackageManifest`：

```rust
struct WorkspaceManifest {
    workspace: WorkspaceConfig,
}

struct WorkspaceConfig {
    members: BTreeMap<String, String>,  // key -> toml path
}

struct Workspace {
    root: PathBuf,
    manifest: WorkspaceManifest,
    members: Vec<WorkspaceMember>,
    lock: LockFile,
}

struct WorkspaceMember {
    name: String,           // [workspace.members] 的 key
    root: PathBuf,
    manifest: PackageManifest,
}
```

**探测逻辑：** 加载 toml 时，如果有 `[workspace]` 段则解析为 `WorkspaceManifest`，否则解析为 `PackageManifest`。

### workspace 依赖引用

`{ workspace = "member-name" }` 的语义：
- 在 `dependencies` 中引用另一个工作空间成员
- 开发时解析为本地路径
- 发布时替换为 Registry 版本
- 成员名必须在 `[workspace.members]` 中存在

### lockfile 共享

- 工作空间只有一个 `yaoxiang.lock`（在根目录）
- 所有成员的依赖解析合并到同一个 lockfile
- 版本冲突在 lockfile 生成时报错，附带冲突来源信息

## 权衡

### 优点

- 多包项目统一管理
- 共享 lockfile 保证一致性
- 路径依赖开发体验好
- 与 Cargo workspace 无缝集成

### 缺点

- 所有成员必须使用相同的外部依赖版本（可能过于严格）
- 根 toml 不能有自己的依赖（设计约束）
- Cargo workspace 集成增加了复杂度

## 替代方案

| 方案 | 为什么没选 |
|------|-----------|
| 独立项目 + path 依赖 | lockfile 不统一，版本漂移风险 |
| 类似 npm workspaces | npm 的 workspace 问题多，不值得模仿 |
| Cargo workspace 直接复用 | YaoXiang 和 Cargo 是不同的包生态系统 |

## 实现策略

### 阶段划分

| 阶段 | 内容 |
|------|------|
| Phase 6a | `[workspace.members]` 解析 + WorkspaceManifest |
| Phase 6b | 共享 lockfile + 依赖合并解析 |
| Phase 6c | `{ workspace = "name" }` 路径依赖引用 |
| Phase 6d | 发布时路径依赖自动替换 |
| Phase 6e | Cargo workspace 集成 |

### 依赖关系

- 依赖 RFC-014 Phase 3（全局缓存）
- 可选依赖 RFC-014b（构建系统，用于 native 成员）

## 开放问题

- [ ] 是否允许成员之间的循环依赖？
- [ ] 是否支持 workspace 级别的 `[build]` 配置？
- [ ] 成员是否可以有自己的 lockfile（覆盖根 lockfile）？
- [ ] 是否支持嵌套 workspace？

---

## 参考文献

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
