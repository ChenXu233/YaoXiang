---
title: 'RFC-014: Package Management System Design'
status: 'Accepted'
author: 'Chenxu'
created: '2026-02-12'
updated: '2026-09-29'
group: 'rfc-014' # This RFC is the outline of the package management system, with sub-RFCs: 014a/014b/014c
issue: '#88'
impl: '48%'
impl_status: 'partial'
---

# RFC-014: Package Management System Design (Outline)

> **Sub-RFCs:**
>
> - [RFC-014a: Registry Protocol Specification](../accepted/014a-registry-protocol.md)
> - [RFC-014b: Build System and Binary Distribution](../accepted/014b-build-system.md)
> - [RFC-014c: Workspace Support](../accepted/014c-workspace.md)

## Summary

Design a package management system for the YaoXiang language, supporting semantic version control,
local and GitHub dependencies, unified import syntax, `yaoxiang.toml` configuration file, and
`yaoxiang.lock` lock file.

## Motivation

### Why is this feature/change needed?

Package management is the infrastructure of modern programming language ecosystems. Currently, the
YaoXiang language lacks:

- Dependency declaration mechanism
- Version management capability
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
│           Resolution Engine                  │ ← Dependency Resolution
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← Extensible Source
├──────────┬──────────┬──────────┬────────────┤
│  Local   │   Git    │ Registry │   GitHub   │
│          │  (VCS)   │  (Open)  │ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**Extension Mechanism**: Adding new Source types only requires implementing a trait, no need to
modify the resolution engine.

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

[workspace.members]     # Only for workspace root
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

> **Resolution of 2026-09-15: Mutual exclusion semantics for core package sources (Python venv /
> Node node_modules style).** Deprecate the original "5-layer penetration lookup chain"
> description—**vendor and global are mutually exclusive** as the only core package source, std is
> embedded in the core source, and local modules can override all others.

**Core Package Source Determination (mutually exclusive, never mixed)**:

- If project has `.yaoxiang/vendor/` → **vendor is the only core package source**. All non-local
  module `use` statements only resolve from vendor; missing packages in vendor error out directly
  (hint `yaoxiang install`), **no global fallback**.
- Otherwise → **global is the core package source** (install-dir std + global cache).

There is no "vendor missing falls back to global cache" per-package penetration—mixing the two
sources is exactly the root cause of version drift and "works on my machine".

#### Project Mode (with yaoxiang.toml)

```
use foo.bar.baz;

Lookup order:
1. ./src/foo/bar/baz.yx     Local module — highest priority, can override same-name modules in core source
2. <core-source>/foo/bar/baz.yx
   · Vendor mode: .yaoxiang/vendor/<pkg>-<ver>/src/foo/bar/baz.yx (2026-09-28 revision: std is not in vendor, see Project Mode Rules)
   · Global mode:  <install-dir>/yx/<ver>/std/foo/bar/baz.yx + ~/.yaoxiang/cache/...
3. std.* dedicated fallback: embedded binary (only std.* namespace; 2026-09-28 revision: formal mechanism, see Project Mode Rules)
4. Error (module does not exist); in vendor mode, missing packages hint `yaoxiang install`
```

**Project Mode Rules**:

- When vendor exists but is inconsistent with `yaoxiang.lock`, `run`/`build` errors and hints
  `yaoxiang install` (Node semantics: no silent auto-install)
- When local modules override same-name modules in core source, emit W-level diagnostic hint by
  default (`--deny-shadowing` can escalate to error)
- `path` dependencies are treated as extensions of local modules, resolved directly by path, not via
  core package source

> **2026-09-28 Revision: std is not packaged (overturning the std-packaging portion of the
> 2026-09-15 resolution).** Embedded binary std has transitioned from "transitional fallback" to
> **formal mechanism**: std is coupled with the compiler at the ABI level (native layer/runtime
> built-ins must match VM layout), locking std versions per package creates subtle chaos like
> "std-1.0.1 + compiler 1.0.2"; the correct solution to std drift is project-level toolchain locking
> (to be discussed separately if needed), not std packages. Go/Rust/Python precedents
> agree—language-provided std follows the toolchain. `yaoxiang add std@<ver>` is unavailable;
> `std.*` is a reserved namespace, local modules cannot override (embedded std has no file form, so
> overriding is moot); RFC-037's `.yaoxiang/vendor/std/` interface file directory (first level of
> LSP lookup chain) is preserved, continuing to serve the "view source code" role. Incidental fix:
> the diagnostic scope for local module shadowing is dependencies, no longer including the std.*
> special case wording.

