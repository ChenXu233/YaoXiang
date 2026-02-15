---
title: Command Line Interface
description: Detailed Description of All Package Manager Commands
---

# Command Line Interface

## yaoxiang init

Initialize a new YaoXiang project.

### Usage

```bash
yaoxiang init <project-name>
```

### Parameters

| Parameter | Description |
|-----------|-------------|
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

Add dependencies to the project.

### Usage

```bash
yaoxiang add <package-name> [version]
yaoxiang add <package-name> --dev
```

### Parameters

| Parameter | Description |
|-----------|-------------|
| package-name | Name of the dependency to add |
| version | Version number (optional, default `*`) |

### Options

| Option | Description |
|--------|-------------|
| `--dev`, `-D` | Add as development dependency |

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

Remove dependencies from the project.

### Usage

```bash
yaoxiang rm <package-name>
yaoxiang rm <package-name> --dev
```

### Parameters

| Parameter | Description |
|-----------|-------------|
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

- Read dependency declarations from `yaoxiang.toml`
- Download all dependencies to `vendor` directory
- Generate/update `yaoxiang.lock` to lock versions
- Detect dependency version conflicts

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

### Parameters

| Parameter | Description |
|-----------|-------------|
| package-name | Specific dependency to update (optional) |

### Description

- Without parameters: Update all dependencies
- With parameters: Only update specified dependency

### Examples

```bash
# Update all dependencies
yaoxiang update

# Example output:
# 📦 Updating dependencies...
#   http (1.0.0 → 1.1.0)
# ✅ Updated 1 dependency, lock file updated

# Update single dependency
yaoxiang update http
```

---

## yaoxiang list

List all dependencies in the project.

### Usage

```bash
yaoxiang list
```

### Description

Display all runtime dependencies and development dependencies, along with their versions and sources.

### Examples

```bash
# List dependencies
yaoxiang list

# Example output:
# 📦 Project dependencies
#
# Runtime dependencies:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# Development dependencies:
#   test-utils  0.5.0    registry
```
