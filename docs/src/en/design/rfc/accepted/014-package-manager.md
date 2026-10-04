---
title: 'RFC-014: Package Management System Design'
status: 'Accepted'
author: 'Chenxu'
created: '2026-02-12'
updated: '2026-10-03'
group: 'rfc-014' # This RFC is the master document for the package management system; sub-RFCs: 014a/014b/014c
issue: '#88'
impl: '100%'
impl_status: 'complete'
---

# RFC-014: Package Management System Design (Master Document)

> **Sub-RFCs:**
>
> - [RFC-014a: Registry Protocol Specification](../accepted/014a-registry-protocol.md)
> - [RFC-014b: Build System and Binary Distribution](../accepted/014b-build-system.md)
> - [RFC-014c: Workspace Support](../accepted/014c-workspace.md)

## Summary

Design the package management system for the YaoXiang language, supporting semantic versioning,
local and GitHub dependencies, unified import syntax, the `yaoxiang.toml` config file and
`yaoxiang.lock` lock file.

## Motivation

### Why is this feature/change needed?

Package management is the infrastructure of modern programming language ecosystems. The current
YaoXiang language lacks:

- A dependency declaration mechanism
- Version management capability
- A standard distribution channel

### Current problems

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
│           Resolution Engine                  │ ← dependency resolution
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← extensible sources
├──────────┬──────────┬──────────┬────────────┤
│  Local   │   Git    │ Registry │   GitHub   │
│ (local)  │  (VCS)   │ (open)   │ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**Extension Mechanism**: Adding a new Source type only requires implementing a trait, with no need
to modify the resolution engine.

### Example

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
├── yaoxiang.toml        # package config
├── yaoxiang.lock        # lock file (auto-generated)
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/              # local dependencies
        ├── foo-1.2.3/
        └── bar-0.5.0/
```

## Detailed Design

### Config File Format

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
foo = "1.2.3"           # exact version
bar = "^1.0.0"          # compatible version
baz = "~1.2.0"          # patch version
qux = { git = "...", version = "0.5.0" }
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"

[build]
strategy = "none"       # none | cargo | cmake | custom

[binaries]
"linux-x86_64" = { url = "...", sha256 = "..." }

[workspace.members]     # only at workspace root
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

> **2026-09-15 Resolution: Core package source mutual-exclusion semantics (Python venv / Node
> node_modules style).** Deprecating the original "5-layer cascading lookup chain"
> description—vendor and global are **mutually exclusive** as the single core package source; std is
> embedded in the core source; local modules can override everything else.

**Core package source determination (mutually exclusive, never mixed)**:

- If the project has `.yaoxiang/vendor/` → **vendor is the sole core package source**. All `use` of
  non-local modules resolves only from vendor; a missing package in vendor reports an error directly
  (prompting `yaoxiang install`); **no global fallback**.
- Otherwise → **global is the core package source** (install dir std + global cache).

There is no "fall through to global cache on per-package basis when vendor doesn't have it"—mixing
the two sources is precisely the root cause of version drift and "works on my machine" bugs.

#### Project Mode (with yaoxiang.toml)

```
use foo.bar.baz;

Lookup order:
1. ./src/foo/bar/baz.yx     local module — highest priority, can override same-name modules in the core source
2. <core source>/foo/bar/baz.yx
   · vendor mode: .yaoxiang/vendor/<pkg>-<ver>/src/foo/bar/baz.yx (2026-09-28 revision: std is not inside vendor, see project mode rules)
   · global mode:   <install-dir>/yx/<ver>/std/foo/bar/baz.yx + ~/.yaoxiang/cache/...