#### Single File Mode (without yaoxiang.toml)

```
use foo.bar.baz;

Lookup order:
1. ./src/foo/bar/baz.yx     Local module
2. Global core package source: <install-dir>/yx/<version>/std/foo/bar/baz.yx
3. Embedded binary std fallback
4. $YXPATH/foo/bar/baz.yx   (global path, reserved)
```

**Single File Mode Rules**:

- No project-level dependency concept, std comes directly from global; global standard library path
  is bound to compiler version: `<install-dir>/yx/<version>/std/`
- Single file mode never reads `.yaoxiang/` (no vendor concept)

### Standard Library Installation Directory Structure

#### Global Standard Library

```
<yaoxiang-install-dir>/
├── yx/                          # YaoXiang language directory
│   ├── 1.0.1/                   # Version directory
│   │   ├── std/
│   │   │   ├── test.yx          # Pure YaoXiang standard library module
│   │   │   ├── math.yx          # Future bootstrapping module
│   │   │   └── ...
│   │   └── ...
│   └── 1.1.0/
│       └── std/
│           └── ...
└── bin/
    └── yaoxiang                 # Compiler binary
```

#### Project-Level Standard Library

> **2026-09-15 Resolution: No longer set up independent `.yaoxiang/std/` directory.** **2026-09-28
> Revision: std is not packaged** (see reasoning at end of "Project Mode Rules")—std remains
> embedded binary + RFC-037 interface file directory, `add std@<ver>` is unavailable, `std.*` is
> reserved and cannot be shadowed. The directory mutual exclusion conclusion (no independent
> `.yaoxiang/std/`) remains valid.

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

- Embedded binary as compatibility layer: provide std modules through embedded binary before the
  filesystem standard library is fully landed
- Version directory isolation: `yx/<version>/std/` allows different versions of the standard library
  to coexist without affecting each other
- std uses the same mechanism as regular dependencies (add/lock/vendor), no special directory, no
  special lookup layer
- Single file mode falls back to global std; when vendor exists, std still comes from embedded
  binary/interface directory (2026-09-28 revision: std is not packaged, does not follow vendor)

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

// Resolved dependency (2026-09-15 resolution: integrity only uses single integrity field, format "sha256-<hex>", no duplicate checksum)
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
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

Adopt a unified approach, integrating the compiler, package manager, and REPL into a single CLI
tool:

#### Single File Mode vs Project Mode

| Command                 | Single File | Project Mode | Description            |
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

