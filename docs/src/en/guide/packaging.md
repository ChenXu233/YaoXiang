---
title: 'Package Manager'
description: 'YaoXiang Official Package Manager Tutorial'
---

# Package Manager Usage Guide

YaoXiang's built-in package manager, providing complete dependency management functionality.

## Overview

The YaoXiang Package Manager (YPM) uses declarative dependency management:

- Declare project dependencies in `yaoxiang.toml`
- `yaoxiang.lock` locks exact versions to ensure reproducible builds
- Dependencies are downloaded to the `vendor` directory

## Quick Start

```bash
# Create a new project
yx init my-project
cd my-project

# Add dependencies
yx add http
yx add json

# Install dependencies
yx install

# Run the project
yx run src/main.yx
```

## Project Structure

```
my-project/
├── yaoxiang.toml      # Project manifest
├── yaoxiang.lock      # Dependency lock file
├── .yaoxiang/vendor/       # Dependency storage (actual test: yx init generates .yaoxiang/vendor/std/)
└── src/
    └── main.yx
```

---

## init

Initialize a new project.

### Usage

```bash
yx init <name>
```

### Arguments

| Argument | Type   | Description  |
| -------- | ------ | ------------ |
| `name`   | string | Project name |

### Description

Create a new YaoXiang project in the current directory or at the specified path.

### Files Created

- `yaoxiang.toml` - Project manifest
- `yaoxiang.lock` - Dependency lock file
- `src/main.yx` - Entry file
- `.gitignore` - Git ignore configuration

### Example

```bash
# Create a project in the current directory
yx init my-project

# Output
# ✨ Project created: my-project
#   my-project/yaoxiang.toml
#   my-project/yaoxiang.lock
#   my-project/src/main.yx
#   my-project/.gitignore
```

---

## add

Add dependencies to the project.

### Usage

```bash
yx add <name> --version <version>
yx add <name> --dev
```

### Arguments

| Argument  | Type   | Description                            |
| --------- | ------ | -------------------------------------- |
| `name`    | string | Package name                           |
| `version` | string | Version number (optional, default `*`) |

### Options

| Option        | Description                     |
| ------------- | ------------------------------- |
| `--dev`, `-D` | Add as a development dependency |

### Description

Add dependencies to the project's `yaoxiang.toml` file, and update `yaoxiang.lock`.

### Version Specifications

| Specification | Description        | Example           |
| ------------- | ------------------ | ----------------- |
| `*`           | Any version        | `http = "*"`      |
| `1.0.0`       | Exact version      | `http = "1.0.0"`  |
| `>=1.0.0`     | Minimum version    | `http = ">1.0.0"` |
| `~1.0.0`      | Compatible version | `http = "~1.0.0"` |
| `^1.0.0`      | caret version      | `http = "^1.0.0"` |

### Dependency Sources

#### Registry (default)

```bash
yx add http
yx add http --version 1.0.0
```

#### Git Repository

```bash
# The following configuration will be generated in the manifest
# http = { version = "1.0.0", git = "https://github.com/example/http" }
```

#### Local Path

```bash
# The following configuration will be generated in the manifest
# mylib = { version = "0.1.0", path = "./mylib" }
```

### Example

```bash
# Add the latest version
yx add http

# Add a specific version
yx add http 1.0.0

# Add a version range
yx add json --version ">=2.0.0"

# Add a development dependency
yx add test-utils --dev
yx add benchmark -D
```

---

## rm

Remove dependencies from the project.

### Usage

```bash
yx rm <name>
yx rm <name> --dev
```

### Arguments

| Argument | Type   | Description  |
| -------- | ------ | ------------ |
| `name`   | string | Package name |

### Options

| Option        | Description                     |
| ------------- | ------------------------------- |
| `--dev`, `-D` | Remove a development dependency |

### Description

Remove the specified dependency from the project's `yaoxiang.toml`, and update `yaoxiang.lock`.

### Example

```bash
# Remove runtime dependency
yx rm http

# Remove development dependency
yx rm test-utils --dev
```

---

## install

Install project dependencies.

### Usage

```bash
yx install
```

### Description

Read dependency declarations from `yaoxiang.toml` and perform the following operations:

1. Resolve dependency versions
2. Detect version conflicts
3. Download dependencies to the `vendor` directory
4. Generate/update `yaoxiang.lock`

### Behavior

