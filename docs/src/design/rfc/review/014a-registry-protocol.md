---
title: "RFC-014a: Registry 协议规范"
status: "审核中"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
group: "rfc-014"
---

# RFC-014a: Registry 协议规范

> 本 RFC 是 [RFC-014: 包管理系统设计](../accepted/014-package-manager.md) 的子 RFC。

## 摘要

定义 YaoXiang 包管理系统的 Registry 协议：开放接口设计、官方 Registry 规范、GitHub 适配层、包发布/撤回流程、认证模型。

## 动机

RFC-014 总纲定义了包管理系统的整体架构，但 Registry 部分仅标记为"预留"。没有 Registry 协议，包无法分发——这就像设计了一个没有商店的购物车。

### 当前的问题

- `RegistrySource` 是桩代码（`source/mod.rs:150-203`），`resolve` 直接返回声明版本，`download` 返回空路径
- 没有 HTTP 客户端（无 `reqwest` 依赖）
- 没有包发布机制
- 没有认证/授权

## 提案

### 核心设计：开放协议 + 适配层

```
┌──────────────────────────────────────────┐
│         yaoxiang publish/install         │  ← CLI 层
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│          Registry Trait                  │  ← 协议层（开放接口）
│  ┌─────────┬──────────┬────────────┐    │
│  │ .publish│ .search  │ .download  │    │
│  │ .yank   │ .info    │ .versions  │    │
│  └─────────┴──────────┴────────────┘    │
└──────────────────┬───────────────────────┘
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
   ┌─────────┐ ┌────────┐ ┌────────┐
   │ 官方    │ │ GitHub │ │ 自定义 │
   │ Registry│ │ 适配   │ │ Registry│
   └─────────┘ └────────┘ └────────┘
```

### 异步架构决策

`Source` trait 统一改为 async，全面拥抱 tokio：

```rust
// 现有（同步）→ 改为（异步）
#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String>;
    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage>;
}
```

所有实现（`LocalSource`、`GitSource`、`RegistrySource`）统一改为 async。CLI 入口通过 `#[tokio::main]` 或 `Runtime::block_on` 驱动。

**理由：**
- Registry 需要 HTTP 请求，阻塞会卡死整个安装流程
- 多依赖并行下载（`join_all`）显著提升安装速度
- Git clone 也是 I/O 操作，async 更自然
- tokio 已在项目依赖中

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// 发布包
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// 删除已发布版本（不可恢复，版本号锁死）
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// 查询包信息
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// 查询可用版本列表
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// 搜索包
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// 下载指定版本
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// 认证
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### 源优先级（默认查找链）

`yaoxiang add foo`（无 flag）时的默认查找顺序：

| 优先级 | 查找 | 说明 |
|--------|------|------|
| 1 | 全局缓存 | `~/.yaoxiang/cache/registry/foo-<ver>/` |
| 2 | 官方 Registry | 查询版本 → 下载 |
| 3 | 失败 | 报错，提示用户检查包名或网络 |

**显式覆盖（不走默认链）：**

| flag | 行为 |
|------|------|
| `--git <url>` | 跳过 Registry，直接 Git clone（优先 Release assets → fallback 到 tag/branch） |
| `--path <dir>` | 跳过 Registry，直接用本地路径 |
| `--registry <url>` | 跳过官方 Registry，用指定 Registry |

### 官方 Registry

官方 Registry 类似 crates.io，是包分发的主要渠道。

**API 端点：**

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/packages/{name}` | GET | 查询包信息 |
| `/api/v1/packages/{name}/versions` | GET | 查询版本列表 |
| `/api/v1/packages/{name}/{version}` | GET | 下载包 |
| `/api/v1/packages` | PUT | 发布包 |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | 撤回版本 |
| `/api/v1/search?q={query}` | GET | 搜索包 |
| `/api/v1/login` | POST | 认证 |

### GitHub 集成

GitHub 作为包源时，采用 Go modules 风格的策略：

1. **优先 Release assets**：检查 GitHub Release 页面有无匹配平台的预编译产物
2. **Fallback 到 main 分支**：无 Release 则 git clone

```toml
[dependencies]
# 基本 git 依赖
foo = { git = "https://github.com/user/foo" }

# 指定版本（匹配 tag）
bar = { git = "https://github.com/user/bar", version = "^1.0.0" }

# 指定分支
baz = { git = "https://github.com/user/baz", branch = "main" }

# 指定 commit
qux = { git = "https://github.com/user/qux", rev = "abc123" }

# 私有仓库（使用 credentials.toml 中的 GitHub token）
private = { git = "https://github.com/my-org/private-lib" }
```

### 包格式（.yxpkg）

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # 包元数据
├── src/                   # 源代码
├── build/                 # 构建产物（如果有）
│   └── native/
│       └── linux-x86_64/
│           └── libfoo.so
├── build.yx               # 构建脚本（如果有）
└── SHA256SUMS             # 校验和
```

### publish 流程

```bash
# 发布到官方 Registry
yaoxiang publish

# 发布到指定 Registry
yaoxiang publish --registry my-company

# 同时创建 GitHub Release
yaoxiang publish --github

# 干跑
yaoxiang publish --dry-run
```

