---
title: "包管理状态"
---

# 包管理（Package）

> **模块状态**：稳定（4 项待改进）
> **位置**：`src/package/`
> **最后更新**：2026-06-01

---

## 模块概述

包管理模块负责项目依赖管理、包配置解析、依赖下载等。实现了 RFC-014 定义的 Phase 1（toml 解析、本地依赖、lock 生成）和 Phase 2（GitHub 支持、.yaoxiang/vendor 管理、下载工具）。

**代码量**：约 5000 行（23 个源文件）

---

## 功能清单

### 已实现的功能（12 项）

1. ✅ **yaoxiang.toml 清单文件** — 包元数据（name, version, description, authors, license）、依赖声明（dependencies / dev-dependencies）、TOML 序列化/反序列化
2. ✅ **yaoxiang.lock 锁文件** — 锁定依赖条目（version, source, checksum）、从 manifest 同步、强制更新、过期依赖清理
3. ✅ **依赖规格解析 (DependencySpec)** — 从 TOML 值解析（字符串形式 `"1.0.0"` 和表形式 `{version, git, path}`）
4. ✅ **语义化版本解析 (SemVer / VersionReq)** — 解析 `major.minor.patch[-pre]` 格式、支持操作符 `^`, `~`, `>=`, `>`, `<=`, `<`, 精确匹配, `*`
5. ✅ **依赖来源抽象 (Source trait)** — `LocalSource`（本地路径）、`GitSource`（Git 仓库克隆，支持 tag/branch/rev）、`RegistrySource`（占位，Phase 3）
6. ✅ **Git 依赖支持** — URL 解析（`?tag=`, `?branch=`, `?rev=` 参数）、`git ls-remote` 标签列表获取、semver 标签匹配、`git clone --depth 1` 浅克隆
7. ✅ **版本冲突检测** — 自动检测同一包的不兼容版本要求
8. ✅ **模块解析器 (ModuleResolver)** — 按优先级查找：vendor -> src -> YXPATH -> std
9. ✅ **Vendor 目录管理 (VendorManager)** — `.yaoxiang/vendor/<name>-<version>/` 目录管理、安装/卸载/列出/清理
10. ✅ **SHA-256 校验和** — 自行实现的内联 SHA-256（无外部依赖）、文件与目录级校验
11. ✅ **批量下载器 (fetcher)** — 统一的依赖下载接口、与 source/vendor/lock 集成
12. ✅ **CLI 命令（6 个）** — `init`、`add`、`rm`、`install`、`list`、`update`

### RFC 中提及但未实现的功能（3 项）

- ❌ `outdated` 命令 — 检查过时依赖
- ❌ `clean` 命令 — 清理构建产物（仅 vendor 级别有 clean 方法）
- ❌ `task <name>` 命令 — 运行自定义任务

---

## 测试覆盖

**137 个测试，全部通过**

- 每个模块都有完整的单元测试
- 覆盖：正常解析、序列化往返、CRUD 操作、错误路径、边界情况、确定性校验
- 测试使用 `tempfile::TempDir` 隔离文件系统操作

---

## RFC 对比（RFC-014）

### 完全符合 RFC 的部分

- ✅ yaoxiang.toml 格式（[package], [dependencies], [dev-dependencies]）
- ✅ 项目结构（src/, .yaoxiang/vendor/, yaoxiang.toml, yaoxiang.lock）
- ✅ 模块解析顺序（vendor -> src -> YXPATH -> std）
- ✅ Source trait 可扩展架构（Local, Git, Registry 三种来源）
- ✅ CLI 命令（init, add, rm, install, update, list）
- ✅ 语义化版本（^, ~, 精确, 范围操作符）

### 与 RFC 的差异

1. **Lock 文件格式微调** — RFC 使用 `[[package]]` 数组形式，实现使用 `[package.name]` map 形式，功能等价
2. **超出 RFC 的设计** — 自动版本冲突检测、内联 SHA-256 实现、init 命令额外生成 `.yaoxiang/std/` 标准库接口文件

### 未来扩展（Phase 3，RFC 中标注"预留"）

- ❌ Registry 注册表来源 — 仅有占位实现
- ❌ 工作空间（workspace）支持
- ❌ 依赖覆盖（override）机制

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 未完成事项 | 4 | outdated、clean、task 命令、Phase 3 Registry |
| 测试覆盖 | 优秀 | 137 个测试，全部通过 |
| 文档质量 | 良好 | 所有模块有 `//!` 文档注释，公共函数有 `///` 文档 |
| 代码架构 | 优秀 | commands/source/vendor/template 分层清晰 |
| RFC 合规 | 高度符合 | 仅 Lock 文件格式微调 |

---

## 待改进项

1. **实现 `outdated` 命令**
2. **实现 `clean` CLI 命令**
3. **实现 `task <name>` 自定义任务**
4. **开始 Phase 3：Registry 注册表来源**
