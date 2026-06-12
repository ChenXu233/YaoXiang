---
title: "RFC-014: Package Management System Design"
status: "Accepted"
author: "Chenxu"
created: "2026-02-12"
updated: "2026-06-11"
group: "rfc-014"  # This RFC is the main document for the package management system, sub-RFCs: 014a/014b/014c
---

# RFC-014: Package Management System Design (Main Document)

> **Sub-RFCs:**
> - [RFC-014a: Registry Protocol Specification](../draft/014a-registry-protocol.md)
> - [RFC-014b: Build System and Binary Distribution](../draft/014b-build-system.md)
> - [RFC-014c: Workspace Support](../draft/014c-workspace.md)

## Summary

Design a package management system for the YaoXiang language, supporting semantic versioning, local and GitHub dependencies, a unified import syntax, `yaoxiang.toml` configuration file, and `yaoxiang.lock` lock file.

## Motivation

### Why is this feature/change needed?

Package management is the infrastructure foundation of a modern programming language ecosystem. Currently, the YaoXiang language lacks:
- A dependency declaration mechanism
- Version management capabilities
- A standard distribution channel

### Current Problems

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
│           Resolution Engine                  │ ← Dependency resolution
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← Extensible sources
├──────────┬──────────┬──────────┬────────────┤
│  Local   │   Git    │ Registry │   GitHub   │
│  (Local) │  (VCS)   │  (Open)  │ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**Extension Mechanism**: Adding a new Source type only requires implementing a trait, with no need to modify the resolution engine.

### Examples

```bash
# 1. Create a project
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
license = "MIT"
authors = ["Your Name <you@example.com>"]
repository = "https://github.com/you/my-package"
keywords = ["cli", "utility"]

[dependencies]
foo = "1.2.3"           # Exact version
bar = "^1.0.0"          # Compatible version
baz = "~1.2.0"          # Patch version
qux = { git = "...", version = "0.5.0" }
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"

[build]
strategy = "none"       # none | cargo | cmake | custom

[binaries]
"linux-x86_64" = { url = "...", sha256 = "..." }

[workspace.members]     # Workspace root only
core = "packages/core/yaoxiang.toml"
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

Lookup order:
1. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/)
2. ./src/foo/bar/baz.yx                     (local modules)
3. ~/.yaoxiang/cache/foo/<ver>/src/foo/bar/baz.yx  (global cache)
4. $YXPATH/foo/bar/baz.yx                   (global path, reserved)
5. $YXLIB/std/foo/bar/baz.yx                (standard library)
```

### Core Data Structures

```rust
// Dependency source (extensible)
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> },
    GitHub { owner: String, repo: String, ref_: GitRef },  // Native GitHub
}

enum GitRef {
    Tag(String),
    Branch(String),
    Rev(String),
    DefaultBranch,
}

// Dependency declaration
enum DependencySpec {
    Version(VersionConstraint),
    Git { url: Url, version: Option<VersionConstraint> },
    Local { path: PathBuf },
    Workspace { member: String },  // Workspace member reference
}

// Resolved dependency
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
    checksum: Option<String>,  // SHA-256
}

// Build strategy
enum BuildStrategy {
    None,          // Pure .yx package
    Cargo,         // Invoke cargo build
    Cmake,         # Invoke cmake
    Custom,        // Execute build.yx script
    Precompiled,   // Use precompiled artifacts directly
}
```

### CLI Command Design

Adopting a unified solution, integrating the compiler, package manager, and REPL into a single CLI tool:

#### Single-File Mode vs Project Mode

| Command | Single-File | Project Mode | Description |
|------|--------|---------|------|
| `yaoxiang run <file>` | ✅ | ✅ | Run file/project entry |
| `yaoxiang build` | ❌ | ✅ | Build project |
| `yaoxiang build <file>` | ✅ | ✅ | Build single file |
| `yaoxiang init <name>` | ❌ | ✅ | Create project |
| `yaoxiang add <dep>` | ❌ | ✅ | Add dependency |
| `yaoxiang update` | ❌ | ✅ | Update dependencies |
| `yaoxiang fmt` | ✅ | ✅ | Format code |
| `yaoxiang check` | ✅ | ✅ | Type check |
| `yaoxiang` (no args) | ✅ | ✅ | Enter REPL directly |

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
| `yaoxiang publish` | Publish package to Registry | `yaoxiang publish` |
| `yaoxiang publish --github` | Publish and create GitHub Release | `yaoxiang publish --github` |
| `yaoxiang yank <pkg>@<ver>` | Delete published version (irreversible) | `yaoxiang yank foo@1.2.3` |
| `yaoxiang login --registry <url>` | Registry authentication | `yaoxiang login --registry https://reg.example.com` |
| `yaoxiang login --github` | GitHub authentication | `yaoxiang login --github` |
| `yaoxiang logout --registry <url>` | Logout | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean` | Clean global cache | `yaoxiang cache clean` |
| `yaoxiang workspace <cmd>` | Workspace operations | `yaoxiang workspace list` |

#### Command Constraints

```bash
# Single-file mode: yaoxiang.toml not required
yaoxiang run hello.yx   # ✅ Works normally
yaoxiang add foo        # ❌ Error: not a project directory

