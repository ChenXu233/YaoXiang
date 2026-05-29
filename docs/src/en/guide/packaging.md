---
title: Package Manager
description: YaoXiang Official Package Manager Tutorial
---

# Package Manager

YaoXiang's built-in package manager providing complete dependency management functionality.

## Overview

YaoXiang Package Manager (YPM) uses declarative dependency management:

- Declare project dependencies in `yaoxiang.toml`
- `yaoxiang.lock` locks exact versions to ensure reproducible builds
- Dependencies are downloaded to the `vendor` directory

## Quick Start

```bash
# Create a new project
yaoxiang init my-project
cd my-project

# Add dependencies
yaoxiang add http
yaoxiang add json

# Install dependencies
yaoxiang install

# Run the project
yaoxiang run src/main.yx
```

## Project Structure

```
my-project/
├── yaoxiang.toml      # Project manifest
├── yaoxiang.lock      # Dependency lock file
├── vendor/            # Dependency storage
└── src/
    └── main.yx
```

---

## init

Initialize a new project.

### Usage

```bash
yaoxiang init <name>
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | Project name |

### Description

Creates a new YaoXiang project in the current directory or at the specified path.

### Created Files

- `yaoxiang.toml` - Project manifest
- `yaoxiang.lock` - Dependency lock file
- `src/main.yx` - Entry file
- `.gitignore` - Git ignore configuration

### Examples

```bash
# Create project in current directory
yaoxiang init my-project

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
yaoxiang add <name> [version]
yaoxiang add <name> --dev
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | Package name |
| `version` | string | Version number (optional, default `*`) |

### Options

| Option | Description |
|--------|-------------|
| `--dev`, `-D` | Add as dev dependency |

### Description

Adds the dependency to the project's `yaoxiang.toml` file and updates `yaoxiang.lock`.

### Version Specifications

| Specifier | Description | Example |
|-----------|-------------|---------|
| `*` | Any version | `http = "*"` |
| `1.0.0` | Exact version | `http = "1.0.0"` |
| `>=1.0.0` | Minimum version | `http = ">1.0.0"` |
| `~1.0.0` | Compatible version | `http = "~1.0.0"` |
| `^1.0.0` | Caret version | `http = "^1.0.0"` |

### Dependency Sources

#### Registry (default)

```bash
yaoxiang add http
yaoxiang add http 1.0.0
```

#### Git Repository

```bash
# This generates the following configuration in the manifest
# http = { version = "1.0.0", git = "https://github.com/example/http" }
```

#### Local Path

```bash
# This generates the following configuration in the manifest
# mylib = { version = "0.1.0", path = "./mylib" }
```

### Examples

```bash
# Add latest version
yaoxiang add http

# Add specific version
yaoxiang add http 1.0.0

# Add version range
yaoxiang add json ">=2.0.0"

# Add dev dependency
yaoxiang add test-utils --dev
yaoxiang add benchmark -D
```

---

## rm

Remove dependencies from the project.

### Usage

```bash
yaoxiang rm <name>
yaoxiang rm <name> --dev
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | Package name |

### Options

| Option | Description |
|--------|-------------|
| `--dev`, `-D` | Remove dev dependency |

### Description

Removes the specified dependency from the project's `yaoxiang.toml` and updates `yaoxiang.lock`.

### Examples

```bash
# Remove runtime dependency
yaoxiang rm http

# Remove dev dependency
yaoxiang rm test-utils --dev
```

---

## install

Install project dependencies.

### Usage

```bash
yaoxiang install
```

### Description

Reads dependency declarations from `yaoxiang.toml` and performs the following:

1. Resolve dependency versions
2. Detect version conflicts
3. Download dependencies to `vendor` directory
4. Generate/update `yaoxiang.lock`

### Behavior

- If there are no dependencies, displays a notification and exits
- If the `vendor` directory already exists, checks and reuses cached dependencies
- If version conflicts are detected, displays an error message and exits

### Examples

```bash
# Install all dependencies
yaoxiang install

# Output
# 📦 Resolving dependencies...
#   http (1.0.0) [installed]
#   json (2.0.0) [cached]
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
yaoxiang update
yaoxiang update <name>
```

### Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | Package name (optional) |

### Description

### Full Update

When called without arguments, updates all dependencies:

1. Clears currently locked versions
2. Cleans old versions from the `vendor` directory
3. Re-downloads all dependencies
4. Updates `yaoxiang.lock`

### Single Update

When called with an argument, only updates the specified dependency:

1. Removes the old version from `vendor`
2. Re-downloads the new version
3. Updates the corresponding entry in `yaoxiang.lock`
4. Other dependencies are unaffected

### Examples

```bash
# Update all dependencies
yaoxiang update

# Output
# 📦 Updating dependencies...
#   http (1.0.0 → 1.1.0)
#   json (2.0.0 → 2.1.0)
# ✅ Updated 2 dependencies, lock file updated

# Update single dependency
yaoxiang update http

# Output
# ✅ Updated http (1.0.0 → 1.1.0)
```

---

## list

List project dependencies.

### Usage

```bash
yaoxiang list
```

### Description

Displays all dependencies in the project, including:

- Runtime dependencies (from `[dependencies]`)
- Dev dependencies (from `[dev-dependencies]`)
- Version and source for each dependency

### Examples

```bash
yaoxiang list

# Output
# 📦 Project Dependencies
#
# Runtime Dependencies:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# Dev Dependencies:
#   test-utils  0.5.0    registry
```

---

## Configuration Files

### yaoxiang.toml

Project manifest file that declares project metadata and dependencies.

```toml
[package]
name = "my-project"
version = "0.1.0"
description = "Project description"
authors = ["Author <email@example.com>"]
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

### Runtime Dependencies vs Dev Dependencies

- **Runtime dependencies** (`[dependencies]`): Packages required for the project to run
- **Dev dependencies** (`[dev-dependencies]`): Packages only needed during development and testing

### Dependency Sources

| Type | Configuration Example | Description |
|------|----------------------|-------------|
| Registry | `http = "1.0.0"` | Fetched from remote package registry |
| Git | `{ version = "1.0.0", git = "https://..." }` | Fetched from Git repository |
| Path | `{ version = "0.1.0", path = "./lib" }` | Fetched from local path |

### Lock File

`yaoxiang.lock` is automatically generated by the package manager. Please **be sure to commit it to version control**:

- Ensures all team members use exactly the same dependency versions
- Ensures CI builds are reproducible
- Avoids "works on my machine" problems

### vendor Directory

Dependencies are stored in the `vendor` directory after downloading:

- Automatically managed by `yaoxiang install` and `yaoxiang update`
- Can be deleted and rebuilt by running `install` again
- Recommended to add to `.gitignore` so team members manage independently

---

## FAQ

### Q: What to do about dependency version conflicts?

YPM detects dependency version conflicts and reports errors. Solutions:

1. Adjust dependency version requirements
2. Wait for the dependency author to fix it
3. Consider removing the conflicting dependency

### Q: How to use private packages?

For private packages, you can use the Git source:

```bash
# Add via Git URL
# Manually edit yaoxiang.toml
[dependencies]
private-pkg = { version = "1.0.0", git = "https://github.com/org/private-pkg" }
```

### Q: Can the vendor directory be deleted?

Yes. After deletion, running `yaoxiang install` will re-download all dependencies.

### Q: How to view information about a specific package?

Use `yaoxiang list` to view all dependencies, or check `yaoxiang.toml`.