3. std.* exclusive fallback: embedded binary (only for the std.* namespace; 2026-09-28 revision: formal mechanism, see project mode rules)
4. Error (module does not exist); in vendor mode a missing package prompts `yaoxiang install`
```

**Project Mode Rules**:

- When vendor exists but is inconsistent with `yaoxiang.lock`, `run`/`build` reports an error and
  prompts `yaoxiang install` (Node semantics: no silent auto-install)
- When a local module overrides a same-name module in the core source, a W-level diagnostic is
  emitted by default to flag shadowing (`--deny-shadowing` upgrades to error)
- `path` dependencies are treated as an extension of local modules, resolved by path directly, not
  going through the core package source

> **2026-09-28 Revision: std is not packaged (overturning the std-packaging part of the 2026-09-15
> resolution).** The embedded binary std has graduated from "transitional fallback" to an **official
> mechanism**: std is ABI-coupled to the compiler (the native layer / runtime built-ins must match
> the VM layout), and locking the std version by package would create subtle "std-1.0.1 + compiler
> 1.0.2" mismatches; the proper fix for std drift is project-level toolchain locking (to be
> discussed separately if needed), not a std package. Go/Rust/Python precedents
> agree—language-provided std follows the toolchain. `yaoxiang add std@<ver>` is unavailable;
> `std.*` is a reserved namespace, and local modules may not shadow it (the embedded std has no file
> form, so shadowing is impossible); RFC-037's `.yaoxiang/vendor/std/` interface file directory (LSP
> lookup chain first level) is retained, continuing to serve the "view source" role. Incidental fix:
> the diagnostic scope for local-module shadowing covers dependency packages only, no longer
> carrying the std.* special-case wording.

#### Single-File Mode (no yaoxiang.toml)

```
use foo.bar.baz;

Lookup order:
1. ./src/foo/bar/baz.yx     local module
2. Global core package source: <install-dir>/yx/<version>/std/foo/bar/baz.yx
3. Embedded binary std fallback
4. $YXPATH/foo/bar/baz.yx   (global path, reserved)
```

**Single-File Mode Rules**:

- There is no concept of project-level dependencies; std comes directly from global; the global
  standard library path is bound to the compiler version: `<install-dir>/yx/<version>/std/`
- Single-file mode never reads `.yaoxiang/` (no vendor concept)

### Standard Library Installation Directory Structure

#### Global Standard Library

```
<yaoxiang-install-dir>/
├── yx/                          # YaoXiang language directory
│   ├── 1.0.1/                   # version directory
│   │   ├── std/
│   │   │   ├── test.yx          # pure YaoXiang standard library module
│   │   │   ├── math.yx          # future self-hosted module
│   │   │   └── ...
│   │   └── ...
│   └── 1.1.0/
│       └── std/
│           └── ...
└── bin/
    └── yaoxiang                 # compiler binary
```

#### Project-Level Standard Library

> **2026-09-15 Resolution: No separate `.yaoxiang/std/` directory.** **2026-09-28 Revision: std is
> not packaged** (reasoning at end of "Project Mode Rules")—std remains embedded binary + RFC-037
> interface file directory; `add std@<ver>` is unavailable; `std.*` is reserved and not shadowable.
> The directory mutual-exclusion conclusion (no separate `.yaoxiang/std/`) still holds.

```
my-project/
├── yaoxiang.toml
├── yaoxiang.lock
├── .yaoxiang/
│   └── vendor/
│       ├── std-1.0.1/           # std as a normal vendor package
│       └── foo-1.2.3/
├── src/
│   └── main.yx
```

**Design Highlights**:

- Embedded binary as a compatibility layer: before the file-system standard library is fully landed,
  std modules are provided via embedded binary first
- Version directory isolation: `yx/<version>/std/` allows different standard library versions to
  coexist without interfering with each other
- std uses the same mechanism as ordinary dependencies (add/lock/vendor), with no special directory
  and no special lookup layer
- Single-file mode falls back to global std; when vendor exists, std likewise comes from the
  embedded binary / interface directory (2026-09-28 revision: std is not packaged, not bundled with
  vendor)

### Core Data Structures

```rust
// Dependency source (extensible)
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> },
    GitHub { owner: String, repo: String, ref_: GitRef },  // GitHub-native
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
    Workspace { member: String },  // workspace member reference
}

