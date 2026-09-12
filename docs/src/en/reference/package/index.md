---
title: 'Package Manager'
description: 'YaoXiang Package Manager Reference Documentation'
---

# Package Manager

The built-in package manager for YaoXiang, providing project initialization, dependency management,
version locking, and other features.

## Overview

The YaoXiang Package Manager (abbreviated as YPM) adopts a design philosophy similar to Cargo:

- **Declarative dependencies**: Declare required dependencies in `yaoxiang.toml`
- **Deterministic builds**: Lock versions through `yaoxiang.lock` to ensure reproducible builds
- **Local cache**: Dependencies are downloaded to the `vendor` directory, supporting offline use

## Quick Start

```bash
# 1. Create a new project
yx init my-project

# 2. Add dependencies
cd my-project
yx add http

# 3. Install dependencies
yx install

# 4. Run the project
yx run src/main.yx
```

## Command List

| Command                               | Description              |
| ------------------------------------- | ------------------------ |
| [`yx init`](./commands#yx-init)       | Initialize a new project |
| [`yx add`](./commands#yx-add)         | Add dependencies         |
| [`yx rm`](./commands#yx-rm)           | Remove dependencies      |
| [`yx install`](./commands#yx-install) | Install dependencies     |
| [`yx update`](./commands#yx-update)   | Update dependencies      |
| [`yx list`](./commands#yx-list)       | List dependencies        |

## Project Structure

```
my-project/
├── yaoxiang.toml      # Project manifest (required)
├── yaoxiang.lock      # Dependency lock file (auto-generated)
├── vendor/            # Dependency storage directory (auto-generated)
└── src/
    └── main.yx       # Entry file
```

## Documentation Index

- [Command Line Interface](./commands) - Detailed description of all commands
- [yaoxiang.toml Format](./manifest) - Project configuration file format
- [yaoxiang.lock Format](./lock) - Lock file format description
- [Error Codes](./error-codes) - Common errors and handling methods
