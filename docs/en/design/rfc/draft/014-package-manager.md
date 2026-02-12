---
title: 'RFC-014: Package Manager Design'
---

# RFC-014: Package Manager Design

> **Status**: Draft
> **Author**: ChenXu
> **Created Date**: 2026-02-12
> **Last Updated**: 2026-02-12

## Summary

Design a package management system for YaoXiang language, supporting semantic versioning (SemVer), local modules and GitHub dependencies (downloaded to vendor/ directory), unified `use <name>` import syntax, `yaoxiang.toml` configuration file and `yaoxiang.lock` lockfile.

## Motivation

### Why this feature/change is needed?

In modern programming language ecosystems, package management is a core part of infrastructure. YaoXiang language currently has the following problems:

1. **Missing dependency management**: Users cannot declare project dependencies, can only manually copy code
2. **Version chaos**: Cannot manage dependency versions, prone to compatibility issues
3. **Difficult ecosystem building**: Third-party libraries have no standard release and distribution mechanism

### Current Problems

```
my-project/
├── src/
│   └── main.yx          # Code needs functions from other modules
├── lib/                 # Modules copied manually
│   ├── foo.yx
│   └── bar.yx
└── ???                 # No standard dependency declaration method
```

Expected user workflow:
```bash
# Declare dependencies
[dependencies]
foo = "1.2.3"

# One-click install
yaoxiang add foo

# Use directly
use foo;    # No need to care about source
```

## Proposal

### Core Design

Use layered architecture, abstract "dependency source" as `Source` trait:

```
┌─────────────────────────────────────────┐
│           Resolution Engine              │
│    (Dependency Resolution, Version      │
│     Constraint Matching, Conflict Detection)
└─────────────┬───────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│              Source Trait                │
├─────────────┬─────────────┬───────────────┤
│   Local     │    Git      │   Registry   │
│   Source    │   Source   │    (Future)  │
│  (Local)   │  (GitHub)   │ (crates.io)  │
└─────────────┴─────────────┴───────────────┘
```

### Project Structure

```
my-project/
├── yaoxiang.toml        # Package configuration
├── yaoxiang.lock        # Lockfile (auto-generated)
├── src/                 # Source code
│   └── main.yx
└── vendor/              # Downloaded dependencies
    ├── foo-1.2.3/
    │   ├── yaoxiang.toml
    │   └── src/
    └── bar-0.5.0/
```

### User Workflow

```bash
# 1. Create project
yaoxiang init my-project

# 2. Edit yaoxiang.toml to add dependencies
[dependencies]
foo = { git = "https://github.com/user/foo", version = "1.2.3" }

# 3. Download dependencies
yaoxiang add foo   # or yaoxiang install

# 4. Use in code (unified as local path)
use foo;
use foo.bar;
```

### yaoxiang.toml Configuration Specification

```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"
authors = ["Author Name <email@example.com>"]

[dependencies]
# Semantic version constraints
foo = "1.2.3"           # Exact version
bar = "^1.0.0"          # Compatible version (>=1.0.0, <2.0.0)
baz = "~1.2.0"          # Patch version (>=1.2.0, <1.3.0)

# GitHub dependency
qux = { git = "https://github.com/user/qux", version = "0.5.0" }

# Local path dependency
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"

[build]
script = "build.yx"
```

### Version Constraint Syntax

| Constraint | Meaning | Example |
|------------|---------|---------|
| `1.2.3` | Exact version | `1.2.3` |
| `^1.2.3` | Compatible version | `>=1.2.3, <2.0.0` |
| `~1.2.3` | Patch version | `>=1.2.3, <1.3.0` |
| `>=1.0` | Minimum version | `>=1.0.0` |
| `1.x` | Major wildcard | `>=1.0.0, <2.0.0` |
| `*` | Any version | Any |

### yaoxiang.lock Format

```toml
# This file is auto-generated, please do not manually modify
version = 1

[[package]]
name = "foo"
version = "1.2.3"
source = "git"
resolved = "https://github.com/user/foo?tag=v1.2.3"
integrity = "sha256-xxxx"

[[package]]
name = "bar"
version = "0.5.0"
source = "local"
resolved = "./vendor/bar-0.5.0"

[[package]]
name = "my-package"
version = "0.1.0"
dependencies = [
    "foo 1.2.3",
    "bar 0.5.0",
]
```

### Module Resolution Order