// Resolved dependency (2026-09-15 resolution: integrity uses only a single integrity field in format "sha256-<hex>", no duplicate checksum)
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
}

// Build strategy
enum BuildStrategy {
    None,          // pure .yx package
    Cargo,         // invoke cargo build
    Cmake,         // invoke cmake
    Custom,        // execute build.yx script
    Precompiled,   # use prebuilt artifacts directly
}
```

### CLI Command Design

Adopt a unified scheme that integrates the compiler, package manager, and REPL into a single CLI
tool:

#### Single-File Mode vs Project Mode

| Command                   | Single-File | Project Mode | Description              |
| ------------------------- | ----------- | ------------ | ------------------------ |
| `yaoxiang run <file>`     | ✅          | ✅           | Run file / project entry |
| `yaoxiang build`          | ❌          | ✅           | Build project            |
| `yaoxiang build <file>`   | ✅          | ✅           | Build a single file      |
| `yaoxiang init <name>`    | ❌          | ✅           | Create a project         |
| `yaoxiang add <dep>`      | ❌          | ✅           | Add dependency           |
| `yaoxiang update`         | ❌          | ✅           | Update dependencies      |
| `yaoxiang fmt`            | ✅          | ✅           | Format                   |
| `yaoxiang check`          | ✅          | ✅           | Type check               |
| `yaoxiang` (no arguments) | ✅          | ✅           | Enter REPL directly      |

#### Command Details

| Command                            | Function                                                                  | Example                                                                              |
| ---------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `yaoxiang`                         | Enter REPL directly                                                       | `yaoxiang`                                                                           |
| `yaoxiang run <file>`              | Run single file / project                                                 | `yaoxiang run main.yx`                                                               |
| `yaoxiang init <name>`             | Create a new project                                                      | `yaoxiang init my-app`                                                               |
| `yaoxiang build`                   | Build project                                                             | `yaoxiang build`                                                                     |
| `yaoxiang build <file>`            | Build a single file                                                       | `yaoxiang build foo.yx`                                                              |
| `yaoxiang add <dep>`               | Add dependency                                                            | `yaoxiang add foo`                                                                   |
| `yaoxiang add -D <dep>`            | Add dev dependency                                                        | `yaoxiang add -D test`                                                               |
| `yaoxiang rm <dep>`                | Remove dependency                                                         | `yaoxiang rm foo`                                                                    |
| `yaoxiang update`                  | Update all dependencies                                                   | `yaoxiang update`                                                                    |
| `yaoxiang update foo`              | Update specified dependency                                               | `yaoxiang update foo`                                                                |
| `yaoxiang install`                 | Install all dependencies                                                  | `yaoxiang install`                                                                   |
| `yaoxiang list`                    | List dependencies                                                         | `yaoxiang list`                                                                      |
| `yaoxiang outdated`                | Check outdated dependencies                                               | `yaoxiang outdated`                                                                  |
| `yaoxiang fmt`                     | Format code                                                               | `yaoxiang fmt`                                                                       |
| `yaoxiang check`                   | Type check                                                                | `yaoxiang check`                                                                     |
| `yaoxiang clean`                   | Clean build artifacts                                                     | `yaoxiang clean`                                                                     |
| `yaoxiang task <name>`             | Run custom task                                                           | `yaoxiang task lint`                                                                 |
| `yaoxiang publish`                 | Publish package to Registry                                               | Deferred: official Registry indefinitely deferred; bare publish errors with guidance |
| `yaoxiang publish --dry-run`       | Validate + pack `.yxpkg` to `target/yxpkg/`                               | `yaoxiang publish --dry-run`                                                         |
| `yaoxiang publish --github`        | Publish as GitHub Release (`.yxpkg` asset; requires tag to already exist) | `yaoxiang publish --github`                                                          |
| `yaoxiang yank <pkg>@<ver>`        | Delete a published version (irrecoverable)                                | Deferred: with official Registry                                                     |
| `yaoxiang login --registry <url>`  | Registry authentication                                                   | Deferred: with official Registry (GitHub side currently uses `$YX_GITHUB_TOKEN`)     |
| `yaoxiang login --github`          | GitHub authentication                                                     | Deferred: same as above                                                              |
| `yaoxiang logout --registry <url>` | Log out                                                                   | Deferred: same as above                                                              |
| `yaoxiang cache clean`             | Clean global cache                                                        | `yaoxiang cache clean`                                                               |
| `yaoxiang workspace <cmd>`         | Workspace operations                                                      | `yaoxiang workspace list`                                                            |

#### Command Constraint Description

```bash
# Single-file mode: no yaoxiang.toml required
yaoxiang run hello.yx   # ✅ works normally
yaoxiang add foo        # ❌ error: not a project directory

