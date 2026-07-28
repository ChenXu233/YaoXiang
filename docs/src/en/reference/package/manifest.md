---
title: 'yaoxiang.toml Format'
description: Project configuration file format specification
---

# yaoxiang.toml Format

`yaoxiang.toml` is the manifest file for YaoXiang projects, declaring project metadata and dependencies.

## File Structure

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

## package Section

| Field         | Type   | Required | Description                                                        |
| ------------- | ------ | -------- | ------------------------------------------------------------------ |
| `name`        | string | Yes      | Project name, must follow naming conventions (lowercase, digits, hyphens) |
| `version`     | string | Yes      | Semantic version number, follows semver specification              |
| `description` | string | No       | Short project description                                          |
| `authors`     | array  | No       | List of authors                                                    |
| `license`     | string | No       | License identifier                                                 |

### Example

```toml
[package]
name = "my-awesome-app"
version = "1.2.3"
description = "一个很棒的应用"
authors = ["张三 <zhangsan@example.com>"]
license = "MIT"
```

## Dependency Declaration

### Simple Version

```toml
[dependencies]
http = "1.0.0"
json = "*"
```

### Detailed Configuration

```toml
[dependencies]
# Git dependency
http = { version = "1.0.0", git = "https://github.com/example/http" }

# Local path dependency
utils = { version = "0.1.0", path = "./utils" }

# Git dependency with branch
bleeding-edge = { git = "https://github.com/example/edge", branch = "main" }
```

### Dependency Field Reference

| Field      | Type   | Description              |
| ---------- | ------ | ------------------------ |
| `version`  | string | Version number or range  |
| `git`      | string | Git repository address  |
| `branch`   | string | Git branch name          |
| `path`     | string | Local relative path      |

## Version Number Syntax

| Syntax               | Description    | Example                |
| -------------------- | -------------- | ---------------------- |
| `*`                  | Any version    | `"*"`                  |
| `1.0.0`              | Exact version   | `"1.0.0"`              |
| `>=1.0.0`            | Minimum version | `">=1.0.0"`            |
| `<2.0.0`             | Maximum version | `"<2.0.0"`             |
| `>=1.0.0, <2.0.0`    | Range version   | `">=1.0.0, <2.0.0"`    |
| `~1.0.0`             | Compatible version | `"~1.0.0"`          |
| `^1.0.0`             | Caret version   | `"^1.0.0"`             |

## Complete Example

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
