---
title: '包管理器错误'
description: '包管理器错误的类型与处理方式'
---

# 包管理器错误

> 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/package/error.rs` 的 `PackageError` 枚举生成，请勿手工编辑。
改文案请改该文件里的 `#[error("…")]` 格式串，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

包管理器（`yx install` / `add` / `rm` / `update` / `publish` / `cache` / `workspace`）的错误
**不带 `E` 码号**：它们是 `thiserror` 定义的 Rust 枚举 `PackageError`，
在 CLI 层以 `Error: <格式串渲染结果>` 的形式打印，不进入编译器的诊断渲染管线。
当前共 **23** 个变体，定义在 `src/package/error.rs`。
编译器诊断码请见[错误码参考](../error-code/index.md)。

## 变体一览

| 变体 | 错误文案（`Display`，英文即实际输出） | 含义 |
| --- | --- | --- |
| `ProjectExists` | `Project already exists: {0}` | 目标目录已存在（`yx init` 不会覆盖） |
| `NotProject` | `Not a YaoXiang project: yaoxiang.toml not found` | 当前目录不是 YaoXiang 项目：找不到 `yaoxiang.toml` |
| `DependencyNotFound` | `Dependency not found: {0}` | 依赖未在 `yaoxiang.toml` 中声明 |
| `DependencyAlreadyExists` | `Dependency already exists: {0}` | 依赖已在 `yaoxiang.toml` 中声明 |
| `DependencyInstallFailed` | `dependency installation failed: {0}` | 一个或多个依赖安装失败 |
| `InvalidManifest` | `Invalid yaoxiang.toml format: {0}` | `yaoxiang.toml` 格式非法或内容不完整 |
| `Cache` | `cache error: {0}` | 全局包缓存读写失败（RFC-014 Phase 3） |
| `NotWorkspace` | `not a YaoXiang workspace: [workspace] section not found` | 当前不在工作区内：任一 `yaoxiang.toml` 都缺 `[workspace]` 段（RFC-014c） |
| `MemberMissing` | `workspace member '{key}' not found: {path}` | 工作区成员的清单文件缺失（RFC-014c） |
| `MemberInvalid` | `workspace member '{key}' invalid: {reason}` | 工作区成员的清单无法解析或不合规（RFC-014c） |
| `NestedWorkspace` | `nested workspace is not supported: member '{key}' at {path} has its own [workspace] section` | 嵌套工作区不被支持：成员自带 `[workspace]` 段（RFC-014c） |
| `PackageTooLarge` | `package too large: {0}` | 源码包超过 20 MiB 上限（RFC-014a） |
| `ChecksumMismatch` | `checksum mismatch: expected {expected}, got {actual}` | 校验和不符：下载内容与锁文件记录不一致（RFC-014a） |
| `InvalidPackage` | `invalid package: {0}` | 包归档非法：缺清单、路径逃逸、条目类型不合法等（RFC-014a） |
| `Network` | `network error: {0}` | 网络请求失败（GitHub 适配器，RFC-014a Phase 4） |
| `RateLimited` | `rate limited: {0}` | 命中 API 速率限制（RFC-014a decision 6） |
| `RegistryDeferred` | `official registry is deferred (RFC-014a); use \`publish --github\` or \`--dry-run\`` | 官方注册表尚未开放：改用 `publish --github` 或 `--dry-run`（RFC-014a decision 1） |
| `VersionAlreadyExists` | `release already exists: {0}` | 该版本的 Release 已存在（RFC-014a 发布校验） |
| `AuthFailed` | `auth failed: {0}` | 认证失败：凭据缺失、无效或被拒（RFC-014a） |
| `PublishTarget` | `publish target unresolved: {0}` | 发布目标仓库无法解析 |
| `TestsFailed` | `tests failed: {0}` | 发布前置测试未通过（RFC-014a 发布校验第 3 步） |
| `Io` | `IO error: {0}` | 底层 I/O 失败：磁盘空间、权限、文件占用等 |
| `Toml` | `TOML parse error: {0}` | TOML 序列化 / 反序列化失败（锁文件与清单均适用） |

