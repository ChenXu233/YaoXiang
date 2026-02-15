---
title: yaoxiang.lock 格式
description: 依赖锁定文件格式说明
---

# yaoxiang.lock 格式

`yaoxiang.lock` 是 YaoXiang 的依赖锁定文件，记录所有依赖的精确版本信息。

## 概述

- **自动生成**：由 `yaoxiang install` 和 `yaoxiang update` 自动生成和更新
- **请勿手动编辑**：此文件由包管理器自动维护
- **应提交到版本控制**：确保团队成员和 CI 构建使用相同的依赖版本

## 文件结构

```toml
# 由 YaoXiang 包管理器自动生成

[package]
version = 1

[package.依赖名]
version = "版本号"
source = "来源类型"
checksum = "校验和（可选）"
```

## 字段说明

### [package] 部分

| 字段 | 类型 | 说明 |
|------|------|------|
| `version` | integer | 锁文件格式版本，当前为 1 |

### [package.\<name\>] 部分

| 字段 | 类型 | 说明 |
|------|------|------|
| `version` | string | 解析后的精确版本号 |
| `source` | string | 依赖来源：`registry`、`git`、`path` |
| `checksum` | string | SHA-256 校验和（可选） |

## 示例

```toml
# 由 YaoXiang 包管理器自动生成
# 2024-01-15 10:30:00

[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"
checksum = "abc123..."

[package.json]
version = "2.0.0"
source = "registry"

[package.utils]
version = "0.1.0"
source = "path"

[package.bleeding]
version = "main"
source = "git"
```

## 来源类型

| 类型 | 说明 | 配置示例 |
|------|------|----------|
| `registry` | 从远程注册表获取 | `http = "1.0.0"` |
| `git` | 从 Git 仓库获取 | `{ git = "https://..." }` |
| `path` | 从本地路径获取 | `{ path = "./lib" }` |

## 与 manifest 的关系

```
yaoxiang.toml          yaoxiang.lock
     │                       │
     │   yaoxiang install    │
     ├──────────────────────►│
     │                       │
     │   声明 "http = *>"   │ 锁定 "http = 1.0.0"
     │                       │
```

- `yaoxiang.toml`：声明**想要**的依赖（可使用范围）
- `yaoxiang.lock`：记录**实际安装**的版本（精确版本）
