---
title: 'RFC-014: Package Management System Design'
status: 'Accepted'
author: 'Chenxu'
created: '2026-02-12'
updated: '2026-09-15'
group: 'rfc-014' # This RFC is the master document of the package management system; sub-RFCs: 014a/014b/014c
issue: '#88'
impl: '48%'
impl_status: 'partial'
---

# RFC-014: Package Management System Design (Master Document)

> **Sub-RFCs:**
>
> - [RFC-014a: Registry Protocol Specification](../review/014a-registry-protocol.md)
> - [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md)
> - [RFC-014c: Workspace Support](../review/014c-workspace.md)

## Summary

Design the package management system for the YaoXiang language, supporting semantic versioning,
local and GitHub dependencies, unified import syntax, `yaoxiang.toml` configuration file, and
`yaoxiang.lock` lock file.

## Motivation

### Why is this feature/change needed?

Package management is the infrastructure foundation of modern programming language ecosystems.
Currently the YaoXiang language lacks:

- Dependency declaration mechanism
- Version management capability
- Standard distribution channel

### Current Problem

```
my-project/
├── src/
│   └── main.yx          # code depends on other modules
├── lib/                  # manually copied modules
│   ├── foo.yx
│   └── bar.yx
└── ???                   # no standard dependency management
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
│ (Local)  │  (VCS)   │ (Open)   │ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**Extension Mechanism**: Adding a new Source type only requires implementing the trait, no need to
modify the resolution engine.

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

[workspace.members]     # Only in workspace root
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

> **2026-09-15 Resolution: Mutually exclusive core package source semantics (Python venv / Node
> node_modules style).** The original "5-layer passthrough lookup chain" description is
> deprecated—vendor and global are **mutually exclusive** as the sole core package source, std is
> embedded in the core source, and local modules can override everything else.

**Core Package Source Determination (mutually exclusive, never mixed)**:

- If the project has `.yaoxiang/vendor/` → **vendor is the sole core package source**. All `use` for
  non-local modules is resolved only from vendor; missing packages in vendor report an error
  directly (prompt `yaoxiang install`), **no global fallback**.
- Otherwise → **global is the core package source** (installation directory std + global cache).

There is no "if not in vendor, fall back to global cache" per-package passthrough—mixing the two
sources is precisely the root cause of version drift and "works on my machine" issues.

#### Project Mode (has yaoxiang.toml)

```
use foo.bar.baz;

Lookup order:
1. ./src/foo/bar/baz.yx     Local module — highest priority, can override modules with the same name in the core source
2. <core-source>/foo/bar/baz.yx
   · Vendor mode: .yaoxiang/vendor/<pkg>-<ver>/src/foo/bar/baz.yx (std is also in vendor)
   · Global mode:   <install-dir>/yx/<ver>/std/foo/bar/baz.yx + ~/.yaoxiang/cache/...
3. std.* exclusive fallback: embedded binary (only the std.* namespace; transitional layer before the filesystem std lands, version-bound to compiler)
4. Error (module does not exist); in vendor mode, missing package prompts `yaoxiang install`
```

**Project Mode Rules**:

- `yaoxiang add std@1.0.1` installs std as a regular dependency into vendor and locks the version;
  at this point the embedded binary std no longer takes effect (vendor std takes priority)
- When vendor exists but is inconsistent with `yaoxiang.lock`, `run`/`build` reports an error and
  prompts `yaoxiang install` (Node semantics: no silent auto-install)
- When local modules override same-name modules in the core source, emit a W-level diagnostic
  prompting about shadowing by default (`--deny-shadowing` can escalate to error); overriding
  `std.*` shows an explicit warning in the diagnostic message
- `path` dependencies are treated as an extension of local modules, resolved directly by path, not
  through the core package source

#### Single-File Mode (no yaoxiang.toml)

```
use foo.bar.baz;

