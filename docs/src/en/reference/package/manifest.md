---
title: yaoxiang.toml Format
description: Project Configuration File Format Specification
---

# yaoxiang.toml Format

`yaoxiang.toml` is the manifest file for YaoXiang projects, declaring project metadata and dependencies.

## File Structure

```toml
[package]
name = "project name"
version = "0.1.0"
description = "project description"
authors = ["author name"]
license = "MIT"

[dependencies]
# Regular dependencies

[dev-dependencies]
# Development dependencies
```

## Package Section

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Project name, must follow naming conventions (lowercase letters, numbers, hyphens) |
| `version` | string | Yes | Semantic version number, follows semver specification |
| `description` | string | No | Short project description |
| `authors` | array | No | List of authors |
| `license` | string | No | License identifier |

### Example

```toml
[package]
name = "my-awesome-app"
version = "1.2.3"
description = "An awesome application"
authors = ["Zhang San <zhangsan@example.com>"]
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

| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Version number or range |
| `git` | string | Git repository URL |
| `branch` | string | Git branch name |
| `path` | string | Local relative path |

## Version Syntax

| Syntax | Description | Example |
|--------|-------------|---------|
| `*` | Any version | `"*"` |
| `1.0.0` | Exact version | `"1.0.0"` |
| `>=1.0.0` | Minimum version | `">=1.0.0"` |
| `<2.0.0` | Maximum version | `"<2.0.0"` |
| `>=1.0.0, <2.0.0` | Range version | `">=1.0.0, <2.0.0"` |
| `~1.0.0` | Compatible version | `"~1.0.0"` |
| `^1.0.0` | Caret version | `"^1.0.0"` |

## Complete Example

```toml
[package]
name = "web-server"
version = "0.1.0"
description = "A simple web server"
authors = ["Developer <dev@example.com>"]
license = "MIT"

[dependencies]
http = "1.0.0"
json = "2.0.0"
router = { version = "0.5.0", path = "./router" }

[dev-dependencies]
test-utils = "1.0.0"
benchmark = "0.1.0"
```