# Project mode: yaoxiang.toml required
cd my-project
yaoxiang run main.yx    # ✅ Run entry file
yaoxiang build          # ✅ Build project
yaoxiang add foo        # ✅ Add dependency
```

### Backward Compatibility

- ✅ Existing `use` syntax is fully preserved
- ✅ Existing module resolution logic remains unchanged
- ✅ New `.yaoxiang/vendor` directory does not affect existing projects

### Global Cache

All downloaded dependencies are cached to `~/.yaoxiang/cache/`, and the project vendor directory is copied from the cache.

```
~/.yaoxiang/
├── cache/
│   ├── registry/
│   │   └── foo-1.2.3/
│   ├── git/
│   │   └── github.com-user-bar-abc123/
│   └── binaries/
│       └── foo-1.2.3-linux-x86_64.tar.gz
├── credentials.toml
└── config.toml
```

```toml
# ~/.yaoxiang/config.toml
[cache]
dir = "~/.yaoxiang/cache"
max_size = "2GB"
ttl = "30d"
```

Cache invalidation rules:
- Registry packages: version numbers are immutable, never expire
- Git dependencies: cached by tag/rev; if tag is unchanged, no expiration
- `yaoxiang cache clean` for manual cleanup

### Authentication

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

- Environment variables take priority: `$YX_GITHUB_TOKEN`, `$YX_REGISTRY_TOKEN`
- Tokens are never written to `yaoxiang.toml` or `yaoxiang.lock`
- File permission 600

### Yank Semantics

`yaoxiang yank foo@1.2.3` performs **deletion + version number lockdown**:

- The package is completely deleted and cannot be recovered
- The version number is permanently occupied and cannot be republished
- Projects whose existing lockfile references this version will error and need to upgrade
- **Security purpose**: prevent npm-style supply chain attacks (attackers reclaiming deleted version numbers to inject malicious code)

### Registry Protocol

See [RFC-014a: Registry Protocol Specification](../draft/014a-registry-protocol.md) for details.

Core design: open protocol + adapter layer. The official Registry is primary, with GitHub Release/main branch as secondary support, and custom Registry support.

### Build System

See [RFC-014b: Build System and Binary Distribution](../draft/014b-build-system.md) for details.

Core design: declarative `[build]` configuration, precompiled-first/source-fallback, supporting cargo/cmake/custom strategies.

### Workspace

See [RFC-014c: Workspace Support](../draft/014c-workspace.md) for details.

Core design: dictionary-form members declaration, shared lockfile, path dependencies, Cargo workspace integration.

## Trade-offs

### Advantages

- Unified import syntax, users don't need to care about dependency sources
- Deterministic builds, lock file ensures build consistency
- Offline support, offline development possible after downloading locally
- Source trait facilitates future extension

### Disadvantages

- Additional storage space required (.yaoxiang/vendor directory)
- Version conflicts need to be resolved manually by the user

## Alternatives

| Approach | Why Not Chosen |
|------|-----------|
| Real-time GitHub access | Difficult to ensure security and cache reuse |
| Global cache only ($HOME/.yaoxiang) | Poor isolation, complex version conflicts |
| Registry-only support | GitHub is the current mainstream code hosting platform |

## Implementation Strategy

### Phased Breakdown

| Phase | Content | Status |
|------|------|------|
| **Phase 1** | toml parsing, local dependencies, lock generation, basic algorithms | ✅ Completed |
| **Phase 2** | GitHub support, .yaoxiang/vendor management, download tools | ✅ Completed |
| **Phase 3** | Global cache, semver crate replacement, CLI refinement | Pending |
| **Phase 3.5** | Source trait to async, async-trait integration | Pending |
| **Phase 4** | Registry protocol, publish, auth (RFC-014a) | Pending |
| **Phase 5** | Build system, precompiled binaries (RFC-014b) | Pending |
| **Phase 6** | Workspace support (RFC-014c) | Pending |

### Dependencies

- No prerequisites
- Needs integration with `ModuleGraph` (`middle/passes/module/`)

### Risks

| Risk | Mitigation |
|------|----------|
| Dependency resolution algorithm is complex | Implement simple version first, add conflict detection later |
| Git download instability | Retry and cache mechanisms |
| Performance issues | Lazy loading, incremental resolution |

## Open Questions

- [x] `dev-dependencies` conditional compilation syntax? → Handled uniformly by RFC-014b build system
- [x] Integrity verification algorithm (SHA-256 / BLAKE3)? → SHA-256
- [ ] `excludes` to exclude specific files from download?
- [ ] Package naming conventions (does it support namespace, e.g., `@org/pkg`)?
- [ ] Registry API versioning strategy?

---

## Dependencies (Cargo.toml additions required)

| Purpose | crate | Description |
|------|-------|------|
| Semantic versioning | `semver` | Replace hand-written parser |
| HTTP client | `reqwest` | Registry communication |
| SHA-256 | `sha2` | Integrity verification |
| Compression | `flate2` + `tar` | Package format processing |

---

## References

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)