| Command                            | Function                                                           | Example                                                                              |
| ---------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| `yaoxiang`                         | Enter REPL directly                                                | `yaoxiang`                                                                           |
| `yaoxiang run <file>`              | Run single file/project                                            | `yaoxiang run main.yx`                                                               |
| `yaoxiang init <name>`             | Create new project                                                 | `yaoxiang init my-app`                                                               |
| `yaoxiang build`                   | Build project                                                      | `yaoxiang build`                                                                     |
| `yaoxiang build <file>`            | Build single file                                                  | `yaoxiang build foo.yx`                                                              |
| `yaoxiang add <dep>`               | Add dependency                                                     | `yaoxiang add foo`                                                                   |
| `yaoxiang add -D <dep>`            | Add dev dependency                                                 | `yaoxiang add -D test`                                                               |
| `yaoxiang rm <dep>`                | Remove dependency                                                  | `yaoxiang rm foo`                                                                    |
| `yaoxiang update`                  | Update all dependencies                                            | `yaoxiang update`                                                                    |
| `yaoxiang update foo`              | Update specific dependency                                         | `yaoxiang update foo`                                                                |
| `yaoxiang install`                 | Install all dependencies                                           | `yaoxiang install`                                                                   |
| `yaoxiang list`                    | List dependencies                                                  | `yaoxiang list`                                                                      |
| `yaoxiang outdated`                | Check outdated dependencies                                        | `yaoxiang outdated`                                                                  |
| `yaoxiang fmt`                     | Format code                                                        | `yaoxiang fmt`                                                                       |
| `yaoxiang check`                   | Type check                                                         | `yaoxiang check`                                                                     |
| `yaoxiang clean`                   | Clean build artifacts                                              | `yaoxiang clean`                                                                     |
| `yaoxiang task <name>`             | Run custom task                                                    | `yaoxiang task lint`                                                                 |
| `yaoxiang publish`                 | Publish package to Registry                                        | Deferred: official Registry indefinitely deferred; bare publish errors with guidance |
| `yaoxiang publish --dry-run`       | Validate + pack `.yxpkg` to `target/yxpkg/`                        | `yaoxiang publish --dry-run`                                                         |
| `yaoxiang publish --github`        | Publish as GitHub Release (`.yxpkg` assets; requires tag to exist) | `yaoxiang publish --github`                                                          |
| `yaoxiang yank <pkg>@<ver>`        | Delete published version (irreversible)                            | Deferred: with official Registry                                                     |
| `yaoxiang login --registry <url>`  | Registry authentication                                            | Deferred: with official Registry (GitHub side currently uses `$YX_GITHUB_TOKEN`)     |
| `yaoxiang login --github`          | GitHub authentication                                              | Deferred: same as above                                                              |
| `yaoxiang logout --registry <url>` | Logout                                                             | Deferred: same as above                                                              |
| `yaoxiang cache clean`             | Clean global cache                                                 | `yaoxiang cache clean`                                                               |
| `yaoxiang workspace <cmd>`         | Workspace operations                                               | `yaoxiang workspace list`                                                            |

#### Command Constraint Description

```bash
# Single file mode: no yaoxiang.toml required
yaoxiang run hello.yx   # ✅ Works normally
yaoxiang add foo        # ❌ Error: not a project directory

# Project mode: requires yaoxiang.toml
cd my-project
yaoxiang run main.yx    # ✅ Run entry file
yaoxiang build          # ✅ Build project
yaoxiang add foo        # ✅ Add dependency
```

### Backward Compatibility

- ✅ Existing `use` syntax fully preserved
- ✅ Existing module resolution logic unchanged
- ✅ Adding .yaoxiang/vendor directory does not affect existing projects

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

- Registry packages: version number is immutable, never invalidates
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
- File permissions 600

### Yank Semantics

`yaoxiang yank foo@1.2.3` performs **deletion + version number lockout**:

- Package is completely deleted, irrecoverable
- Version number is permanently occupied, cannot republish the same version number
- Projects with lockfile references to that version will error and need to upgrade
- **Security purpose**: prevent npm-style supply chain attacks (attackers grabbing deleted version
  numbers to inject malicious code)

### Registry Protocol

See [RFC-014a: Registry Protocol Specification](../accepted/014a-registry-protocol.md) for details.

Core design: open protocol + adapter layer. Official Registry is primary, GitHub Release/main branch
is supplementary, custom Registries are supported.

### Build System

See [RFC-014b: Build System and Binary Distribution](../accepted/014b-build-system.md) for details.

Core design: declarative `[build]` configuration, precompiled priority/source code fallback,
supports cargo/cmake/custom strategies.

### Workspace

See [RFC-014c: Workspace Support](../accepted/014c-workspace.md) for details.

Core design: dictionary-style members declaration, shared lockfile, path dependencies, Cargo
workspace integration.

## Trade-offs

### Advantages

- Unified import syntax, users don't need to care about dependency sources
- Deterministic builds, lock file guarantees build consistency
- Offline support, can develop offline after downloading to local
- Source trait is easy to extend later

### Disadvantages

- Requires additional storage space (.yaoxiang/vendor directory)
- Version conflicts require manual user resolution

## Alternatives

| Solution                       | Why not chosen                                         |
| ------------------------------ | ------------------------------------------------------ |
| Real-time GitHub access        | Security and cache reuse hard to guarantee             |
| Global cache ($HOME/.yaoxiang) | Poor isolation, complex version conflicts              |
| Registry only                  | GitHub is the current mainstream code hosting platform |