- If there are no dependencies, display a prompt message and exit
- If the `vendor` directory already exists, check and reuse the cache
- If a version conflict is detected, display an error message and exit

### Example

```bash
# Install all dependencies
yx install

# Output
# 📦 Resolving dependencies...
#   http (1.0.0) [Installed]
#   json (2.0.0) [Cached]
# ✅ Dependencies installed, lock file updated
```

### Lock File Update

The `install` command updates `yaoxiang.lock`:

```toml
# yaoxiang.lock
[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"

[package.json]
version = "2.0.0"
source = "registry"
```

---

## update

Update project dependencies.

### Usage

```bash
yx update
yx update <name>
```

### Arguments

| Argument | Type   | Description             |
| -------- | ------ | ----------------------- |
| `name`   | string | Package name (optional) |

### Description

### Full Update

Without arguments, update all dependencies:

1. Clear currently locked versions
2. Clean up old versions in the `vendor` directory
3. Re-download all dependencies
4. Update `yaoxiang.lock`

### Single Update

With arguments, update only the specified dependency:

1. Delete the old version from `vendor`
2. Re-download the new version
3. Update the corresponding entry in `yaoxiang.lock`
4. Other dependencies are not affected

### Example

```bash
# Update all dependencies
yx update

# Output
# 📦 Updating dependencies...
#   http (1.0.0 → 1.1.0)
#   json (2.0.0 → 2.1.0)
# ✅ Updated 2 dependencies, lock file updated

# Update a single dependency
yx update http

# Output
# ✅ Updated http (1.0.0 → 1.1.0)
```

---

## list

List project dependencies.

### Usage

```bash
yx list
```

### Description

Display all dependencies in the project, including:

- Runtime dependencies (from `[dependencies]`)
- Development dependencies (from `[dev-dependencies]`)
- Each dependency's version and source

### Example

```bash
yx list

# Output
# 📦 Project Dependencies
#
# Runtime Dependencies:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# Development Dependencies:
#   test-utils  0.5.0    registry
```

---

## Configuration Files

### yaoxiang.toml

Project manifest file, declares project metadata and dependencies.

```toml
[package]
name = "my-project"
version = "0.1.0"
description = "项目描述"
authors = ["作者 <email@example.com>"]
license = "MIT"

[dependencies]
http = "1.0.0"
json = "*"

[dev-dependencies]
test-utils = "0.5.0"
```

### yaoxiang.lock

Dependency lock file, automatically generated by the package manager.

```toml
# Auto-generated by YaoXiang Package Manager

[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"
```

---

## Core Concepts

### Runtime Dependencies vs Development Dependencies

- **Runtime dependencies** (`[dependencies]`): Packages required when the project runs
- **Development dependencies** (`[dev-dependencies]`): Packages required only for development and
  testing

### Dependency Sources

| Type     | Configuration Example                        | Description                        |
| -------- | -------------------------------------------- | ---------------------------------- |
| Registry | `http = "1.0.0"`                             | Fetch from remote package registry |
| Git      | `{ version = "1.0.0", git = "https://..." }` | Fetch from Git repository          |
| Path     | `{ version = "0.1.0", path = "./lib" }`      | Fetch from local path              |

### Lock File

`yaoxiang.lock` is automatically generated by the package manager, please **be sure to commit it to
version control**:

- Ensure team members use exactly the same dependency versions
- Ensure CI builds are reproducible
- Avoid the "works on my machine" problem

### vendor Directory

Dependencies are stored in the `vendor` directory after downloading:

- Automatically managed by `yx install` and `yx update`
- Can be deleted and then re-run `install` to rebuild
- Recommended to add to `.gitignore`, managed independently by different team members

---

## Frequently Asked Questions

### Q: What if there is a dependency version conflict?

YPM will detect dependency version conflicts and report an error. Solutions:

1. Adjust dependency version requirements
2. Wait for the dependency author to fix it
3. Consider removing the conflicting dependency

### Q: How to use private packages?

For private packages, you can use Git source:

```bash
# Add via Git URL
# Manually edit yaoxiang.toml
[dependencies]
private-pkg = { version = "1.0.0", git = "https://github.com/org/private-pkg" }
```

### Q: Can the vendor directory be deleted?

Yes. After deletion, run `yx install` to re-download all dependencies.

### Q: How to view information about a package?

Use `yx list` to view all dependencies, or check `yaoxiang.toml`.
