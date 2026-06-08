---
title: "RFC-014: Package Management System Design"
status: "Accepted"
author: "Chen Xu"
created: "2026-02-12"
updated: "2026-02-14"
---

# RFC-014: Package Management System Design

## Summary

Design a package management system for YaoXiang language, supporting semantic versioning, local and GitHub dependencies, unified import syntax, `yaoxiang.toml` configuration file and `yaoxiang.lock` lock file.

## Motivation

### Why is this feature/change needed?

Package management is the infrastructure of modern programming language ecosystems. Currently, YaoXiang language lacks:

- Dependency declaration mechanism
- Version management capability
- Standard distribution channel

### Current Problem

```
my-project/
├── src/
│   └── main.yx          # Code depends on other modules
├── lib/                  # Manually copied modules
│   ├── foo.yx
│   └── bar.yx
└── ???                   # No standard dependency management
```

## Proposal

### Core Design

**Layered Architecture**:
```
┌─────────────────────────────────────────────┐
│           Resolution Engine                  │ ← Dependency Resolution
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                   │ ← Extensible Source
├─────────────┬─────────────┬─────────────────┤
│   Local     │    Git      │   Registry     │
│   (Local)   │  (GitHub)   │   (Reserved)   │
└─────────────┴─────────────┴─────────────────┘
```

**Extension Mechanism**: Adding a new Source type only requires implementing the trait, without modifying the resolution engine.

### Example

```bash
# 1. Create project
yaoxiang init my-project

# 2. Edit yaoxiang.toml to add dependencies
[dependencies]
foo = "^1.0.0"
bar = { git = "https://github.com/user/bar", version = "0.5.0" }

# 3. Install dependencies
yaoxiang add foo

# 4. Use in code
use foo;
use bar.baz;
```

### Project Structure

```
my-project/
├── yaoxiang.toml        # Package configuration
├── yaoxiang.lock        # Lock file (auto-generated)
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/              # Local dependencies
        ├── foo-1.2.3/
        └── bar-0.5.0/
```

## Detailed Design

### Configuration File Format

**yaoxiang.toml**:
```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"

[dependencies]
foo = "1.2.3"           # Exact version
bar = "^1.0.0"          # Compatible version
baz = "~1.2.0"          # Patch version
qux = { git = "...", version = "0.5.0" }
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"
```

**yaoxiang.lock**:
```toml
version = 1

[[package]]
name = "foo"
version = "1.2.3"
source = "git"
resolved = "https://github.com/user/foo?tag=v1.2.3"
integrity = "sha256-xxxx"
```

### Module Resolution Order

```
use foo.bar.baz;

Lookup Order:
1. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/)
2. ./src/foo/bar/baz.yx           (Local modules)
3. $YXPATH/foo/bar/baz.yx         (Global path, reserved)
4. $YXLIB/std/foo/bar/baz.yx      (Standard library)
```

### Core Data Structures

```rust
// Dependency source (extensible)
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> }, // Reserved
}

// Dependency declaration
enum DependencySpec {
    Version(VersionConstraint),
    Git { url: Url, version: Option<VersionConstraint> },
    Local { path: PathBuf },
}

// Resolved dependency
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
}
```

### CLI Command Design

Adopt a unified approach, integrating the compiler, package manager, and REPL into a single CLI tool:

#### Single File Mode vs Project Mode

| Command | Single File | Project Mode | Description |
|------|--------|---------|------|
| `yaoxiang run <file>` | ✅ | ✅ | Run file/project entry point |
| `yaoxiang build` | ❌ | ✅ | Build project |
| `yaoxiang build <file>` | ✅ | ✅ | Build single file |
| `yaoxiang init <name>` | ❌ | ✅ | Create project |
| `yaoxiang add <dep>` | ❌ | ✅ | Add dependency |
| `yaoxiang update` | ❌ | ✅ | Update dependencies |
| `yaoxiang fmt` | ✅ | ✅ | Format code |
| `yaoxiang check` | ✅ | ✅ | Type check |
| `yaoxiang` (no arguments) | ✅ | ✅ | Enter REPL directly |

#### Command Details

| Command | Function | Example |
|------|------|------|
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

#### Command Constraint Description

```bash
# Single file mode: yaoxiang.toml not required
yaoxiang run hello.yx   # ✅ Works normally
yaoxiang add foo        # ❌ Error: not a project directory

# Project mode: yaoxiang.toml required
cd my-project
yaoxiang run main.yx    # ✅ Run entry point
yaoxiang build          # ✅ Build project
yaoxiang add foo        # ✅ Add dependency
```

### Backward Compatibility

- ✅ Existing `use` syntax fully preserved
- ✅ Existing module resolution logic unchanged
- ✅ New .yaoxiang/vendor directory does not affect existing projects

## Tradeoffs

### Advantages

- Unified import syntax, users don't need to care about dependency source
- Deterministic builds, lock file ensures build consistency
- Offline support, can develop offline after downloading locally
- Source trait facilitates future extension

### Disadvantages

- Requires additional storage space (.yaoxiang/vendor directory)
- Version conflicts require manual resolution by users

## Alternatives

| Alternative | Why Not Chosen |
|------|-----------|
| Real-time GitHub access | Security and cache reuse hard to guarantee |
| Global cache ($HOME/.yaoxiang) | Poor isolation, complex version conflicts |
| Registry-only support | GitHub is the current mainstream code hosting platform |

## Implementation Strategy

### Phase Division

| Phase | Content |
|------|------|
| **Phase 1** | toml parsing, local dependencies, lock generation, basic algorithms |
| **Phase 2** | GitHub support, .yaoxiang/vendor management, download tools |
| **Future Extension** | Registry source, workspaces, integrity verification, dependency overrides |

### Dependencies

- No prerequisites
- Needs integration with `ModuleGraph` (`middle/passes/module/`)

### Risks

| Risk | Mitigation |
|------|----------|
| Dependency resolution algorithm complex | Implement simple version first, add conflict detection later |
| Git download unstable | Retry and cache mechanism |
| Performance issues | Lazy loading, incremental resolution |

## Open Questions

- [ ] `dev-dependencies` conditional compilation syntax?
- [ ] Integrity verification algorithm (SHA-256 / BLAKE3)?
- [ ] `excludes` to exclude specific files from download?

---

## References

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)