## 逐变体说明

### `ProjectExists`

- **含义**：目标目录已存在（`yx init` 不会覆盖）
- **文案**：`Project already exists: {0}`
- **源码 doc**：`Project directory already exists`

### `NotProject`

- **含义**：当前目录不是 YaoXiang 项目：找不到 `yaoxiang.toml`
- **文案**：`Not a YaoXiang project: yaoxiang.toml not found`
- **源码 doc**：`Not inside a YaoXiang project (no yaoxiang.toml found)`

### `DependencyNotFound`

- **含义**：依赖未在 `yaoxiang.toml` 中声明
- **文案**：`Dependency not found: {0}`
- **源码 doc**：`Dependency not found in manifest`

### `DependencyAlreadyExists`

- **含义**：依赖已在 `yaoxiang.toml` 中声明
- **文案**：`Dependency already exists: {0}`
- **源码 doc**：`Dependency already exists in manifest`

### `DependencyInstallFailed`

- **含义**：一个或多个依赖安装失败
- **文案**：`dependency installation failed: {0}`
- **源码 doc**：`One or more dependencies could not be installed`

### `InvalidManifest`

- **含义**：`yaoxiang.toml` 格式非法或内容不完整
- **文案**：`Invalid yaoxiang.toml format: {0}`
- **源码 doc**：`Invalid manifest format`

### `Cache`

- **含义**：全局包缓存读写失败（RFC-014 Phase 3）
- **文案**：`cache error: {0}`
- **源码 doc**：`Global package cache error (RFC-014 Phase 3)`

### `NotWorkspace`

- **含义**：当前不在工作区内：任一 `yaoxiang.toml` 都缺 `[workspace]` 段（RFC-014c）
- **文案**：`not a YaoXiang workspace: [workspace] section not found`
- **源码 doc**：`Not inside a YaoXiang workspace (no [workspace] in any yaoxiang.toml, RFC-014c)`

### `MemberMissing`

- **含义**：工作区成员的清单文件缺失（RFC-014c）
- **文案**：`workspace member '{key}' not found: {path}`
- **源码 doc**：`Workspace member manifest missing (RFC-014c)`

### `MemberInvalid`

- **含义**：工作区成员的清单无法解析或不合规（RFC-014c）
- **文案**：`workspace member '{key}' invalid: {reason}`
- **源码 doc**：`Workspace member manifest invalid (RFC-014c)`

### `NestedWorkspace`

- **含义**：嵌套工作区不被支持：成员自带 `[workspace]` 段（RFC-014c）
- **文案**：`nested workspace is not supported: member '{key}' at {path} has its own [workspace] section`
- **源码 doc**：`Nested workspace (forbidden, RFC-014c 2026-09-15 decision 4)`

### `PackageTooLarge`

- **含义**：源码包超过 20 MiB 上限（RFC-014a）
- **文案**：`package too large: {0}`
- **源码 doc**：`Package content exceeds the source-package size limit (RFC-014a 2026-09-15 decision 7: 20 MiB)`

### `ChecksumMismatch`

- **含义**：校验和不符：下载内容与锁文件记录不一致（RFC-014a）
- **文案**：`checksum mismatch: expected {expected}, got {actual}`
- **源码 doc**：`Checksum mismatch (RFC-014a)`

### `InvalidPackage`

- **含义**：包归档非法：缺清单、路径逃逸、条目类型不合法等（RFC-014a）
- **文案**：`invalid package: {0}`
- **源码 doc**：`Invalid package archive (missing manifest, path escape, bad entry type, ...)`

### `Network`

- **含义**：网络请求失败（GitHub 适配器，RFC-014a Phase 4）
- **文案**：`network error: {0}`
- **源码 doc**：`Network request failed (GitHub adapter, RFC-014a Phase 4)`

