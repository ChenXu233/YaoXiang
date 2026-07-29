---
title: 'RFC-014: Package Management System Design'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-02-12'
updated: '2026-06-11'
group: 'rfc-014' # This RFC is the umbrella for package management; sub-RFCs: 014a/014b/014c
issue: '#88'
impl: '48%'
impl_status: 'partial'
---

# RFC-014: Package Management System Design (Overview)

> **Sub RFCs:**
>
> - [RFC-014a: Registry Protocol Specification](../draft/014a-registry-protocol.md)
> - [RFC-014b: Build System and Binary Distribution](../draft/014b-build-system.md)
> - [RFC-014c: Workspace Support](../draft/014c-workspace.md)

## Summary

Design a package management system for the YaoXiang language, supporting semantic versioning, local
and GitHub dependencies, unified import syntax, `yaoxiang.toml` configuration files, and
`yaoxiang.lock` lock files.

## Motivation

### Why is this feature/change needed?

Package management is the foundation of modern programming language ecosystems. The YaoXiang
language currently lacks:

- Dependency declaration mechanism
- Version management capabilities
- Standard distribution channels

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

**Extensibility Mechanism**: Adding a new Source type only requires implementing the trait, without
modifying the resolution engine.

### Examples

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

The resolution order depends on the working mode (whether `yaoxiang.toml` exists).

#### Project Mode (with yaoxiang.toml)

```
use foo.bar.baz;

Search order:
0. Embedded binary                          (std/*.yx — compile-time embedded, version-bound, std.* namespace only)
1. ./.yaoxiang/std/foo/bar/baz.yx     (project-level stdlib — global stdlib skipped entirely when present)
2. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/)
3. ./src/foo/bar/baz.yx                       (local modules)
4. ~/.yaoxiang/cache/foo/<ver>/src/foo/bar/baz.yx  (global cache)
5. $YXPATH/foo/bar/baz.yx                     (global path, reserved)
```

**Project Mode Rules**:

- Embedded binary only applies to `std.*` namespace, highest priority
- When project-level stdlib (`.yaoxiang/std/`) exists, global stdlib is completely skipped — ensures
  deterministic builds
- Projects can add stdlib as a dependency via `yaoxiang add std@1.0.1` to lock the version

#### Single-file Mode (without yaoxiang.toml)

```
use foo.bar.baz;

Search order:
0. Embedded binary                          (std/*.yx — compile-time embedded, version-bound)
1. <yaoxiang-install-dir>/yx/<version>/std/foo/bar/baz.yx  (global stdlib)
2. ./src/foo/bar/baz.yx                                       (local modules)
3. $YXPATH/foo/bar/baz.yx                                     (global path, reserved)
```

**Single-file Mode Rules**:

- No project-level dependencies; stdlib loads directly from global path
- Global stdlib path is bound to compiler version: `<install-dir>/yx/<version>/std/`

### Standard Library Installation Directory Structure

#### Global Standard Library

```
<yaoxiang-install-dir>/
├── yx/                          # YaoXiang language directory
│   ├── 1.0.1/                   # Version directory
│   │   ├── std/
│   │   │   ├── test.yx          # Pure YaoXiang stdlib module
│   │   │   ├── math.yx          # Future self-hosted module
│   │   │   └── ...
│   │   └── ...
│   └── 1.1.0/
│       └── std/
│           └── ...
└── bin/
    └── yaoxiang                 # Compiler binary
```

#### Project-level Standard Library

Projects can add stdlib as a dependency via `yaoxiang add std@1.0.1`, stored in `.yaoxiang/std/`:

```
my-project/
├── yaoxiang.toml
├── yaoxiang.lock
├── .yaoxiang/
│   ├── std/                     # Project-level stdlib (global stdlib ineffective when present)
│   │   ├── test.yx
│   │   ├── math.yx
│   │   └── ...
│   └── vendor/                  # Other dependencies
│       └── ...
├── src/
│   └── main.yx
```

**Design Key Points**:

- Embedded binary serves as a compatibility layer: before filesystem stdlib is fully in place,
  provide stdlib modules via embedded binary
- Version directory isolation: `yx/<version>/std/` allows different versions of stdlib to coexist
  without interfering with each other
