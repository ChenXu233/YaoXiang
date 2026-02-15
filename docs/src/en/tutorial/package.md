---
title: Package Manager
description: YaoXiang Package Manager Reference Documentation
---

# Package Manager

YaoXiang's built-in package manager, providing project initialization, dependency management, version locking, and more.

## Overview

The YaoXiang Package Manager (YPM) follows a Cargo-like design philosophy:

- **Declarative Dependencies**: Declare required dependencies in `yaoxiang.toml`
- **Deterministic Builds**: Lock versions via `yaoxiang.lock` to ensure reproducible builds
- **Local Cache**: Dependencies are downloaded to the `vendor` directory, supporting offline usage

## Quick Start

```bash
# 1. Create a new project
yaoxiang init my-project

# 2. Add dependencies
cd my-project
yaoxiang add http

# 3. Install dependencies
yaoxiang install

# 4. Run the project
yaoxiang run src/main.yx
```

## Command List

| Command | Description |
|---------|-------------|
| [`yaoxiang init`](./commands#yaoxiang-init) | Initialize a new project |
| [`yaoxiang add`](./commands#yaoxiang-add) | Add dependencies |
| [`yaoxiang rm`](./commands#yaoxiang-rm) | Remove dependencies |
| [`yaoxiang install`](./commands#yaoxiang-install) | Install dependencies |
| [`yaoxiang update`](./commands#yaoxiang-update) | Update dependencies |
| [`yaoxiang list`](./commands#yaoxiang-list) | List dependencies |

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
- [yaoxiang.lock Format](./lock) - Lock file format explanation
- [Error Codes](./error-codes) - Common errors and handling methods