```
use foo.bar.baz;

Search order:
1. ./vendor/*/src/foo/bar/baz.yx  (matching version in vendor/)
2. ./src/foo/bar/baz.yx           (local module)
3. $YXPATH/foo/bar/baz.yx         (global path, future)
4. $YXLIB/std/foo/bar/baz.yx      (standard library)
```

## Detailed Design

### Core Data Structures

```rust
// src/package/manifest.rs

/// Package manifest (parsed from yaoxiang.toml)
#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub package: PackageConfig,
    pub dependencies: HashMap<String, DependencySpec>,
    pub dev_dependencies: HashMap<String, DependencySpec>,
    pub build: Option<BuildConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PackageConfig {
    pub name: String,
    pub version: Version,
    pub description: Option<String>,
    pub authors: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum DependencySpec {
    /// Simple version constraint: "1.2.3"
    Version(VersionConstraint),
    /// Git dependency
    Git {
        url: Url,
        version: Option<VersionConstraint>,
        tag: Option<String>,
        branch: Option<String>,
        rev: Option<String>,
    },
    /// Local path
    Local { path: PathBuf },
}
```

```rust
// src/package/version.rs

/// Semantic version
#[derive(Debug, Clone, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
    pub build: Option<String>,
}

/// Version constraint expression
#[derive(Debug, Clone)]
pub enum VersionConstraint {
    Exact(Version),
    Compatible(Version),  // ^1.2.3
    Patch(Version),       // ~1.2.3
    Range { min: Option<Version>, max: Option<Version> },
    Wildcard(MatchLevel),
}

enum MatchLevel { Major, Minor }
```

```rust
// src/package/resolution.rs

/// Resolved dependency
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub name: String,
    pub version: Version,
    pub source: SourceInfo,
    pub integrity: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SourceInfo {
    Local { path: PathBuf },
    Git { url: Url, tag: Option<String>, commit: Option<String> },
    Registry { registry: String, namespace: Option<String> },
}

/// Package identifier: name + version (ensures same package with different versions can coexist)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    pub name: String,
    pub version: Version,
}
```

### Component Architecture

```
src/package/
├── mod.rs              # Entry, exports public API
├── manifest.rs         # yaoxiang.toml parsing
├── version.rs          # SemVer parsing and constraint
├── resolution.rs       # Dependency resolution algorithm
├── lock.rs             # yaoxiang.lock read/write
├── fetch.rs            # Download dependencies to vendor/
├── storage.rs          # Local cache management
└── id.rs               # PackageId definition
```

### CLI Command Design

Use unified approach, integrate compiler, package manager, and REPL into a single CLI tool:

#### Single File Mode vs Project Mode

| Command | Single File | Project Mode | Description |
|---------|-------------|--------------|-------------|
| `yaoxiang run <file>` | ✅ | ✅ | Run file/project entry |
| `yaoxiang build` | ❌ | ✅ | Build project |
| `yaoxiang build <file>` | ✅ | ✅ | Build single file |
| `yaoxiang init <name>` | ❌ | ✅ | Create project |
| `yaoxiang add <dep>` | ❌ | ✅ | Add dependency |
| `yaoxiang update` | ❌ | ✅ | Update dependencies |
| `yaoxiang fmt` | ✅ | ✅ | Format |
| `yaoxiang check` | ✅ | ✅ | Type check |
| `yaoxiang` (no args) | ✅ | ✅ | Enter REPL directly |

#### Command Details

| Command | Function | Example |
|---------|----------|---------|
| `yaoxiang` | Enter REPL directly | `yaoxiang` |
| `yaoxiang run <file>` | Run single file/project | `yaoxiang run main.yx` |
| `yaoxiang init <name>` | Create new project | `yaoxiang init my-app` |
| `yaoxiang build` | Build project | `yaoxiang build` |
| `yaoxiang build <file>` | Build single file | `yaoxiang build foo.yx` |
| `yaoxiang add <dep>` | Add dependency | `yaoxiang add foo` |
| `yaoxiang add -D <dep>` | Add dev dependency | `yaoxiang add -D test` |
| `yaoxiang rm <dep>` | Remove dependency | `yaoxiang rm foo` |
| `yaoxiang update` | Update all dependencies | `yaoxiang update` |
| `yaoxiang update foo` | Update specific dependency | `yaoxiang update foo` |
| `yaoxiang install` | Install all dependencies | `yaoxiang install` |
| `yaoxiang list` | List dependencies | `yaoxiang list` |
| `yaoxiang outdated` | Check outdated dependencies | `yaoxiang outdated` |
| `yaoxiang fmt` | Format code | `yaoxiang fmt` |
| `yaoxiang check` | Type check | `yaoxiang check` |
| `yaoxiang clean` | Clean build artifacts | `yaoxiang clean` |
| `yaoxiang task <name>` | Run custom task | `yaoxiang task lint` |