- Project-level stdlib overrides global stdlib: ensures deterministic builds, unaffected by global
  environment changes
- Without yaoxiang.toml (single-file mode), falls back to global stdlib
- The existence of `.yaoxiang/std/` indicates "project-level stdlib enabled", global stdlib no
  longer participates

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

// Dependency specification
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
    Cmake,         // Invoke cmake
    Custom,        // Execute build.yx script
    Precompiled,   // Use precompiled artifacts directly
}
```

### CLI Command Design

Adopt a unified approach that integrates compiler, package manager, and REPL into a single CLI tool:

#### Single-file Mode vs Project Mode

| Command                   | Single-file | Project mode | Description            |
| ------------------------- | ----------- | ------------ | ---------------------- |
| `yaoxiang run <file>`     | ✅          | ✅           | Run file/project entry |
| `yaoxiang build`          | ❌          | ✅           | Build project          |
| `yaoxiang build <file>`   | ✅          | ✅           | Build single file      |
| `yaoxiang init <name>`    | ❌          | ✅           | Create project         |
| `yaoxiang add <dep>`      | ❌          | ✅           | Add dependency         |
| `yaoxiang update`         | ❌          | ✅           | Update dependencies    |
| `yaoxiang fmt`            | ✅          | ✅           | Format                 |
| `yaoxiang check`          | ✅          | ✅           | Type check             |
| `yaoxiang` (no arguments) | ✅          | ✅           | Enter REPL directly    |

#### Command Details

| Command                            | Function                                | Example                                              |
| ---------------------------------- | --------------------------------------- | ---------------------------------------------------- |
| `yaoxiang`                         | Enter REPL directly                     | `yaoxiang`                                           |
| `yaoxiang run <file>`              | Run single file/project                 | `yaoxiang run main.yx`                               |
| `yaoxiang init <name>`             | Create new project                      | `yaoxiang init my-app`                               |
| `yaoxiang build`                   | Build project                           | `yaoxiang build`                                     |
| `yaoxiang build <file>`            | Build single file                       | `yaoxiang build foo.yx`                              |
| `yaoxiang add <dep>`               | Add dependency                          | `yaoxiang add foo`                                   |
| `yaoxiang add -D <dep>`            | Add dev dependency                      | `yaoxiang add -D test`                               |
| `yaoxiang rm <dep>`                | Remove dependency                       | `yaoxiang rm foo`                                    |
| `yaoxiang update`                  | Update all dependencies                 | `yaoxiang update`                                    |
| `yaoxiang update foo`              | Update specific dependency              | `yaoxiang update foo`                                |
| `yaoxiang install`                 | Install all dependencies                | `yaoxiang install`                                   |
| `yaoxiang list`                    | List dependencies                       | `yaoxiang list`                                      |
| `yaoxiang outdated`                | Check outdated dependencies             | `yaoxiang outdated`                                  |
| `yaoxiang fmt`                     | Format code                             | `yaoxiang fmt`                                       |
| `yaoxiang check`                   | Type check                              | `yaoxiang check`                                     |
| `yaoxiang clean`                   | Clean build artifacts                   | `yaoxiang clean`                                     |
| `yaoxiang task <name>`             | Run custom task                         | `yaoxiang task lint`                                 |
| `yaoxiang publish`                 | Publish package to Registry             | `yaoxiang publish`                                   |
| `yaoxiang publish --github`        | Publish and create GitHub Release       | `yaoxiang publish --github`                          |
| `yaoxiang yank <pkg>@<ver>`        | Remove published version (irreversible) | `yaoxiang yank foo@1.2.3`                            |
| `yaoxiang login --registry <url>`  | Registry authentication                 | `yaoxiang login --registry https://reg.example.com`  |
| `yaoxiang login --github`          | GitHub authentication                   | `yaoxiang login --github`                            |
| `yaoxiang logout --registry <url>` | Logout                                  | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean`             | Clean global cache                      | `yaoxiang cache clean`                               |
| `yaoxiang workspace <cmd>`         | Workspace operations                    | `yaoxiang workspace list`                            |

#### Command Constraint Notes

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

- ✅ Existing `use` syntax fully preserved
- ✅ Existing module resolution logic unchanged
- ✅ New .yaoxiang/vendor directory does not affect existing projects

### Global Cache

All downloaded dependencies cached in `~/.yaoxiang/cache/`, project vendor directory copied from
cache.

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
- Git dependencies: cached by tag/rev; if tag unchanged, not invalidated
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
- Tokens never written to `yaoxiang.toml` or `yaoxiang.lock`
- File permissions 600

### Yank Semantics

`yaoxiang yank foo@1.2.3` performs **deletion + version number lock**:

- Package completely deleted, unrecoverable
- Version number permanently occupied, cannot republish same version
- Projects with lockfile referencing that version will error, need upgrade
- **Security purpose**: Prevent npm-style supply chain attacks (attackers claiming deleted version
  numbers to inject malicious code)

### Registry Protocol

See [RFC-014a: Registry Protocol Specification](../draft/014a-registry-protocol.md).

Core design: open protocol + adapter layer. Official Registry as primary, GitHub Release/main branch
as secondary, support custom Registry.

### Build System

See [RFC-014b: Build System and Binary Distribution](../draft/014b-build-system.md).

Core design: declarative `[build]` configuration, precompiled-first/source-fallback, supports
cargo/cmake/custom strategies.

### Workspace

See [RFC-014c: Workspace Support](../draft/014c-workspace.md).

Core design: dictionary-style members declaration, shared lockfile, path dependencies, Cargo
workspace integration.

## Trade-offs

### Advantages

- Unified import syntax; users don't need to care about dependency sources
- Deterministic builds; lock files ensure build consistency
- Offline support; can develop offline after downloading locally
- Source trait makes future extension easy

### Disadvantages

- Requires additional storage space (.yaoxiang/vendor directory)
- Version conflicts require manual user resolution

## Alternative Approaches

| Approach                       | Why Not Chosen                                         |
| ------------------------------ | ------------------------------------------------------ |
| Real-time GitHub access        | Security and cache reuse difficult                     |
| Global cache ($HOME/.yaoxiang) | Poor isolation, complex version conflicts              |
| Registry-only support          | GitHub is the current mainstream code hosting platform |

## Implementation Strategy

### Phase Breakdown

| Phase         | Content                                                             | Status      |
| ------------- | ------------------------------------------------------------------- | ----------- |
| **Phase 1**   | TOML parsing, local dependencies, lock generation, basic algorithms | ✅ Complete |
| **Phase 2**   | GitHub support, .yaoxiang/vendor management, download tools         | ✅ Complete |
| **Phase 3**   | Global cache, semver crate replacement, CLI improvements            | Pending     |
| **Phase 3.5** | Source trait async conversion, async-trait integration              | Pending     |
| **Phase 4**   | Registry protocol, publish, auth (RFC-014a)                         | Pending     |
| **Phase 5**   | Build system, precompiled binaries (RFC-014b)                       | Pending     |
| **Phase 6**   | Workspace support (RFC-014c)                                        | Pending     |

### Dependencies

- No prerequisites
- Needs integration with `ModuleGraph` (`middle/passes/module/`)

### Risks

| Risk                                    | Mitigation                                                   |
| --------------------------------------- | ------------------------------------------------------------ |
| Complex dependency resolution algorithm | Implement simple version first, add conflict detection later |
| Unstable Git downloads                  | Retry and caching mechanisms                                 |
| Performance issues                      | Lazy loading, incremental resolution                         |

## Open Questions

- [x] `dev-dependencies` conditional compilation syntax? → Handled uniformly by RFC-014b build
      system
- [x] Integrity check algorithm (SHA-256 / BLAKE3)? → SHA-256
- [ ] `excludes` to exclude specific files from download?
- [ ] Package naming convention (namespace support, e.g., `@org/pkg`)?
- [ ] Registry API versioning strategy?

---

## Dependencies (Cargo.toml additions required)

| Purpose             | crate            | Description                 |
| ------------------- | ---------------- | --------------------------- |
| Semantic versioning | `semver`         | Replace hand-written parser |
| HTTP client         | `reqwest`        | Registry communication      |
| SHA-256             | `sha2`           | Integrity verification      |
| Compression         | `flate2` + `tar` | Package format handling     |

---

## References

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
