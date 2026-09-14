---
title: 'Command Line Interface'
description: 'Detailed description of all package manager commands'
---

# Command Line Interface

## yx init

Initialize a new YaoXiang project.

### Usage

```bash
yx init <project_name>
```

### Arguments

| Argument     | Description             |
| ------------ | ----------------------- |
| project_name | Name of the new project |

### Options

| Option   | Description           |
| -------- | --------------------- |
| `--help` | Show help information |

### Example

```bash
# Create a new project
yx init my-project

# Result:
# ✨ Project created: my-project
#   my-project/yaoxiang.toml
#   my-project/yaoxiang.lock
#   my-project/src/main.yx
#   my-project/.gitignore
```

---

## yx add

Add a dependency to the project.

### Usage

```bash
yx add <package_name> [version]
yx add <package_name> --dev
```

### Arguments

| Argument     | Description                                |
| ------------ | ------------------------------------------ |
| package_name | Name of the dependency to add              |
| version      | Version number (optional, defaults to `*`) |

### Options

| Option        | Description                     |
| ------------- | ------------------------------- |
| `--dev`, `-D` | Add as a development dependency |

### Example

```bash
# Add the latest version
yx add http

# Add a specific version
yx add http 1.0.0

# Add a version range
yx add json ">=2.0.0"

# Add a development dependency
yx add test-utils --dev
yx add benchmark -D
```

---

## yx rm

Remove a dependency from the project.

### Usage

```bash
yx rm <package_name>
yx rm <package_name> --dev
```

### Arguments

| Argument     | Description                      |
| ------------ | -------------------------------- |
| package_name | Name of the dependency to remove |

### Options

| Option        | Description                     |
| ------------- | ------------------------------- |
| `--dev`, `-D` | Remove a development dependency |

### Example

```bash
# Remove a regular dependency
yx rm http

# Remove a development dependency
yx rm test-utils --dev
```

---

## yx install

Install project dependencies.

### Usage

```bash
yx install
```

### Description

- Read the dependency declarations from `yaoxiang.toml`
- Download all dependencies to the `vendor` directory
- Generate/update `yaoxiang.lock` to lock versions
- Detect dependency version conflicts

### Example

```bash
# Install all dependencies
yx install

# Sample output:
# 📦 Resolving dependencies...
#   http (1.0.0) [installed]
#   json (2.0.0) [cached]
# ✅ Dependencies installed, lock file updated
```

---

## yx update

Update project dependencies.

### Usage

```bash
yx update
yx update <package_name>
```

### Arguments

| Argument     | Description                              |
| ------------ | ---------------------------------------- |
| package_name | Specific dependency to update (optional) |

### Description

- Without arguments: update all dependencies
- With arguments: update only the specified dependency

### Example

```bash
# Update all dependencies
yx update

# Sample output:
# 📦 Updating dependencies...
1.0.#   http (0 → 1.1.0)
# ✅ Updated 1 dependency, lock file updated

# Update a single dependency
yx update http
```

---

## yx list

List all dependencies of the project.

### Usage

```bash
yx list
```

### Description

Displays all runtime and development dependencies, along with their versions and sources.

### Example

```bash
# List dependencies
yx list

# Sample output:
# 📦 Project dependencies
#
# Runtime dependencies:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# Development dependencies:
#   test-utils  0.5.0    registry
```
