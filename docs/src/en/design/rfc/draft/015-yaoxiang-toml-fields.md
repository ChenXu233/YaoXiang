---
title: 'RFC-015: yaoxiang.toml Configuration Fields Research'
---

# RFC-015: yaoxiang.toml Configuration Fields Research

> **Status**: Draft
> **Author**: ChenXu
> **Created Date**: 2026-02-12
> **Last Updated**: 2026-02-12

> **Prerequisite RFC**: [RFC-014: Package Manager Design](014-package-manager.md)

## Summary

In-depth research on `yaoxiang.toml` configuration file field design, comparing mainstream language ecosystem configuration specifications, formulating a configuration field set that matches YaoXiang language characteristics.

### Layered Configuration Architecture

```
Priority (high → low):
1. Project-level yaoxiang.toml
2. User-level ~/.config/yaoxiang/config.toml
3. Compiler defaults
```

### i18n Support

```toml
[i18n]
lang = "en"        # en / zh / zh-x-miao
fallback = "en"    # Fallback language when translation missing
```

## 1. Field Research

### 1.1 Cargo.toml Analysis

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
authors = ["Alice <alice@example.com>"]
description = "A short description"
documentation = "https://docs.rs/my-crate"
homepage = "https://github.com/alice/my-crate"
repository = "https://github.com/alice/my-crate"
license = "MIT OR Apache-2.0"
license-file = "LICENSE"
readme = "README.md"
keywords = ["cli", "parsing"]
categories = ["command-line-utilities", "parser-implementations"]

[features]
default = ["std"]
std = []
docsrs = []

[dependencies]
foo = "1.0"

[dev-dependencies]
test-utils = "1.0"

[build-dependencies]
cc = "1.0"

[workspace]
members = ["crate-a", "crate-b"]
exclude = ["temp/"]

[profile.release]
opt-level = 3
lto = true
```

**Key Fields**:
- `edition`: Rust version feature set
- `features`: Conditional compilation features
- `profile.*`: Build optimization configuration

### 1.2 package.json Analysis

```json
{
  "name": "my-project",
  "version": "1.0.0",
  "description": "A short description",
  "main": "dist/index.js",
  "module": "dist/index.mjs",
  "types": "dist/index.d.ts",
  "exports": {
    ".": {
      "import": "./dist/index.mjs",
      "require": "./dist/index.js"
    },
    "./foo": "./dist/foo.js"
  },
  "scripts": {
    "build": "tsc",
    "test": "jest"
  },
  "keywords": ["cli", "parser"],
  "author": "Alice <alice@example.com>",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/alice/my-project.git"
  },
  "bugs": {
    "url": "https://github.com/alice/my-project/issues"
  },
  "homepage": "https://github.com/alice/my-project#readme",
  "engines": {
    "node": ">=16.0.0"
  },
  "os": ["darwin", "linux"],
  "cpu": ["x64", "arm64"],
  "dependencies": {
    "foo": "^1.0.0"
  },
  "devDependencies": {
    "test-utils": "^1.0.0"
  },
  "peerDependencies": {
    "react": ">=17.0.0"
  },
  "bundledDependencies": [],
  "optionalDependencies": {},
  "publishConfig": {
    "registry": "https://npm.pkg.github.com/"
  }
}
```

**Key Fields**:
- `exports`: Conditional exports (ESM/CommonJS)
- `engines`: Runtime version requirements
- `os`/`cpu`: Platform restrictions
- `peerDependencies`: Peer dependencies
- `publishConfig`: Publish configuration

### 1.3 pyproject.toml Analysis

```toml
[project]
name = "my-package"
version = "0.1.0"
description = "A short description"
authors = [
    { name = "Alice", email = "alice@example.com" }
]
license = { text = "MIT" }
readme = "README.md"
requires-python = ">=3.8"
keywords = ["cli", "parser"]
classifiers = [
    "Development Status :: 4 - Beta",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: MIT License",
    "Programming Language :: Python :: 3"
]
dependencies = [
    "requests>=2.25.0",
]

[project.optional-dependencies]
dev = [
    "pytest>=7.0.0",
    "black>=22.0.0",
]

[project.scripts]
my-script = "my_package.cli:main"

[tool.black]
line-length = 88
target-version = ['py38']