### `RateLimited`

- **含义**：命中 API 速率限制（RFC-014a decision 6）
- **文案**：`rate limited: {0}`
- **源码 doc**：`API rate limit exhausted (RFC-014a decision 6)`

### `RegistryDeferred`

- **含义**：官方注册表尚未开放：改用 `publish --github` 或 `--dry-run`（RFC-014a decision 1）
- **文案**：`official registry is deferred (RFC-014a); use `publish --github` or `--dry-run``
- **源码 doc**：`Bare `publish` without a channel (official registry deferred, RFC-014a decision 1)`

### `VersionAlreadyExists`

- **含义**：该版本的 Release 已存在（RFC-014a 发布校验）
- **文案**：`release already exists: {0}`
- **源码 doc**：`Release for the version already exists (RFC-014a publish validation)`

### `AuthFailed`

- **含义**：认证失败：凭据缺失、无效或被拒（RFC-014a）
- **文案**：`auth failed: {0}`
- **源码 doc**：`Authentication failed (RFC-014a)`

### `PublishTarget`

- **含义**：发布目标仓库无法解析
- **文案**：`publish target unresolved: {0}`
- **源码 doc**：`Publish target repository could not be resolved`

### `TestsFailed`

- **含义**：发布前置测试未通过（RFC-014a 发布校验第 3 步）
- **文案**：`tests failed: {0}`
- **源码 doc**：`Pre-publish tests failed (RFC-014a publish validation step 3)`

### `Io`

- **含义**：底层 I/O 失败：磁盘空间、权限、文件占用等
- **文案**：`IO error: {0}`
- **源码 doc**：`IO error`

### `Toml`

- **含义**：TOML 序列化 / 反序列化失败（锁文件与清单均适用）
- **文案**：`TOML parse error: {0}`
- **源码 doc**：`TOML serialization/deserialization error`

## 处理方式

按族归类排查：

| 症状 | 相关变体 | 处理方式 |
| --- | --- | --- |
| 找不到 `yaoxiang.toml` | `NotProject` / `NotWorkspace` | 在项目根目录内执行命令；工作区场景确认任一 `yaoxiang.toml` 含 `[workspace]` 段 |
| 清单 / 锁文件解析失败 | `InvalidManifest` / `Toml` | 校验 `yaoxiang.toml` 与 `yaoxiang.lock` 的 TOML 语法 |
| 依赖增删异常 | `DependencyNotFound` / `DependencyAlreadyExists` / `DependencyInstallFailed` | 用 `yx list` 查看现有依赖；已存在则先 `yx rm` 再重装 |
| 工作区成员异常 | `MemberMissing` / `MemberInvalid` / `NestedWorkspace` | 核对 `[workspace] members` 路径；嵌套工作区不被支持，需拍平结构 |
| 发布 / 网络类 | `Network` / `RateLimited` / `AuthFailed` / `RegistryDeferred` / `VersionAlreadyExists` / `PublishTarget` / `TestsFailed` | 官方注册表尚未开放，发布请显式加 `--github` 或 `--dry-run`；限流待重试；凭据问题先检查 token |
| 包内容 / 缓存 | `PackageTooLarge` / `InvalidPackage` / `ChecksumMismatch` / `Cache` | 源码包上限 20 MiB；校验和不符说明下载损坏，清理缓存后重试 |
| 底层 I/O | `Io` | 检查磁盘空间、文件权限、文件是否被占用 |

## 常见问题

### Q：安装依赖失败怎么办？

1. 检查网络连接
2. 确认依赖名称与版本约束正确
3. 尝试 `yx update` 刷新索引

### Q：vendor 目录损坏怎么办？

清理缓存后重新安装：`yx cache clean`，再 `yx install`。

### Q：发布时提示官方注册表未开放？

这是预期行为（RFC-014a decision 1）。改用 `yx publish --github` 发布到 GitHub Release，或加 `--dry-run` 先做本地校验。
