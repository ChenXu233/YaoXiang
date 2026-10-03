---
title: 'yaoxiang.toml Format'
description: 'Project configuration file format specification'
---

# yaoxiang.toml Format

`yaoxiang.toml` is the manifest file for a YaoXiang project, declaring project metadata,
dependencies, and entry points.

## File Structure

```toml
[package]
name = "项目名称"
version = "0.1.0"
description = "项目描述"
authors = ["作者名"]
license = "MIT"
repository = "https://github.com/org/repo"

[dependencies]
# Runtime dependencies

[dev-dependencies]
# Dev dependencies
```

## [package]

| Field         | Type   | Required | Description                                                       |
| ------------- | ------ | -------- | ----------------------------------------------------------------- |
| `name`        | string | Yes      | Project name                                                      |
| `version`     | string | Yes      | Semantic version number                                           |
| `description` | string | No       | Project description; **required for `publish`**                   |
| `authors`     | array  | No       | List of authors                                                   |
| `license`     | string | No       | License identifier                                                |
| `repository`  | string | No       | Repository URL; one of the targets resolved by `publish --github` |

## Dependency Declaration

### Available Sources

| Source            | Syntax                    | Status                                              |
| ----------------- | ------------------------- | --------------------------------------------------- |
| Workspace member  | `{ workspace = "key" }`   | ✅ Available                                        |
| Git repository    | `{ git = "https://..." }` | ✅ Available                                        |
| Official registry | `some-lib = "1.0.0"`      | ⛔ Not implemented (RFC-014a deferred indefinitely) |
| Local path        | `{ path = "../lib" }`     | ⚠️ Known issue: cannot be `use`d after install      |

### Example

```toml
[dependencies]
# Git dependency (currently the only channel for cross-repo distribution)
some-lib = { git = "https://github.com/example/some-lib", version = "^0.1.0" }

# Workspace member
utils = { workspace = "mylib" }

# Dev dependency
test-utils = { git = "https://github.com/example/test-utils" }
```

### Dependency Fields

| Field       | Type           | Description                                                  |
| ----------- | -------------- | ------------------------------------------------------------ |
| `version`   | string         | Version number or version range                              |
| `git`       | string         | Git repository URL                                           |
| `path`      | string         | Local relative path (see known issue above)                  |
| `workspace` | string \| bool | Reference a workspace member or inherit the root declaration |

::: warning The `branch` field is currently invalid There is **no** `branch` field on dependencies.
Writing `branch = "main"` will not produce an error, but it will not take effect — the dependency is
still resolved against the default branch.

To pin a branch, tag, or commit, put the ref into the **git URL's query string**:

```toml
[dependencies]
pinned-dev = { git = "https://github.com/example/lib?branch=dev" }
at-tag    = { git = "https://github.com/example/lib?tag=v1.0.0" }
at-rev    = { git = "https://github.com/example/lib?rev=abc1234" }
```

Dependencies pinned this way are reported as `pinned` by `yx outdated` and are excluded from version
comparison. :::

## Version Syntax

| Syntax              | Description                  | Example             |
| ------------------- | ---------------------------- | ------------------- |
| `1.0.0`             | **Exact match** (not caret)  | `"1.0.0"`           |
| `*`                 | Any version                  | `"*"`               |
| `^1.0.0`            | caret, allows minor upgrades | `"^1.0.0"`          |
| `~1.0.0`            | Allows patch upgrades        | `"~1.0.0"`          |
| `>=1.0.0`, `<2.0.0` | Lower/upper bound            | `">=1.0.0"`         |
| `>=1.0.0, <2.0.0`   | Comma combination            | `">=1.0.0, <2.0.0"` |
| `1.*`               | Segment wildcard             | `"1.*"`             |

`1.2` and `1` are automatically padded to `1.2.0` and `1.0.0`.

## [workspace]

The `yaoxiang.toml` at the **root** of a workspace uses the `[workspace]` section, and contains no
`[package]`:

```toml
[workspace.members]
app = "app/yaoxiang.toml"
mylib = "mylib/yaoxiang.toml"

[workspace.dependencies]
# Pin versions at the root, members inherit via { workspace = true }
```

The value of `members` must be the **manifest file path**, not a directory. A member references a
sibling member with `{ workspace = "<key>" }`, where `key` is the name it was registered under.

::: warning Differences between the root and member directories The workspace root has no
`[package]` section, so `list`, `outdated`, and `update` report `missing field package` when run at
the root — these must be run inside a member directory. `install`, `workspace`, `publish`, `cache`,
and `clean` are available at the root. :::

## Other Sections

| Section      | Description                                                                        |
| ------------ | ---------------------------------------------------------------------------------- |
| `[lib]`      | Library entry file (relative to the manifest directory)                            |
| `[[bin]]`    | Binary target entries                                                              |
| `[run]`      | Default entry and arguments                                                        |
| `[exports]`  | `use` path prefix → file path mapping (cross-package export surface, RFC-015/029f) |
| `[build]`    | Build declaration: strategy, C headers, tool version requirements (RFC-014b)       |
| `[binaries]` | Pre-built artifact declaration, platform triple → artifact                         |
| `[i18n]`     | Project-level language configuration                                               |

## Complete Example

```toml
[package]
name = "web-server"
version = "0.1.0"
description = "一个简单的 Web 服务器"
authors = ["开发者 <dev@example.com>"]
license = "MIT"
repository = "https://github.com/org/web-server"

[dependencies]
some-lib = { git = "https://github.com/example/some-lib", version = "^0.1.0" }
mylib = { workspace = "mylib" }

[dev-dependencies]
test-utils = { git = "https://github.com/example/test-utils" }
```