[tool.pytest.ini_options]
testpaths = ["tests"]
```

**Key Fields**:
- `classifiers`: PyPI classification tags
- `requires-python`: Python version requirement
- `project.scripts`: Entry script definition
- `[tool.*]`: Tool configuration sections

### 1.4 go.mod Analysis

```mod
module github.com/alice/my-project

go 1.21

require (
    github.com/foo/bar v1.2.3
    github.com/baz/qux v0.5.0
)

require github.com/example/pkg v1.0.0 // indirect

replace github.com/foo/bar => ./local-bar

exclude github.com/foo/bad v1.0.0
```

**Key Fields**:
- `replace`: Local path replacement
- `exclude`: Exclude specific versions
- `// indirect`: Indirect dependency marker

### 1.5 deno.json Analysis

```json
{
  "name": "@alice/my-project",
  "version": "0.1.0",
  "exports": "./mod.ts",
  "imports": {
    "foo": "https://deno.land/x/foo@v1.0.0/mod.ts"
  },
  "tasks": {
    "build": "deno run --allow-all build.ts",
    "test": "deno test"
  },
  "fmt": {
    "lineWidth": 80,
    "indentWidth": 2,
    "useTabs": false,
    "singleQuote": true
  },
  "lint": {
    "rules": {
      "tags": ["recommended"]
    }
  },
  "test": {
    "files": {
      "include": ["test/"]
    }
  },
  "nodeModulesDir": "none",
  "compilerOptions": {
    "strict": true,
    "jsx": "react-jsx"
  }
}
```

**Key Fields**:
- `imports`: URL import map
- `tasks`: Custom tasks
- `fmt`/`lint`/`test`: Tool configuration
- `nodeModulesDir`: Node modules directory control

### 1.6 Comparison Summary