## Implementation Strategy

### Phase Division

| Phase         | Content                                                                                                                 | Status                                                   |
| ------------- | ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| **Phase 1**   | toml parsing, local dependencies, lock generation, basic algorithms                                                     | ✅ Completed                                             |
| **Phase 2**   | GitHub support, .yaoxiang/vendor management, download tools                                                             | ✅ Completed                                             |
| **Phase 3**   | Global cache, semver crate replacement, CLI improvements                                                                | ✅ Completed                                             |
| **Phase 3.5** | Source dispatch enum-ification + native async (014a resolution 4, no async-trait)                                       | ✅ Completed                                             |
| **Phase 4**   | GitHub adapter layer, .yxpkg packaging, publish --github (RFC-014a reduced scope; official Registry/auth/yank deferred) | ✅ Completed                                             |
| **Phase 5**   | Build system, precompiled binaries (RFC-014b)                                                                           | ✅ Completed                                             |
| **Phase 6**   | Workspace support (RFC-014c)                                                                                            | ✅ 6a-c + member management + 6d completed (6e deferred) |

**Execution order adjustment (2026-09-15)**: `3 → 3.5 → 6 → 4 → 5`.

- Workspace (Phase 6) is moved up before the build system—it does not depend on the network or build
  system (pure local path resolution + shared lockfile), and provides the most direct benefit for
  multi-package development.
