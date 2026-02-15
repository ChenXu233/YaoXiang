---
title: yaoxiang.toml 格式
description: 项目配置文件格式说明
---

# yaoxiang.toml 格式

`yaoxiang.toml` 是 YaoXiang 项目的清单文件，声明项目元数据和依赖。

## 文件结构

```toml
[package]
name = "项目名称"
version = "0.1.0"
description = "项目描述"
authors = ["作者名"]
license = "MIT"

[dependencies]
# 普通依赖

[dev-dependencies]
# 开发依赖
```

## package 部分

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `name` | string | 是 | 项目名称，需符合命名规范（小写字母、数字、连字符） |
| `version` | string | 是 | 语义化版本号，遵循 semver 规范 |
| `description` | string | 否 | 项目简短描述 |
| `authors` | array | 否 | 作者列表 |
| `license` | string | 否 | 许可证标识符 |

### 示例

```toml
[package]
name = "my-awesome-app"
version = "1.2.3"
description = "一个很棒的应用"
authors = ["张三 <zhangsan@example.com>"]
license = "MIT"
```

## 依赖声明

### 简单版本

```toml
[dependencies]
http = "1.0.0"
json = "*"
```

### 详细配置

```toml
[dependencies]
# Git 依赖
http = { version = "1.0.0", git = "https://github.com/example/http" }

# 本地路径依赖
utils = { version = "0.1.0", path = "./utils" }

# 带分支的 Git 依赖
bleeding-edge = { git = "https://github.com/example/edge", branch = "main" }
```

### 依赖字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `version` | string | 版本号或版本范围 |
| `git` | string | Git 仓库地址 |
| `branch` | string | Git 分支名 |
| `path` | string | 本地相对路径 |

## 版本号语法

| 语法 | 说明 | 示例 |
|------|------|------|
| `*` | 任意版本 | `"*"` |
| `1.0.0` | 精确版本 | `"1.0.0"` |
| `>=1.0.0` | 最低版本 | `">=1.0.0"` |
| `<2.0.0` | 最高版本 | `"<2.0.0"` |
| `>=1.0.0, <2.0.0` | 范围版本 | `">=1.0.0, <2.0.0"` |
| `~1.0.0` | 兼容版本 | `"~1.0.0"` |
| `^1.0.0` |  caret 版本 | `"^1.0.0"` |

## 完整示例

```toml
[package]
name = "web-server"
version = "0.1.0"
description = "一个简单的 Web 服务器"
authors = ["开发者 <dev@example.com>"]
license = "MIT"

[dependencies]
http = "1.0.0"
json = "2.0.0"
router = { version = "0.5.0", path = "./router" }

[dev-dependencies]
test-utils = "1.0.0"
benchmark = "0.1.0"
```