| Field | Cargo | package.json | pyproject.toml | go.mod | deno.json | Recommendation |
|-------|-------|--------------|----------------|--------|-----------|----------------|
| name | ✅ | ✅ | ✅ | ✅ (module) | ✅ | ✅ Required |
| version | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ Required |
| description | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ Recommended |
| authors | ✅ | ✅ (author) | ✅ | ❌ | ❌ | ✅ Recommended |
| license | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ Recommended |
| repository | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ Recommended |
| homepage | ✅ | ✅ | ❌ | ❌ | ❌ | ✅ Recommended |
| keywords | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ Optional |
| readme | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ Optional |
| edition | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ YaoXiang version |
| features | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ Optional |
| exports | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ Optional |
| dependencies | ✅ | ✅ | ✅ | ✅ | (imports) | ✅ Required |
| dev-dependencies | ✅ | ✅ | ✅ | ❌ (// indirect) | ❌ | ✅ Optional |
| workspace | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ Phase 4 |
| engines | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ Optional |

## 2. Field Design Recommendations

### 2.0 Layered Configuration Architecture

yaoxiang.toml supports **user-level** and **project-level** two configuration layers:

```
┌─────────────────────────────────────────────────────────┐
│              Configuration Priority (high → low)        │
├─────────────────────────────────────────────────────────┤
│  1. Project-level yaoxiang.toml                        │
│  2. User-level ~/.config/yaoxiang/config.toml          │
│  3. Compiler defaults                                  │
└─────────────────────────────────────────────────────────┘
```

**Configuration fallback rules**:
- Project-level configuration **overrides** user-level
- For options not configured at project-level, use user-level
- For options not configured at user-level, use compiler defaults

#### 2.0.1 User-level Configuration Example

```toml
# ~/.config/yaoxiang/config.toml

[i18n]
lang = "en"                    # Default language

[repl]
history-size = 1000            # REPL history entries
history-file = "~/.yaoxiang_history"
prompt = "yx> "                # Prompt
colors = true                  # Syntax highlighting
auto-imports = ["std"]        # Modules auto-imported on REPL start

[fmt]
line-width = 120
indent-width = 4
use-tabs = false
single-quote = true

[lint]
rules = ["recommended"]
strict = false

[build]
output = "dist/"               # Build output directory

[install]
dir = "~/.local/share/yaoxiang"  # Global install directory
```

#### 2.0.2 Project-level Configuration Example

```toml
# yaoxiang.toml

[package]
name = "my-project"
version = "0.1.0"

[yaoxiang]
version = ">=0.1.0, <1.0.0"

[i18n]
# Inherit user-level config, use zh if not configured
lang = "en"                    # Override user-level setting

[repl]
# Use user-level history-size, but override welcome-message
welcome-message = "Welcome to My Project!"

[dependencies]
std = "0.1.0"
```

#### 2.0.3 Field Level Restrictions

| Field | User-level | Project-level | Description |
|-------|------------|---------------|-------------|
| `[package].*` | ❌ | ✅ | Package metadata only at project level |
| `[yaoxiang]` | ❌ | ✅ | Language version only at project level |
| `[dependencies]` | ❌ | ✅ | Dependency declaration only at project level |
| `[dev-dependencies]` | ❌ | ✅ | Dev dependencies only at project level |
| `[bin]` | ❌ | ✅ | Binary config only at project level |
| `[lib]` | ❌ | ✅ | Library config only at project level |
| `[build]` | ✅ | ✅ | Build config (project overrides user) |
| `[install]` | ✅ | ❌ | Global install config only at user level |
| `[repl]` | ✅ | ✅ | REPL config (project overrides user) |
| `[i18n]` | ✅ | ✅ | i18n config (project overrides user) |
| `[fmt]` | ✅ | ✅ | Format config (project overrides user) |
| `[lint]` | ✅ | ✅ | Lint config (project overrides user) |
| `[test]` | ✅ | ✅ | Test config (project overrides user) |
| `[tasks]` | ✅ | ✅ | Tasks config (project overrides user) |

#### 2.0.4 【Innovation】i18n Internationalization Configuration

```toml
# Project-level or user-level

[i18n]
lang = "en"                    # Current language: en / zh / zh-x-miao
fallback = "en"               # Fallback language when translation missing
```

**Usage**:
```bash
# Command line override
yaoxiang run main.yx --lang zh
yaoxiang -L zh run main.yx

# Environment variable override
export YAOXIANG_LANG=zh

# Configuration file (priority: command line > env var > config)
```

### 2.1 Required Fields

```toml
[package]
name = "my-package"       # Package name (lowercase, kebab-case recommended)
version = "0.1.0"        # Semantic version
```

### 2.2 Recommended Fields

```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"
authors = ["Alice <alice@example.com>"]
license = "MIT"
repository = "https://github.com/alice/my-project"
homepage = "https://alice.github.io/my-project"
documentation = "https://docs.alice.github.io/my-project"
keywords = ["cli", "parser"]
readme = "README.md"

[yaoxiang]
# edition uses SemVer, supports version range
version = ">=0.1.0, <1.0.0"    # Requires language version >= 0.1.0 and < 1.0.0
# Or use wildcard
version = "0.x"                # Requires 0.x version
```

### 2.3 Dependency Declaration

```toml
[dependencies]
# Simple version constraints
foo = "1.2.3"
bar = "^1.0.0"
baz = "~1.2.0"

# Git dependencies
qux = { git = "https://github.com/user/qux", version = "0.5.0" }
qux = { git = "https://github.com/user/qux", tag = "v0.5.0" }
qux = { git = "https://github.com/user/qux", branch = "main" }

# Local paths
local = { path = "./local-module" }

# Registry dependencies (Phase 3)
registry-pkg = "1.0.0"

[dev-dependencies]
test-utils = "0.1.0"

[build-dependencies]
build-script = { path = "./build" }
```

### 2.4 Build Configuration

```toml
[build]
script = "build.yx"           # Build script (optional)
output = "dist/"              # Build output directory

[profile.release]
optimize = true                # Optimization level
lto = true                     # Link-time optimization
debug = false                  # Debug information
```

### 2.5 Tool Configuration

```toml
[fmt]
line-width = 120
indent-width = 4
use-tabs = false
single-quote = true

[lint]
rules = ["recommended"]
strict = false

[test]
files = ["tests/"]
report = "junit"
```

### 2.5 Built-in Tasks Configuration

Reference package.json scripts design, integrate common commands into yaoxiang.toml:

```toml
[tasks]
# Built-in tasks, can be overridden
build = "yaoxiang build"
test = "yaoxiang test"
bench = "yaoxiang bench"
clean = "yaoxiang clean"

# Custom tasks
lint = "yaoxiang fmt && yaoxiang check"
release = "yaoxiang build --release"
deploy = "yaoxiang build && cp dist/* /srv"
docs = "yaoxiang doc"

# Task dependencies
[dependencies.benchmark]
run = "yaoxiang bench"
depends = ["build"]

# Conditional tasks
[dependencies.docs]
run = "yaoxiang doc"
only = ["features:docs"]
```

**Run tasks**:
```bash
yaoxiang task build    # Run build task
yaoxiang task lint     # Run lint task
yaoxiang task release  # Run release task
```

### 2.6 REPL Integration Configuration

YaoXiang CLI is unified (compiler + package manager + REPL), supports configuring REPL behavior in yaoxiang.toml:

```toml
[repl]
history-size = 1000       # History entries (default 1000)
history-file = "~/.yaoxiang_history"  # History file path
auto-imports = ["std"]   # Modules auto-imported on REPL start
welcome-message = "Welcome to YaoXiang v0.1.0!"  # Welcome message
prompt = "yx> "          # Prompt
multi-line = true        # Support multi-line input
colors = true            # Syntax highlighting
editor = "vim"           # External editor

[run]
main = "src/main.yx"     # Project default entry file
args = ["--quiet"]       # Default run arguments

[build]
# Auto execute on build
pre-build = "yaoxiang fmt"   # Format before build
post-build = "echo Build done!"  # Prompt after build

# Build artifacts
output = "dist/"         # Output directory
```

### 2.7 Entry and Exports

```toml
[package]
name = "my-library"
version = "0.1.0"

# Library entry
[lib]
path = "src/lib.yx"            # Library main file

# Executable programs
[[bin]]
name = "my-cli"
path = "src/cli.yx"

[[bin]]
name = "my-tool"
path = "src/tool.yx"

# Conditional exports
[exports]
"." = "src/lib.yx"
"./foo" = "src/foo.yx"
"./bar" = { path = "src/bar.yx", yaoxiang = ">=0.2.0" }
```

### 2.8 Platform Constraints

```toml
[platform]
os = ["windows", "linux", "macos"]
cpu = ["x86_64", "aarch64"]
yaoxiang = ">=0.1.0"

[target.x86_64-unknown-linux-gnu]
# Platform-specific configuration
optimize = true

[target.x86_64-pc-windows-msvc]
# Windows-specific configuration
```

### 2.9 Publish Configuration

```toml
[publish]
registry = "https://packages.yaoxiang.dev"
access = "public"              # public | private
```

## 3. Complete Examples

### 3.1 Application

```toml
# yaoxiang.toml
[package]
name = "my-cli-tool"
version = "0.1.0"
description = "A command-line tool for processing files"
authors = ["Alice <alice@example.com>"]
license = "MIT"
repository = "https://github.com/alice/my-cli-tool"
homepage = "https://github.com/alice/my-cli-tool"
readme = "README.md"
keywords = ["cli", "tool", "file-processing"]

[yaoxiang]
# edition uses SemVer
version = ">=0.1.0, <1.0.0"

[dependencies]
# Standard library
std = "0.1.0"

# Third-party dependencies
toml = { git = "https://github.com/yaoxiang/toml", version = "^1.0.0" }

[dev-dependencies]
test-utils = { path = "./crates/test-utils" }

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[build]
script = "scripts/build.yx"

[profile.release]
optimize = true

# Built-in tasks configuration
[tasks]
build = "yaoxiang build"
test = "yaoxiang test"
lint = "yaoxiang fmt && yaoxiang check"
release = "yaoxiang build --release"

# REPL configuration
[repl]
history-size = 1000
auto-imports = ["std"]
welcome-message = "Welcome to My CLI Tool!"

[run]
main = "src/cli.yx"
```

### 3.2 Library

```toml
# yaoxiang.toml
[package]
name = "my-parser-lib"
version = "0.1.0"
description = "A JSON parser library for YaoXiang"
authors = ["Alice <alice@example.com>"]
license = "MIT"
repository = "https://github.com/alice/my-parser-lib"

[yaoxiang]
# edition uses SemVer
version = ">=0.1.0, <1.0.0"

[dependencies]
std = "0.1.0"

[dev-dependencies]
json-fuzz = { path = "./crates/json-fuzz" }

[lib]
path = "src/lib.yx"

[exports]
"." = "src/lib.yx"
"./error" = "src/error.yx"

# REPL configuration
[repl]
auto-imports = ["std", "./error"]
```

### 3.3 Workspace

```toml
# yaoxiang.toml (workspace root)
[workspace]
members = [
    "crates/cli",
    "crates/lib",
    "crates/utils",
]
exclude = ["temp/", "experiments/"]

[workspace.dependencies]
version = "1.0"
shared-utils = { path = "crates/utils" }
```

## 4. Field Priority

| Phase | Field | Priority |
|-------|-------|----------|
| **P0** | `package.name`, `package.version` | Required |
| **P0** | `dependencies` | Required |
| **P1** | `description`, `authors`, `license` | Recommended |
| **P1** | `yaoxiang.edition` | Recommended |
| **P1** | `bin`, `lib` | Common |
| **P2** | `dev-dependencies`, `build-dependencies` | Optional |
| **P2** | `profile.*` | Optional |
| **P3** | `features`, `platform`, `target.*` | Advanced |
| **P3** | `workspace` | Phase 4 |

## 5. Open Questions

- [ ] Whether to support `bundled-dependencies`?
- [ ] Whether to support `peer-dependencies`?
- [ ] What is the syntax design for `features` conditional compilation?
- [x] Layered configuration (user-level + project-level) confirmed
- [x] i18n configuration confirmed

## Appendix A: Field Index

| Field | Location | Type | Required | Description |
|-------|----------|------|----------|-------------|
| `name` | `[package]` | String | ✅ | Package name |
| `version` | `[package]` | Version | ✅ | Version number |
| `description` | `[package]` | String | ❌ | Description |
| `authors` | `[package]` | [String] | ❌ | Author list |
| `license` | `[package]` | String | ❌ | License |
| `repository` | `[package]` | Url | ❌ | Code repository |
| `homepage` | `[package]` | Url | ❌ | Homepage |
| `documentation` | `[package]` | Url | ❌ | Documentation URL |
| `keywords` | `[package]` | [String] | ❌ | Keywords |
| `readme` | `[package]` | Path | ❌ | README file |
| `edition` | `[yaoxiang]` | String | ❌ | Language version |
| `dependencies` | Top-level | Table | ✅ | Dependency declaration |
| `dev-dependencies` | Top-level | Table | ❌ | Dev dependencies |
| `build-dependencies` | Top-level | Table | ❌ | Build dependencies |
| `lib` | `[package]` | Table | ❌ | Library configuration |
| `bin` | `[package]` | [Table] | ❌ | Executable programs |
| `build` | Top-level | Table | ❌ | Build configuration |
| `profile.*` | Top-level | Table | ❌ | Build optimization |
| `fmt` | Top-level | Table | ❌ | Format configuration |
| `lint` | Top-level | Table | ❌ | Lint configuration |
| `test` | Top-level | Table | ❌ | Test configuration |
| `exports` | `[package]` | Table | ❌ | Export map |
| `platform` | Top-level | Table | ❌ | Platform constraints |
| `target.*` | Top-level | Table | ❌ | Platform-specific config |
| `publish` | Top-level | Table | ❌ | Publish configuration |
| `workspace` | Top-level | Table | ❌ | Workspace |
| `i18n` | Top-level | Table | ❌ | 【Innovation】i18n config |
| `tasks` | Top-level | Table | ❌ | 【Innovation】Built-in tasks config |
| `repl` | Top-level | Table | ❌ | 【Innovation】REPL config |
| `install` | Top-level | Table | ❌ | Global install config (user only) |

## References

- [Cargo Manifest](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [npm package.json](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)
- [PEP 621: pyproject.toml](https://peps.python.org/pep-0621/)
- [go.mod reference](https://go.dev/ref/mod#go-mod)
- [deno.json](https://deno.com/manual@v1.28.3/getting-started/configuration_file)

---

## Appendix: Field Level Quick Reference

### Project-level Only (Cannot be in user-level)

| Field | Description |
|-------|-------------|
| `[package].*` | Package name, version, authors and other metadata |
| `[yaoxiang]` | Language version constraints |
| `[dependencies]` | Dependency declaration |
| `[dev-dependencies]` | Dev dependencies |
| `[bin]` | Executable program configuration |
| `[lib]` | Library configuration |
| `[build]` | Build script |

### User-level Only (Cannot be in project-level)

| Field | Description |
|-------|-------------|
| `[install]` | Global install directory etc. |
| `[install].dir` | Global install path |

### Both Allowed (Project overrides user)

| Field | Description |
|-------|-------------|
| `[i18n]` | Internationalization configuration |
| `[repl]` | REPL configuration |
| `[fmt]` | Format configuration |
| `[lint]` | Lint configuration |
| `[test]` | Test configuration |
| `[build].output` | Build output directory |
| `[tasks]` | Custom tasks |
| `[profile.*]` | Build optimization configuration |