- Phase 4 scope reduction: **Official Registry server and auth/yank are indefinitely deferred**,
  first deliver GitHub Release/Git adapter layer + `.yxpkg` packaging + `publish --github`.
  Cold-starting the ecosystem only needs git/GitHub channels (Go's early stage was the same), and
  the operational and governance costs of the Registry server are a pure liability at the stage
  where there are no third-party packages.
- Consequent constraint: before the official Registry goes online,
  `yaoxiang add <bare package name>` is unavailable; adding dependencies requires explicit source
  (`--git` / `--path`).

**Phase 3 landing notes (2026-09-28)**:

- Phase 3.5 landing notes (2026-09-29): Source dispatch per resolution 4 uses `AnySource`
  four-source closed set (Local/Git/Registry/GitHub, the latter two being Phase 4 placeholders); the
  `Source` trait's resolve/download is native async fn in trait, the command layer is driven by
  `futures::executor::block_on` (no runtime, Git subprocesses remain std::process; Phase 4 switches
  to real executor when connecting reqwest); add `futures` dependency (wasm32 compatible). install
  parallel download deferred until a real runtime is introduced.
- Phase 4 landing notes (2026-09-29, commits 234dfea5/e1873133/5ffee636):
  - **GitHub adapter layer** (4a): github.com git dependencies route to `GitHubSource`—version
    resolution via REST API (releases endpoint, falls back to tags if empty), download prefers
    Release `.yxpkg` assets (unpack-validate then enter `cache/github/` before copying to vendor),
    no asset falls back to git clone (SourceKind reports `Git` truthfully). API access with
    exponential backoff (1s/2s/4s, Retry-After takes priority) + **ETag conditional request cache**
    (`cache/github/*.etag|body`, 304 does not count toward GitHub rate quota);
    403+`x-ratelimit-remaining: 0` identified as primary rate limit, no retry.
  - **`.yxpkg` package format** (4b): tar.gz + `SHA256SUMS` manifest (coreutils double-space
    format), deterministic packaging (entry sorting, mtime/uid/gid zeroed); unpacking enforces
    validation (missing manifest/tampering/extra files/path escape/total unpack size exceeded all
    error); total content size 20 MiB limit (resolution 7). Exclusions use blacklist
    (`.git`/`.yaoxiang`/`target`/`node_modules`/`*.yxpkg` etc.) instead of whitelist—`[exports]`
    allows including files outside src/ in the export surface, whitelist would silently miss them.
  - **`publish`** (4c): bare `publish` errors with guidance (Registry deferred); `--dry-run`
    completes "validate (description required) → pack → SHA-256"; `--github` then check for existing
    Release → validate tag exists (Cargo's same semantics: tagging is the user's responsibility) →
    create Release → upload assets. Target repo takes `[package].repository`, falls back to
    `git remote origin`; authentication reads `$YX_GITHUB_TOKEN` (credentials.toml comes with
    official Registry landing). HTTP stack is reqwest (rustls) + package management's own tokio
    current_thread runtime (`package::runtime::drive`), `futures` dependency removed accordingly.
  - Pre-publish test running (014a validation list step 3) wires with Phase 5 (03929ffa).
- Phase 5 landing notes (2026-09-30, feat/rfc014):
  - **Install decision tree wiring** (b6a5b98f and surrounding commits): after `install/update`
    downloads dependencies, packages with `[build]`/`[binaries]` go through
    `build::run_install_build`—precompiled priority (full package SHA-256 + safe unpack) → headers
    (explicit error before 026b) → strategy execution (cargo real implementation / cmake pending /
    custom trust gate). Packages without build declarations pass through at zero cost.
  - **cargo strategy**: `[build.cargo]` splices commands + platform override merging; scratch
    isolated to `.yaoxiang/build/` via `CARGO_TARGET_DIR`, FFI artifacts copied into vendor
    `build/native/<triple>/`; vendor integrity semantics clarified as source tree integrity
    (`build/` not in checksum).
  - **Trust gate** (014b resolution 1): trust records in user config `[trust] build-scripts`;
    `--trust` whitelist persists; non-interactive environments default to deny.
  - publish pre-release test runs by default (RFC-036 discovery mechanism), `--no-test` skips.
  - cmake execution and yx-bindgen generator (RFC-026b) pending; see 014b landing notes for the
    rest.

- Global cache first covers **git channels** (`cache/git/<url>-<tag|rev|commit>/`, branches resolved
  to commit via `ls-remote` and entered into cache with pointer files for offline fallback);
  `cache/registry/`, `cache/binaries/` are directory placeholders. Vendor copies exclude `.git`,
  directory names use the real version detected by dependency manifest probing (vendor/lock/cleanup
  are all from the same source).
- `semver` and `sha2` crates replace handwritten implementations per dependency table;
  `is_compatible` changed from 100k enumeration to interval intersection.
- CLI adds `outdated` / `clean` / `cache clean`, and adds `--git` / `--path` explicit source to
  `add` (implementing the above constraints). `clean` in addition to `.yaoxiang/build/` also trims
  residual packages in vendor not referenced by lock.
- Cache `[cache] dir` configuration is merged into the existing `~/.config/yaoxiang/config.toml`
  user configuration system (the `~/.yaoxiang/config.toml` in the RFC draft never existed), cache
  data default location remains `~/.yaoxiang/cache`.

### Dependencies

- No prerequisite dependencies
- Needs to integrate with `ModuleGraph` (`middle/passes/module/`)

### Risks

| Risk                                    | Mitigation                                                  |
| --------------------------------------- | ----------------------------------------------------------- |
| Dependency resolution algorithm complex | First implement simple version, then add conflict detection |
| Git download unstable                   | Retry and cache mechanisms                                  |
| Performance issues                      | Lazy loading, incremental resolution                        |

## Open Questions

- [x] `dev-dependencies` conditional compilation syntax? → Handled uniformly by RFC-014b build
      system
- [x] Integrity verification algorithm (SHA-256 / BLAKE3)? → SHA-256
- [x] Package naming convention (whether to support namespace, e.g. `@org/pkg`)? → Initially flat
      package names, no namespace support; `@org/pkg` reserved (2026-09-15 resolution)
- [x] Registry API versioning strategy? → URL path `/api/v1/` + response header carries protocol
      version, breaking changes upgrade to v2 and coexist (2026-09-15 resolution, see RFC-014a)
- [ ] `excludes` to exclude specific files from being downloaded?

---

## Dependencies (Cargo.toml additions required)

| Purpose          | crate            | Description                      |
| ---------------- | ---------------- | -------------------------------- |
| Semantic version | `semver`         | Replace handwritten parser       |
| HTTP client      | `reqwest`        | Registry communication (Phase 4) |
| SHA-256          | `sha2`           | Integrity verification           |
| Compression      | `flate2` + `tar` | Package format handling          |

---

## References

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