Lookup order:
1. ./src/foo/bar/baz.yx     Local module
2. Global core package source: <install-dir>/yx/<version>/std/foo/bar/baz.yx
3. Embedded binary std fallback
4. $YXPATH/foo/bar/baz.yx   (global path, reserved)
```

**Single-File Mode Rules**:

- No project-level dependency concept, std comes directly from global; the global standard library
  path is version-bound to the compiler: `<install-dir>/yx/<version>/std/`
- Single-file mode never reads `.yaoxiang/` (no vendor concept)

### Standard Library Installation Directory Structure

#### Global Standard Library

```
<yaoxiang-install-dir>/
├── yx/                          # YaoXiang language directory
│   ├── 1.0.1/                   # Version directory
│   │   ├── std/
│   │   │   ├── test.yx          # Pure YaoXiang standard library module
│   │   │   ├── math.yx          # Future self-hosted module
│   │   │   └── ...
│   │   └── ...
│   └── 1.1.0/
│       └── std/
│           └── ...
└── bin/
    └── yaoxiang                 # Compiler binary
```

#### Project-Level Standard Library

> **2026-09-15 Resolution: No longer set up a separate `.yaoxiang/std/` directory.** std is a
> regular package in the core package source: after `yaoxiang add std@1.0.1`, it lands in
> `.yaoxiang/vendor/std-<version>/`, managed by the same rules as other dependencies. The original
> "project-level std exists so global std is invalidated" rule is no longer needed—mutual exclusion
> of core package sources naturally guarantees it.

```
my-project/
├── yaoxiang.toml
├── yaoxiang.lock
├── .yaoxiang/
│   └── vendor/
│       ├── std-1.0.1/           # std as a regular vendor package
│       └── foo-1.2.3/
├── src/
│   └── main.yx
```

**Design Highlights**:

- Embedded binary as compatibility layer: before the filesystem standard library fully lands, std
  modules are provided through the embedded binary first
- Version directory isolation: `yx/<version>/std/` allows different versions of the standard library
  to coexist without affecting each other
- std and regular dependencies use the same mechanism (add/lock/vendor), with no special directory
  and no special lookup layer
- Single-file mode falls back to global std; when vendor exists, std must come from vendor (or be
  explicitly locked via `add std@`)

### Core Data Structures

```rust
// Dependency source (extensible)
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> },
    GitHub { owner: String, repo: String, ref_: GitRef },  // GitHub native
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

// Resolved dependency (2026-09-15 Resolution: integrity uses a single integrity field with format "sha256-<hex>"; no duplicate checksum)
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
}

