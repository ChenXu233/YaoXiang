---
title: 'Package Manager'
description: 'YaoXiang Package Manager Reference Documentation'
---

# Package Manager

YaoXiang has a built-in package manager that provides project initialization, dependency management,
version locking, and workspace support.

For the complete user-facing workflow (including copy-pasteable repository creation steps), see the
[Package Management System Guide](../../guide/packaging). This page is a reference index.

## Design Principles

- **Declarative dependencies**: declare required dependencies in `yaoxiang.toml`
- **Deterministic builds**: lock versions via `yaoxiang.lock` to ensure reproducible builds
- **Local cache**: dependencies are downloaded to `.yaoxiang/vendor/`, supporting offline reuse

## Dependency Source Status

| Source            | Manifest syntax         | Status                                                   |
| ----------------- | ----------------------- | -------------------------------------------------------- |
| Workspace member  | `{ workspace = "key" }` | ✅ Available                                             |
| Git repository    | `{ git = "<url>" }`     | ✅ Available                                             |
| Official registry | `name = "1.0.0"`        | ⛔ Not yet implemented (RFC-014a indefinitely postponed) |
| Local path        | `{ path = "../lib" }`   | ⚠️ Known issues, see the guide                           |

`yx add` without `--git` / `--path` falls back to the not-yet-implemented registry source, and
`yx install` will subsequently fail.

## Project Structure

The actual output of `yx init`:

```
my-project/
├── yaoxiang.toml            # Project manifest
├── yaoxiang.lock            # Dependency lock file
├── .gitignore               # Already includes .yaoxiang/
├── tests/                   # Test directory
├── src/
│   └── main.yx              # Entry file (lib.yx with --lib)
└── .yaoxiang/
    └── vendor/
        ├── std/              # Standard library interface files
        └── <包名>-<版本>/    # Third-party dependencies
```

## Command Overview

`yx` provides 21 subcommands, of which 11 are related to package management:

| Command                                       | Description                                         |
| --------------------------------------------- | --------------------------------------------------- |
| [`yx init`](./commands#yx-init)               | Initialize a project, supports `--lib`              |
| [`yx add`](./commands#yx-add)                 | Add dependency                                      |
| [`yx install`](./commands#yx-install)         | Install dependencies                                |
| [`yx update`](./commands#yx-update)           | Update dependencies                                 |
| [`yx list`](./commands#yx-list)               | List dependencies and sources                       |
| [`yx rm`](./commands#yx-rm)                   | Remove dependency                                   |
| [`yx outdated`](./commands#yx-outdated)       | Check upgradable dependencies                       |
| [`yx clean`](./commands#yx-clean)             | Clean build artifacts and redundant vendor packages |
| [`yx cache clean`](./commands#yx-cache-clean) | Clear global cache                                  |
| [`yx workspace`](./commands#yx-workspace)     | Workspace member management                         |
| [`yx publish`](./commands#yx-publish)         | Package and publish                                 |

The remaining subcommands (`run`, `check`, `test`, `build`, `format`, `lsp`, `repl`, `eval`,
`explain`, `dump`) are not related to package management.

## Documentation Index

- [Command-Line Interface](./commands) - Detailed description of all package management commands
- [yaoxiang.toml format](./manifest) - Project configuration file format
- [yaoxiang.lock format](./lock) - Lock file format description
- [Error Codes](./error-codes) - Package management related errors and how to handle them
- [Package Management System Guide](../../guide/packaging) - Complete user-facing workflow