# Project mode: yaoxiang.toml required
cd my-project
yaoxiang run main.yx    # ✅ run entry file
yaoxiang build          # ✅ build project
yaoxiang add foo        # ✅ add dependency
```

### Backward Compatibility

- ✅ The existing `use` syntax is fully retained
- ✅ The existing module resolution logic is unchanged
- ✅ The new `.yaoxiang/vendor` directory does not affect existing projects

### Global Cache

All downloaded dependencies are cached to `~/.yaoxiang/cache/`; the project vendor directory is
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

- Registry packages: version number is immutable, never expires
- Git dependencies: cached by tag/rev; if the tag is unchanged, it does not expire
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

- Environment variables take precedence: `$YX_GITHUB_TOKEN`, `$YX_REGISTRY_TOKEN`
- Tokens are never written to `yaoxiang.toml` or `yaoxiang.lock`
- File permission 600

### Yank Semantics

`yaoxiang yank foo@1.2.3` performs **deletion + version-number lockdown**:

- The package is completely deleted and irrecoverable
- The version number is permanently occupied and cannot be republished under the same version number
- Projects that already have a lockfile referencing that version will error and need to upgrade
- **Safety purpose**: to prevent npm-style supply chain attacks (where an attacker seizes a vacated
  version number to inject malicious code)

### Registry Protocol

See [RFC-014a: Registry Protocol Specification](../accepted/014a-registry-protocol.md) for details.

Core design: open protocol + adapter layer. The official Registry is primary, GitHub Release/main
branch is secondary, and custom Registries are supported.

### Build System

See [RFC-014b: Build System and Binary Distribution](../accepted/014b-build-system.md) for details.

Core design: declarative `[build]` configuration, precompiled-first/source-fallback, supports
cargo/cmake/custom strategies.

### Workspace

See [RFC-014c: Workspace Support](../accepted/014c-workspace.md) for details.

Core design: dictionary-form members declaration, shared lockfile, path dependencies, Cargo
workspace integration.

## Trade-offs

### Advantages

- Unified import syntax; users don't need to care about dependency sources
- Deterministic builds; the lock file ensures build consistency
- Offline support; offline development is possible after downloading locally
- Source trait makes future extension easy

### Disadvantages

- Requires additional storage space (`.yaoxiang/vendor` directory)
- Version conflicts require manual resolution by the user

## Alternatives

| Option                         | Why not chosen                                     |
| ------------------------------ | -------------------------------------------------- |
| Real-time GitHub access        | Security and cache reuse are hard to guarantee     |
| Global cache ($HOME/.yaoxiang) | Poor isolation, complex version conflicts          |
| Registry-only support          | GitHub is the dominant code hosting platform today |

## Implementation Strategy

### Phase Breakdown

| Phase         | Content                                                                                                             | Status                                                  |
| ------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| **Phase 1**   | toml parsing, local dependencies, lock generation, basic algorithms                                                 | ✅ Complete                                             |
| **Phase 2**   | GitHub support, `.yaoxiang/vendor` management, download tools                                                       | ✅ Complete                                             |
| **Phase 3**   | Global cache, semver crate replacement, CLI polish                                                                  | ✅ Complete                                             |
| **Phase 3.5** | Source distribution enum-ization + native async (014a resolution 4, no async-trait)                                 | ✅ Complete                                             |
| **Phase 4**   | GitHub adapter, `.yxpkg` packaging, publish --github (RFC-014a reduced scope; official Registry/auth/yank deferred) | ✅ Complete                                             |
| **Phase 5**   | Build system, prebuilt binaries (RFC-014b)                                                                          | ✅ Complete                                             |
| **Phase 6**   | Workspace support (RFC-014c)                                                                                        | ✅ 6a-c + member management + 6d complete (6e deferred) |

**Execution order adjustment (2026-09-15)**: `3 → 3.5 → 6 → 4 → 5`.

- Workspace (Phase 6) is moved ahead of the build system—it doesn't depend on the network or build
  system (purely local path resolution + shared lockfile), and offers the most direct benefit for
  multi-package development.
- Phase 4 scope reduced: **the official Registry server and auth/yank are indefinitely deferred**;
  first deliver the GitHub Release/Git adapter + `.yxpkg` packaging + `publish --github`. Ecosystem
  cold-start only needs the git/GitHub channels (Go was the same early on), and the operations and
  governance cost of a Registry server is pure liability at a stage with no third-party packages.
- Consequent constraint: before the official Registry goes online,
  `yaoxiang add <bare-package-name>` is unavailable; adding a dependency requires an explicit source
  (`--git` / `--path`).

**Phase 3 implementation notes (2026-09-28)**:

- Phase 3.5 implementation notes (2026-09-29): Source distribution follows resolution 4 with a
  closed `AnySource` four-source set (Local/Git/Registry/GitHub; the latter two are Phase 4
  placeholders); `Source` trait's resolve/download are native async fn in trait, driven at the
  command layer by `futures::executor::block_on` (no runtime; Git subprocess keeps `std::process`;
  Phase 4 will swap in a real executor when adopting reqwest); adds `futures` dependency
  (wasm32-compatible). Parallel download for install is deferred until a real runtime is introduced.
- Phase 4 implementation notes (2026-09-29, commits 234dfea5/e1873133/5ffee636):
  - **GitHub adapter** (4a): github.com git dependencies are routed to `GitHubSource`—version
    resolution goes through the REST API (releases endpoint, falling back to tags if empty);
    downloads prefer the Release's `.yxpkg` asset (unpack and verify before entering
    `cache/github/`, then copy to vendor), falling back to git clone if no asset (SourceKind
    honestly reports `Git`). API access uses exponential backoff (1s/2s/4s, Retry-After preferred) +
    **ETag conditional request caching** (`cache/github/*.etag|body`, 304 doesn't count against
    GitHub rate quota); 403 with `x-ratelimit-remaining: 0` is identified as primary rate limiting
    and not retried.
  - **`.yxpkg` package format** (4b): tar.gz + `SHA256SUMS` manifest (coreutils double-space
    format), deterministic packaging (entry sorting, mtime/uid/gid zeroed); unpacking enforces
    verification (missing/tampered manifest, files outside manifest, path traversal, total
    uncompressed size limit all error); 20 MiB content size cap (resolution 7). Exclusions use a
    blacklist (`.git`/`.yaoxiang`/`target`/`node_modules`/`*.yxpkg` etc.) rather than a
    whitelist—`[exports]` allows files outside src/ into the export surface, and a whitelist would
    silently drop them.
  - **`publish`** (4c): bare `publish` errors with guidance (Registry deferred); `--dry-run`
    performs "validate (description required) → pack → SHA-256"; `--github` then checks for
    duplicate Release → verifies the tag exists (Cargo-style semantics: tagging is the user's job) →
    creates the Release → uploads the asset. The target repo takes `[package].repository`, falling
    back to `git remote origin`; authentication reads `$YX_GITHUB_TOKEN` (credentials.toml lands
    with the official Registry). HTTP stack is reqwest (rustls) + a package-management-owned tokio
    current_thread runtime (`package::runtime::drive`), removing the `futures` dependency.
  - Pre-publish test runs (014a validation checklist step 3) are wired in with Phase 5 (03929ffa).
- Phase 5 implementation notes (2026-09-30, feat/rfc014):
  - **Install decision-tree wiring** (around b6a5b98f and several commits): after `install/update`
    downloads dependencies, packages with `[build]`/`[binaries]` go through
    `build::run_install_build`—precompiled preferred (whole-package SHA-256 + safe unpack) → headers
    (explicit error before 026b) → strategy execution (cargo real implementation / cmake pending /
    custom trust gate). Packages without a build declaration pass through at zero cost.
  - **cargo strategy**: `[build.cargo]` assembles the command + merges platform overrides; scratch
    space is isolated to `.yaoxiang/build/` via `CARGO_TARGET_DIR`, and FFI artifacts are copied
    into the vendor `build/native/<triple>/`; vendor integrity semantics are explicitly the
    source-tree integrity (`build/` is not included in checksums).
  - **Trust gate** (014b resolution 1): trust records live in user config `[trust] build-scripts`;
    `--trust` grants and persists; non-interactive environments deny by default.
  - Pre-publish tests run by default (RFC-036 discovery mechanism); `--no-test` skips.
  - cmake execution and yx-bindgen generator (RFC-026b) pending; see 014b implementation notes for
    the rest.

- The global cache first covers the **git channel** (`cache/git/<url>-<tag|rev|commit>/`, branches
  are resolved to commits via `ls-remote` and cached with pointer files for offline fallback);
  `cache/registry/` and `cache/binaries/` are reserved as directories. Vendor copies strip `.git`;
  directory names are based on the real version detected by the dependency manifest
  (vendor/lock/cleanup are all sourced from the same place).
- `semver` and `sha2` crates replace hand-written implementations per the dependency table;
  `is_compatible` moves from 100k-iteration enumeration to range-intersection checks.
- The CLI adds `outdated` / `clean` / `cache clean`, and gives `add` the explicit-source flags
  `--git` / `--path` (the above-constraint landing). `clean` not only trims `.yaoxiang/build/` but
  also prunes orphan packages in vendor that aren't referenced by the lock.
- The cache `[cache] dir` config merges into the existing `~/.config/yaoxiang/config.toml`
  user-config system (the `~/.yaoxiang/config.toml` from the RFC draft never existed), and the
  default cache data location remains `~/.yaoxiang/cache`.

### Dependency Relations

- No prerequisites
- Needs integration with `ModuleGraph` (`middle/passes/module/`)

### Risks

| Risk                                       | Mitigation                                                  |
| ------------------------------------------ | ----------------------------------------------------------- |
| Dependency resolution algorithm complexity | Implement simple version first, then add conflict detection |
| Unstable Git downloads                     | Retry and cache mechanisms                                  |
| Performance issues                         | Lazy loading, incremental resolution                        |

## Open Questions

- [x] `dev-dependencies` conditional compilation syntax? → Handled uniformly by the RFC-014b build
      system
- [x] Integrity verification algorithm (SHA-256 / BLAKE3)? → SHA-256
- [x] Package naming convention (do namespaces like `@org/pkg` work)? → Flat package names
      initially, no namespace; `@org/pkg` reserved (2026-09-15 resolution)
- [x] Registry API versioning strategy? → URL path `/api/v1/` + response header carrying protocol
      version; breaking changes bump to v2 and coexist (2026-09-15 resolution, see RFC-014a)
- [ ] `excludes` to skip specific files from being downloaded?

---

## Dependencies (to be added to Cargo.toml)

| Purpose             | crate            | Description                      |
| ------------------- | ---------------- | -------------------------------- |
| Semantic versioning | `semver`         | Replace hand-written parser      |
| HTTP client         | `reqwest`        | Registry communication (Phase 4) |
| SHA-256             | `sha2`           | Integrity verification           |
| Compression         | `flate2` + `tar` | Package format handling          |

---

## References

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