// Build strategy
enum BuildStrategy {
    None,          // Pure .yx package
    Cargo,         // Call cargo build
    Cmake,         // Call cmake
    Custom,        // Execute build.yx script
    Precompiled,   # Use precompiled artifacts directly
}
```

### CLI Command Design

Adopt a unified approach, integrating the compiler, package manager, and REPL into a single CLI
tool:

#### Single-File Mode vs Project Mode

| Command                 | Single-File | Project Mode | Description            |
| ----------------------- | ----------- | ------------ | ---------------------- |
| `yaoxiang run <file>`   | ✅          | ✅           | Run file/project entry |
| `yaoxiang build`        | ❌          | ✅           | Build project          |
| `yaoxiang build <file>` | ✅          | ✅           | Build single file      |
| `yaoxiang init <name>`  | ❌          | ✅           | Create project         |
| `yaoxiang add <dep>`    | ❌          | ✅           | Add dependency         |
| `yaoxiang update`       | ❌          | ✅           | Update dependencies    |
| `yaoxiang fmt`          | ✅          | ✅           | Format                 |
| `yaoxiang check`        | ✅          | ✅           | Type check             |
| `yaoxiang` (no args)    | ✅          | ✅           | Enter REPL directly    |

#### Command Details

| Command                            | Function                              | Example                                              |
| ---------------------------------- | ------------------------------------- | ---------------------------------------------------- |
| `yaoxiang`                         | Enter REPL directly                   | `yaoxiang`                                           |
| `yaoxiang run <file>`              | Run single file/project               | `yaoxiang run main.yx`                               |
| `yaoxiang init <name>`             | Create new project                    | `yaoxiang init my-app`                               |
| `yaoxiang build`                   | Build project                         | `yaoxiang build`                                     |
| `yaoxiang build <file>`            | Build single file                     | `yaoxiang build foo.yx`                              |
| `yaoxiang add <dep>`               | Add dependency                        | `yaoxiang add foo`                                   |
| `yaoxiang add -D <dep>`            | Add dev dependency                    | `yaoxiang add -D test`                               |
| `yaoxiang rm <dep>`                | Remove dependency                     | `yaoxiang rm foo`                                    |
| `yaoxiang update`                  | Update all dependencies               | `yaoxiang update`                                    |
| `yaoxiang update foo`              | Update specified dependency           | `yaoxiang update foo`                                |
| `yaoxiang install`                 | Install all dependencies              | `yaoxiang install`                                   |
| `yaoxiang list`                    | List dependencies                     | `yaoxiang list`                                      |
| `yaoxiang outdated`                | Check outdated dependencies           | `yaoxiang outdated`                                  |
| `yaoxiang fmt`                     | Format code                           | `yaoxiang fmt`                                       |
| `yaoxiang check`                   | Type check                            | `yaoxiang check`                                     |
| `yaoxiang clean`                   | Clean build artifacts                 | `yaoxiang clean`                                     |
| `yaoxiang task <name>`             | Run custom task                       | `yaoxiang task lint`                                 |
| `yaoxiang publish`                 | Publish package to Registry           | `yaoxiang publish`                                   |
| `yaoxiang publish --github`        | Publish and create GitHub Release     | `yaoxiang publish --github`                          |
| `yaoxiang yank <pkg>@<ver>`        | Yank published version (irreversible) | `yaoxiang yank foo@1.2.3`                            |
| `yaoxiang login --registry <url>`  | Registry authentication               | `yaoxiang login --registry https://reg.example.com`  |
| `yaoxiang login --github`          | GitHub authentication                 | `yaoxiang login --github`                            |
| `yaoxiang logout --registry <url>` | Log out                               | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean`             | Clean global cache                    | `yaoxiang cache clean`                               |
| `yaoxiang workspace <cmd>`         | Workspace operation                   | `yaoxiang workspace list`                            |

#### Command Constraint Description

```bash
# Single-file mode: yaoxiang.toml is not required
yaoxiang run hello.yx   # ✅ Works normally
yaoxiang add foo        # ❌ Error: not a project directory