#### Command Constraint Notes

```bash
# Single file mode: yaoxiang.toml not needed
yaoxiang run hello.yx   # ✅ Works normally
yaoxiang add foo        # ❌ Error: not a project directory

# Project mode: yaoxiang.toml needed
cd my-project
yaoxiang run main.yx    # ✅ Run entry file
yaoxiang build          # ✅ Build project
yaoxiang add foo        # ✅ Add dependency
```

### Backward Compatibility

- **Existing `use` syntax fully preserved**: `use std.io` continues to work
- **Existing module resolution unchanged**: Logic for finding modules in `src/` directory unchanged
- **New vendor/ directory doesn't affect existing projects**

## Trade-offs

### Advantages

- **Unified import syntax**: Users don't need to care about dependency source, unified `use <name>`
- **Deterministic builds**: `yaoxiang.lock` ensures same build result on different machines
- **Offline support**: Can develop offline after downloading
- **Progressive extension**: Source abstracted as trait, easy to add registry support later
- **Unified CLI experience**: Compiler, package manager, REPL combined into one
- **Configurable REPL**: Configure REPL behavior in yaoxiang.toml, works out of the box

### Disadvantages

- **Need to download dependencies**: Compared to real-time GitHub, requires extra storage space
- **Version conflict handling**: Users need to manually resolve incompatible version dependencies

## Alternative Solutions

| Solution | Description | Why Not Chosen |
|----------|-------------|----------------|
| Real-time GitHub access | `use github.com/user/repo` direct pull | Hard to guarantee security and cache reuse |
| Global cache | Dependencies stored in global directory ($HOME/.yaoxiang) | Poor isolation, more complex version conflicts |
| Only semantic versioning | Don't support Git source, only registry | GitHub is current mainstream code hosting platform |

## Implementation Strategy

### Phase Division

**Phase 1: Basic Dependency Management**
- yaoxiang.toml parsing
- Version constraint parsing and matching
- Dependency graph construction and conflict detection
- yaoxiang.lock generation
- Local path dependency support

**Phase 2: GitHub Support**
- Git source implementation
- Download tool
- vendor/ directory management
- Git tag/branch parsing

**Phase 3: Registry Support (Future)**
- Registry Source implementation
- Search function
- Private registry support

**Phase 4: Advanced Features**
- Workspace
- Dependency overrides
- Dependency integrity verification

### Dependencies

- This RFC has no prerequisites
- Needs integration with `ModuleGraph` (`middle/passes/module/`) during implementation

### Risks

| Risk | Mitigation |
|------|------------|
| Complex dependency resolution algorithm | Implement simple version first, add conflict detection later |
| Unstable Git download | Implement retry and caching mechanism |
| Performance issues | Lazy loading, incremental resolution |

## Open Questions

- [ ] Whether to support `dev-dependencies` conditional compilation?
- [ ] What algorithm for dependency `Integrity` verification? (SHA-256 / BLAKE3)
- [ ] Whether to support `excludes` to exclude specific files from download?

## Appendix

### Appendix A: Glossary

| Term | Definition |
|------|------------|
| Manifest | Package manifest file (yaoxiang.toml) |
| Lockfile | Lock file (yaoxiang.lock), records exact versions |
| Vendor | Local dependency storage directory |
| Source | Dependency source (local/Git/registry) |
| Resolution | Dependency resolution process |

### Appendix B: Reference Implementations

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [npm Package Manifest](https://docs.npmjs.com/cli/v9/configuring-npm/package-json)
- [Go Modules](https://go.dev/ref/mod)

---

## References

- [RFC-011: Generic Type System Design](011-generic-type-system.md) - Type system foundation
- [Yarn Berry: Plug'n'Play](https://yarnpkg.com/features/pnp) - Alternative dependency management solution
- [Python PEP 440: Version Identification](https://peps.python.org/pep-0440/) - Version identification standard
