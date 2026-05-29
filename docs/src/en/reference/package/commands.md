---
title: Command Line Interface
description: Detailed documentation for all package manager commands
---

# Command Line Interface

## yaoxiang init

Initialize a new YaoXiang project.

### Usage

```bash
yaoxiang init <project-name>
```

### Arguments

| Argument | Description |
|----------|-------------|
| project-name | Name of the new project |

### Options

| Option | Description |
|--------|-------------|
| `--help` | Display help information |

### Examples

```bash
# Create a new project
yaoxiang init my-project

# Result:
# ✨ Project created: my-project
#   my-project/yaoxiang.toml
#   my-project/yaoxiang.lock
#   my-project/src/main.yx
#   my-project/.gitignore
```

---

## yaoxiang add

Add a dependency to the project.

### Usage

```bash
yaoxiang add <package-name> [version]
yaoxiang add <package-name> --dev
```

### Arguments

| Argument | Description |
|----------|-------------|
| package-name | Name of the dependency to add |
| version | Version number (optional, defaults to `*`) |

### Options

| Option | Description |
|--------|-------------|
| `--dev`, `-D` | Add as a development dependency |

### Examples

```bash
# Add latest version
yaoxiang add http

# Add specific version
yaoxiang add http 1.0.0

# Add version range
yaoxiang add json ">=2.0.0"

# Add development dependency
yaoxiang add test-utils --dev
yaoxiang add benchmark -D
```

---

## yaoxiang rm

Remove a dependency from the project.

### Usage

```bash
yaoxiang rm <package-name>
yaoxiang rm <package-name> --dev
```

### Arguments

| Argument | Description |
|----------|-------------|
| package-name | Name of the dependency to remove |

### Options

| Option | Description |
|--------|-------------|
| `--dev`, `-D` | Remove development dependency |

### Examples

```bash
# Remove regular dependency
yaoxiang rm http

# Remove development dependency
yaoxiang rm test-utils --dev
```

---

## yaoxiang install

Install project dependencies.

### Usage

```bash
yaoxiang install
```

### Description

- Reads dependency declarations from `yaoxiang.toml`
- Downloads all dependencies to the `vendor` directory
- Generates/updates `yaoxiang.lock` with locked versions
- Detects dependency version conflicts

### Examples

```bash
# Install all dependencies
yaoxiang install

# Example output:
# 📦 Resolving dependencies...
#   http (1.0.0) [installed]
#   json (2.0.0) [cached]
# ✅ Dependencies installed, lock file updated
```

---

## yaoxiang update

Update project dependencies.

### Usage

```bash
yaoxiang update
yaoxiang update <package-name>
```

### Arguments

| Argument | Description |
|----------|-------------|
| package-name | Specific dependency to update (optional) |

### Description

- Without arguments: updates all dependencies
- With arguments: updates only the specified dependency

### Examples

```bash
# Update all dependencies
yaoxiang update

# Example output:
# 📦 Updating dependencies...
#   http (0 → 1.1.0)
# ✅ 1 dependency updated, lock file updated

# Update a single dependency
yaoxiang update http
```

---

## yaoxiang list

List all dependencies of the project.

### Usage

```bash
yaoxiang list
```

### Description

Displays all runtime dependencies and development dependencies, along with their versions and sources.

### Examples

```bash
# List dependencies
yaoxiang list

# Example output:
# 📦 Project Dependencies
#
# Runtime Dependencies:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# Development Dependencies:
#   test-utils  0.5.0    registry
```