# Project mode: yaoxiang.toml is required
cd my-project
yaoxiang run main.yx    # ✅ Run entry file
yaoxiang build          # ✅ Build project
yaoxiang add foo        # ✅ Add dependency
```

### Backward Compatibility

- ✅ Existing `use` syntax fully preserved
- ✅ Existing module resolution logic unchanged
- ✅ Adding `.yaoxiang/vendor` directory does not affect existing projects

### Global Cache

All downloaded dependencies are cached to `~/.yaoxiang/cache/`, and the project vendor directory is
copied from the cache.

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
- Git dependencies: cached by tag/rev, no invalidation if tag is unchanged
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

`yaoxiang yank foo@1.2.3` performs **deletion + version number lockout**:

- Package is completely deleted, irrecoverable
- Version number is permanently occupied, the same version number cannot be republished
- Projects with existing lockfile references to that version will report errors and need to upgrade
- **Security purpose**: prevent npm-style supply chain attacks (attacker grabs a deleted version
  number to inject malicious code)

### Registry Protocol

See [RFC-014a: Registry Protocol Specification](../review/014a-registry-protocol.md) for details.

Core design: open protocol + adapter layer. The official Registry is primary, GitHub Release/main
branch is auxiliary, custom Registries are supported.

### Build System

See [RFC-014b: Build System and Binary Distribution](../review/014b-build-system.md) for details.

Core design: declarative `[build]` configuration, precompiled-first/source-fallback, supporting
cargo/cmake/custom strategies.

### Workspace

See [RFC-014c: Workspace Support](../review/014c-workspace.md) for details.

Core design: dictionary-form members declaration, shared lockfile, path dependencies, Cargo
workspace integration.

## Trade-offs

### Advantages

- Unified import syntax, users don't need to care about dependency sources
- Deterministic builds, lock file ensures build consistency
- Offline support, can develop offline after downloading
- Source trait facilitates future extension

### Disadvantages

- Requires additional storage space (.yaoxiang/vendor directory)
- Version conflicts require manual resolution by users

## Alternatives

| Scheme                              | Why Not Chosen                                         |
| ----------------------------------- | ------------------------------------------------------ |
| Real-time GitHub access             | Security and cache reuse hard to guarantee             |
| Global cache only ($HOME/.yaoxiang) | Poor isolation, complex version conflicts              |
| Registry-only support               | GitHub is the current mainstream code hosting platform |

## Implementation Strategy

### Phase Division

| Phase         | Content                                                                                                           | Status   |
| ------------- | ----------------------------------------------------------------------------------------------------------------- | -------- |
| **Phase 1**   | toml parsing, local dependencies, lock generation, basic algorithms                                               | ✅ Done  |
| **Phase 2**   | GitHub support, .yaoxiang/vendor management, download tool                                                        | ✅ Done  |
| **Phase 3**   | Global cache, semver crate replacement, CLI refinement                                                            | To start |
| **Phase 3.5** | Source trait converted to async, async-trait integration                                                          | To start |
| **Phase 4**   | GitHub adapter, .yxpkg packaging, publish --github (RFC-014a reduced scope; official Registry/auth/yank deferred) | To start |
| **Phase 5**   | Build system, precompiled binaries (RFC-014b)                                                                     | To start |
| **Phase 6**   | Workspace support (RFC-014c)                                                                                      | To start |

**Execution Order Adjustment (2026-09-15)**: `3 → 3.5 → 6 → 4 → 5`.

- Workspace (Phase 6) is moved before the build system—it does not depend on networking or the build
  system (pure local path resolution + shared lockfile), and brings the most direct benefit to
  multi-package development.
- Phase 4 scope reduced: **Official Registry server and auth/yank are deferred indefinitely**,
  delivering first the GitHub Release/Git adapter + `.yxpkg` packaging + `publish --github`.
  Cold-starting the ecosystem only needs git/GitHub channels (Go's early days followed the same
  pattern); the operational and governance costs of a Registry server are pure liabilities at the
  stage when there are no third-party packages.
- Consequent constraint: `yaoxiang add <bare-package-name>` is unavailable before the official
  Registry goes online; adding dependencies requires an explicit source (`--git` / `--path`).

### Dependencies

- No prerequisites
- Needs to integrate with `ModuleGraph` (`middle/passes/module/`)

### Risks

| Risk                                       | Mitigation                                                    |
| ------------------------------------------ | ------------------------------------------------------------- |
| Dependency resolution algorithm is complex | First implement a simple version, then add conflict detection |
| Git download instability                   | Retry and cache mechanism                                     |
| Performance issues                         | Lazy loading, incremental resolution                          |

## Open Questions

- [x] `dev-dependencies` conditional compilation syntax? → Handled uniformly by the RFC-014b build
      system
- [x] Integrity verification algorithm (SHA-256 / BLAKE3)? → SHA-256
- [x] Package naming convention (whether to support namespace, e.g. `@org/pkg`)? → Flat package
      names initially, namespace not supported; `@org/pkg` reserved (2026-09-15 resolution)
- [x] Registry API versioning strategy? → URL path `/api/v1/` + response header carries protocol
      version, breaking changes bump to v2 and coexist (2026-09-15 resolution, see RFC-014a)
- [ ] `excludes` to exclude specific files from download?

---

## Dependencies (to be added to Cargo.toml)

| Purpose             | crate            | Description                 |
| ------------------- | ---------------- | --------------------------- |
| Semantic versioning | `semver`         | Replace hand-written parser |
| HTTP client         | `reqwest`        | Registry communication      |
| SHA-256             | `sha2`           | Integrity verification      |
| Compression         | `flate2` + `tar` | Package format processing   |

---

## References

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
