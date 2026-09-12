---
title: 'Package Manager'
description: 'YaoXiang official package manager user guide'
---

# Package Manager

YaoXiang's built-in package manager provides complete dependency management functionality.

## Overview

YaoXiang Package Manager (YPM) uses declarative dependency management:

- Declare project dependencies in `yaoxiang.toml`
- `yaoxiang.lock` locks exact versions, ensuring reproducible builds
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
├── vendor/            # Dependency storage
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

Creates a new YaoXiang project in the current directory or at the specified path.

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

Add a dependency to the project.

### Usage

```bash
yx add <name> [version]
yx add <name> --dev
```

### Arguments

| Argument  | Type   | Description                            |
| --------- | ------ | -------------------------------------- |
| `name`    | string | Package name                           |
| `version` | string | Version number (optional, default `*`) |

### Options

| Option        | Description             |
| ------------- | ----------------------- |
| `--dev`, `-D` | Add as a dev dependency |

### Description

Adds the dependency to the project's `yaoxiang.toml` file and updates `yaoxiang.lock`.

### Version Specifiers

| Specifier | Description        | Example            |
| --------- | ------------------ | ------------------ |
| `*`       | Any version        | `http = "*"`       |
| `1.0.0`   | Exact version      | `http = "1.0.0"`   |
| `>=1.0.0` | Minimum version    | `http = ">=1.0.0"` |
| `~1.0.0`  | Compatible version | `http = "~1.0.0"`  |
| `^1.0.0`  | caret version      | `http = "^1.0.0"`  |

### Dependency Sources

#### Registry (Default)

```bash
yx add http
yx add http 1.0.0
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
yx add json ">=2.0.0"

# Add a dev dependency
yx add test-utils --dev
yx add benchmark -D
```

---

## rm

Remove a dependency from the project.

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

| Option        | Description           |
| ------------- | --------------------- |
| `--dev`, `-D` | Remove dev dependency |

### Description

Removes the specified dependency from the project's `yaoxiang.toml` and updates `yaoxiang.lock`.

### Example

```bash
# Remove a runtime dependency
yx rm http

# Remove a dev dependency
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

Reads the dependency declarations in `yaoxiang.toml` and performs the following operations:

1. Resolve dependency versions
2. Detect version conflicts
3. Download dependencies to the `vendor` directory
4. Generate/update `yaoxiang.lock`

### Behavior

- If there are no dependencies, display a message and exit
- If the `vendor` directory already exists, check and reuse the cache
- If version conflicts are detected, display an error message and exit

### Example

```bash
# Install all dependencies
yx install

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
yx update
yx update <name>
```

### Arguments

| Argument | Type   | Description             |
| -------- | ------ | ----------------------- |
| `name`   | string | Package name (optional) |

### Description

### Full Update

Without arguments, updates all dependencies:

1. Clear the currently locked versions
2. Remove old versions from the `vendor` directory
3. Re-download all dependencies
4. Update `yaoxiang.lock`

### Single Update

With an argument, updates only the specified dependency:

1. Remove the old version from `vendor`
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

Displays all dependencies in the project, including:

- Runtime dependencies (from `[dependencies]`)
- Dev dependencies (from `[dev-dependencies]`)
- The version and source of each dependency

### Example

```bash
yx list

# Output
# 📦 Project dependencies
#
# Runtime dependencies:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# Dev dependencies:
#   test-utils  0.5.0    registry
```

---

## Configuration Files

### yaoxiang.toml

The project manifest file, declaring project metadata and dependencies.

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

The dependency lock file, automatically generated by the package manager.

```toml
# Automatically generated by the YaoXiang package manager

[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"
```

---

## Core Concepts

### Runtime Dependencies vs Dev Dependencies

- **Runtime dependencies** (`[dependencies]`): packages required at runtime
- **Dev dependencies** (`[dev-dependencies]`): packages only needed during development and testing

### Dependency Sources

| Type     | Configuration Example                        | Description                    |
| -------- | -------------------------------------------- | ------------------------------ |
| Registry | `http = "1.0.0"`                             | Fetched from a remote registry |
| Git      | `{ version = "1.0.0", git = "https://..." }` | Fetched from a Git repository  |
| Path     | `{ version = "0.1.0", path = "./lib" }`      | Fetched from a local path      |

### Lock File

`yaoxiang.lock` is automatically generated by the package manager. Be sure to **commit it to version
control**:

- Ensures team members use exactly the same dependency versions
- Ensures reproducible CI builds
- Avoids "works on my machine" issues

### vendor Directory

Dependencies are stored in the `vendor` directory after download:

- Automatically managed by `yx install` and `yx update`
- Can be deleted and rebuilt by running `install` again
- Recommended to be added to `.gitignore`; each team member manages it independently

---

## FAQ

### Q: What should I do if there are dependency version conflicts?

YPM detects dependency version conflicts and reports an error. Solutions:

1. Adjust the dependency version requirements
2. Wait for the dependency author to fix it
3. Consider removing the conflicting dependency

### Q: How do I use private packages?

For private packages, you can use a Git source:

```bash
# Add via Git URL
# Manually edit yaoxiang.toml
[dependencies]
private-pkg = { version = "1.0.0", git = "https://github.com/org/private-pkg" }
```

### Q: Can the vendor directory be deleted?

Yes. After deletion, running `yx install` will re-download all dependencies.

### Q: How do I view information about a specific package?

Use `yx list` to view all dependencies, or check `yaoxiang.toml`.