发布前校验：
1. `yaoxiang.toml` 必须有 `name`、`version`、`description`
2. 版本号不能已存在
3. 运行测试（可选，`--no-test` 跳过）
4. 计算所有文件的 SHA-256
5. 打包为 `.yxpkg`（tar.gz）
6. 上传到 Registry

### yank 语义

```bash
yaoxiang yank foo@1.2.3
```

**删除 + 版本号锁死：**

- 包被彻底删除，不可恢复
- 版本号永久占用，不能重新发布同版本号
- 已有 lockfile 引用该版本的项目会报错，需要升级到其他版本
- **安全目的**：防止 npm 式供应链攻击。攻击者曾抢注被删除的包版本号注入恶意代码，yank 锁死版本号彻底堵死这条路。

### 认证模型

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**映射规则：** `yaoxiang login --registry <url>` 按 URL 匹配 `[registries.*]` 中的 `url` 字段。如果没有匹配，新建一个条目（自动生成名称，如 `reg-1`）。

**优先级：** 环境变量 > 配置文件

| 环境变量 | 用途 |
|----------|------|
| `$YX_GITHUB_TOKEN` | GitHub 认证 |
| `$YX_REGISTRY_TOKEN` | Registry 认证（用于默认 Registry） |
| `$YX_REGISTRY_URL` | 默认 Registry 地址 |

**CLI 命令：**

```bash
yaoxiang login --registry https://yxreg.example.com   # 按 URL 匹配或新建
yaoxiang login --github                                # GitHub OAuth 或 token
yaoxiang logout --registry https://yxreg.example.com   # 删除匹配的条目
```

**安全约束：**
- Token 永远不写入 `yaoxiang.toml` 或 `yaoxiang.lock`
- `credentials.toml` 文件权限 600
- CI 场景用环境变量，开发场景用文件

## 详细设计

### RegistrySource 实现

替换现有桩代码（`source/mod.rs:150-203`）：

```rust
pub struct RegistrySource {
    client: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl Source for RegistrySource {
    fn name(&self) -> &str { "registry" }
    fn kind(&self) -> SourceKind { SourceKind::Registry }

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String> {
        let url = format!("{}/api/v1/packages/{}/versions", self.base_url, spec.name);
        let versions: Vec<Version> = self.client.get(&url).send().await?.json().await?;
        let req = parse_version_req(&spec.version)?;
        select_best(&req, &versions)
            .map(|v| v.to_string())
            .ok_or(PackageError::DependencyNotFound(spec.name.clone()))
    }

    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage> {
        let version = self.resolve(spec).await?;
        let url = format!("{}/api/v1/packages/{}/{}/download", self.base_url, spec.name, version);
        let bytes = self.client.get(&url).send().await?.bytes().await?;

        // SHA-256 校验
        let actual_hash = sha256_hex(&bytes);
        // ... 解压到 dest ...

        Ok(ResolvedPackage {
            name: spec.name.clone(),
            version,
            source_kind: SourceKind::Registry,
            source_url: self.base_url.clone(),
            local_path: dest.to_path_buf(),
            checksum: Some(actual_hash),
        })
    }
}
```

### 依赖项

| crate | 用途 |
|-------|------|
| `reqwest` | HTTP 客户端 |
| `sha2` | SHA-256 校验 |
| `flate2` + `tar` | 包格式处理 |
| `async-trait` | async trait 支持 |

### 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("包 '{0}' 不存在")]
    PackageNotFound(String),

    #[error("版本 '{0}' 不存在")]
    VersionNotFound(String),

    #[error("版本 '{0}' 已被占用")]
    VersionAlreadyExists(String),

    #[error("认证失败: {0}")]
    AuthFailed(String),

    #[error("网络错误: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("SHA-256 校验失败: 期望 {expected}, 实际 {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("权限不足: {0}")]
    Forbidden(String),
}
```

## 权衡

### 优点

- 开放协议，不绑定特定服务器
- GitHub 作为轻量级分发渠道，降低入门门槛
- 版本号锁死的安全模型
- 预编译优先的安装策略

### 缺点

- 官方 Registry 需要独立运维
- GitHub API 有速率限制
- 版本号锁死可能导致版本号浪费

## 替代方案

| 方案 | 为什么没选 |
|------|-----------|
| 仅支持 GitHub | 受限于 GitHub 生态，无法自建 Registry |
| Cargo 风格 crates.io | 过于复杂，YaoXiang 生态初期不需要 |
| npm 风格 yank（仅标记） | 安全风险，已知供应链攻击案例 |

## 实现策略

### 阶段划分

| 阶段 | 内容 |
|------|------|
| Phase 3.5 | Source trait 改 async + async-trait + 所有实现迁移 |
| Phase 4a | Registry trait + reqwest 集成 + 本地 Registry mock |
| Phase 4b | GitHub Release 适配 |
| Phase 4c | publish 命令 + 包格式打包 |
| Phase 4d | 认证 + yank |

### 依赖关系

- 依赖 RFC-014 Phase 3（全局缓存、semver 替换）
- 依赖 RFC-014b（构建系统，用于 `build/` 目录处理）

## 开放问题

- [ ] Registry API 是否需要版本化（`/api/v1/` vs `/api/v2/`）？
- [ ] 包名是否支持 namespace（如 `@org/pkg`）？
- [ ] 速率限制策略？
- [ ] 包大小上限？

---

## 参考文献

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
