---
title: 'RFC-014: Package Management System Design'
status: 'Accepted'
author: 'Chenxu'
created: '2026-02-12'
updated: '2026-10-03'
group: 'rfc-014' # This RFC is the overview of the package management system; sub-RFCs: 014a/014b/014c
issue: '#88'
impl: '100%'
impl_status: 'complete'
---

# RFC-014: Package Management System Design (Overview)

> **Sub-RFCs:**
>
> - [RFC-014a: Registry Protocol Specification](014a-registry-protocol.md)
> - [RFC-014b: Build System and Binary Distribution](014b-build-system.md)
> - [RFC-014c: Workspace Support](014c-workspace.md)

## Summary

Design the package management system for the YaoXiang language, supporting Semantic Versioning,
local and GitHub dependencies, a unified import syntax, the `yaoxiang.toml` configuration file, and
the `yaoxiang.lock` lock file.

## Motivation

### Why is this feature/change needed?

Package management is the infrastructure foundation of a modern programming language ecosystem. The
current YaoXiang language lacks:

- A dependency declaration mechanism
- Version management capabilities
- A standard distribution channel

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
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← Extensible Sources
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

**Extension Mechanism**: Adding a new Source type only requires implementing a trait, with no need
to modify the resolution engine.

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

> **2026-09-15 Decision: Mutually exclusive semantics for core package source (Python venv / Node
> node_modules style).** The original "5-layer cascading lookup chain" description is
> deprecated—vendor and global are **mutually exclusive** as the single core package source; std is
> embedded in the core source; local modules may override everything else.

**Core Package Source Determination (mutually exclusive, never mixed)**:

- If the project has `.yaoxiang/vendor/` → **vendor is the sole core package source**. All `use` for
  non-local modules resolves only from vendor; if a package is missing from vendor, an error is
  raised directly (prompting `yaoxiang install`), with **no global fallback**.
- Otherwise → **global is the core package source** (install-dir std + global cache).

There is no "fall back to global cache per-package when missing from vendor" cascading—mixing the
two sources is precisely the root cause of version drift and "works on my machine" issues.

#### Project Mode (with yaoxiang.toml)

```
use foo.bar.baz;

Lookup order:
1. ./src/foo/bar/baz.yx     Local module — highest priority, may override same-named modules in core source
2. <core source>/foo/bar/baz.yx
   · vendor mode: .yaoxiang/vendor/<pkg>-<ver>/src/foo/bar/baz.yx (2026-09-28 revision: std is not inside vendor, see project mode rules)
   · global mode:  <install-dir>/yx/<ver>/std/foo/bar/baz.yx + ~/.yaoxiang/cache/...
3. std.* dedicated fallback: embedded binary (only std.* namespace; 2026-09-28 revision: formalized mechanism, see project mode rules)
4. Error (module does not exist); in vendor mode, missing package prompts `yaoxiang install`
```

**Project Mode Rules**:

- When vendor exists but is inconsistent with `yaoxiang.lock`, `run`/`build` reports an error and
  prompts `yaoxiang install` (Node semantics: no silent auto-install)
- When a local module shadows a same-named module in the core source, a W-level diagnostic is
  emitted by default to flag the shadowing (`--deny-shadowing` can upgrade it to an error)
- `path` dependencies are treated as an extension of local modules, resolved directly by path, not
  through the core package source

> **2026-09-28 Revision: std is not packaged (overturning the std-as-package portion of the
> 2026-09-15 decision).** The embedded binary std transitions from "transitional fallback" to a
> **formal mechanism**: std is ABI-coupled to the compiler (native layer/runtime builtins must match
> VM layout); locking std by package creates obscure breakage of the "std-1.0.1 + compiler 1.0.2"
> variety; the correct fix for std drift is project-level toolchain locking (to be discussed
> separately if needed), rather than a std package. The Go/Rust/Python precedents
> agree—language-bundled std follows the toolchain. `yaoxiang add std@<ver>` is unavailable; `std.*`
> is a reserved namespace, and local modules cannot shadow it (the embedded std has no file form, so
> shadowing is impossible); the `.yaoxiang/vendor/std/` interface file directory (LSP lookup chain
> level 1) from RFC-037 is retained, continuing to serve the "view source" purpose. Incidental fix:
> the diagnostic scope for local module shadowing covers dependency packages only, with no longer an
> std.* special-case wording.

#### Single-File Mode (without yaoxiang.toml)

```
use foo.bar.baz;

Lookup order:
1. ./src/foo/bar/baz.yx     Local module
2. Global core package source: <install-dir>/yx/<version>/std/foo/bar/baz.yx
3. Embedded binary std fallback
4. $YXPATH/foo/bar/baz.yx   (Global path, reserved)
```

**Single-File Mode Rules**:

- No project-level dependency concept; std comes directly from global; the global standard library
  path is bound to the compiler version: `<install-dir>/yx/<version>/std/`
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

> **2026-09-15 Decision: No separate `.yaoxiang/std/` directory.** **2026-09-28 Revision: std is not
> packaged** (reasoning at the end of "Project Mode Rules")—std remains embedded binary + RFC-037
> interface file directory, `add std@<ver>` is unavailable, `std.*` is reserved and non-shadowable.
> The mutually exclusive directory conclusion (no separate `.yaoxiang/std/`) remains in force.

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

- Embedded binary as a compatibility layer: before the filesystem-based standard library fully
  lands, std modules are first provided via the embedded binary
- Version directory isolation: `yx/<version>/std/` lets different versions of the standard library
  coexist without interfering with each other
- std uses the same mechanism as ordinary dependencies (add/lock/vendor); no special directory, no
  special lookup layer
- Single-file mode falls back to global std; when vendor exists, std also comes from the embedded
  binary / interface directory (2026-09-28 revision: std is not packaged, does not follow vendor)

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
    Workspace { member: String },  // Workspace member reference
}

// Resolved dependency (2026-09-15 decision: integrity uses only the single `integrity` field, format "sha256-<hex>", no duplicate checksum)
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
    Precompiled,   // Use precompiled artifacts directly
}
```

### CLI Command Design

A unified approach is adopted, integrating the compiler, package manager, and REPL into a single CLI
tool:

#### Single-File Mode vs. Project Mode

| Command                 | Single-File | Project Mode | Description            |
| ----------------------- | ----------- | ------------ | ---------------------- |
| `yaoxiang run <file>`   | ✅          | ✅           | Run file/project entry |
| `yaoxiang build`        | ❌          | ✅           | Build project          |
| `yaoxiang build <file>` | ✅          | ✅           | Build a single file    |
| `yaoxiang init <name>`  | ❌          | ✅           | Create project         |
| `yaoxiang add <dep>`    | ❌          | ✅           | Add dependency         |
| `yaoxiang update`       | ❌          | ✅           | Update dependencies    |
| `yaoxiang fmt`          | ✅          | ✅           | Format                 |
| `yaoxiang check`        | ✅          | ✅           | Type check             |
| `yaoxiang` (no args)    | ✅          | ✅           | Enter REPL directly    |

#### Command Reference

| Command                            | Function                                                          | Example                                                                                        |
| ---------------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `yaoxiang`                         | Enter REPL directly                                               | `yaoxiang`                                                                                     |
| `yaoxiang run <file>`              | Run a single file/project                                         | `yaoxiang run main.yx`                                                                         |
| `yaoxiang init <name>`             | Create a new project                                              | `yaoxiang init my-app`                                                                         |
| `yaoxiang build`                   | Build the project                                                 | `yaoxiang build`                                                                               |
| `yaoxiang build <file>`            | Build a single file                                               | `yaoxiang build foo.yx`                                                                        |
| `yaoxiang add <dep>`               | Add dependency                                                    | `yaoxiang add foo`                                                                             |
| `yaoxiang add -D <dep>`            | Add dev dependency                                                | `yaoxiang add -D test`                                                                         |
| `yaoxiang rm <dep>`                | Remove dependency                                                 | `yaoxiang rm foo`                                                                              |
| `yaoxiang update`                  | Update all dependencies                                           | `yaoxiang update`                                                                              |
| `yaoxiang update foo`              | Update a specific dependency                                      | `yaoxiang update foo`                                                                          |
| `yaoxiang install`                 | Install all dependencies                                          | `yaoxiang install`                                                                             |
| `yaoxiang list`                    | List dependencies                                                 | `yaoxiang list`                                                                                |
| `yaoxiang outdated`                | Check for outdated dependencies                                   | `yaoxiang outdated`                                                                            |
| `yaoxiang fmt`                     | Format code                                                       | `yaoxiang fmt`                                                                                 |
| `yaoxiang check`                   | Type check                                                        | `yaoxiang check`                                                                               |
| `yaoxiang clean`                   | Clean build artifacts                                             | `yaoxiang clean`                                                                               |
| `yaoxiang task <name>`             | Run a custom task                                                 | `yaoxiang task lint`                                                                           |
| `yaoxiang publish`                 | Publish package to Registry                                       | Deferred: official Registry deferred indefinitely; bare publish reports an error with guidance |
| `yaoxiang publish --dry-run`       | Validate + pack `.yxpkg` into `target/yxpkg/`                     | `yaoxiang publish --dry-run`                                                                   |
| `yaoxiang publish --github`        | Publish as GitHub Release (`.yxpkg` asset; requires tag to exist) | `yaoxiang publish --github`                                                                    |
| `yaoxiang yank <pkg>@<ver>`        | Yank a published version (irreversible)                           | Deferred: along with official Registry                                                         |
| `yaoxiang login --registry <url>`  | Registry authentication                                           | Deferred: along with official Registry (GitHub side currently uses `$YX_GITHUB_TOKEN`)         |
| `yaoxiang login --github`          | GitHub authentication                                             | Deferred: same as above                                                                        |
| `yaoxiang logout --registry <url>` | Log out                                                           | Deferred: same as above                                                                        |
| `yaoxiang cache clean`             | Clean global cache                                                | `yaoxiang cache clean`                                                                         |
| `yaoxiang workspace <cmd>`         | Workspace operations                                              | `yaoxiang workspace list`                                                                      |

#### Command Constraint Notes

```bash
# Single-file mode: no yaoxiang.toml required
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
- ✅ New `.yaoxiang/vendor` directory does not affect existing projects

### Global Cache

All downloaded dependencies are cached in `~/.yaoxiang/cache/`; the project's vendor directory is
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
- Git dependencies: cached by tag/rev; if the tag is unchanged, not invalidated
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

`yaoxiang yank foo@1.2.3` performs **deletion + permanent version lockout**:

- The package is permanently removed, irrecoverable
- The version number is permanently occupied; the same version number cannot be republished
- Existing lockfiles referencing this version will error and require an upgrade
- **Security purpose**: to prevent npm-style supply-chain attacks (an attacker squatting on a
  deleted version number to inject malicious code)

### Registry Protocol

See [RFC-014a: Registry Protocol Specification](014a-registry-protocol.md) for details.

Core design: open protocol + adapter layer. The official Registry is primary, with GitHub Release /
main branch as fallback, and support for custom Registries.

### Build System

See [RFC-014b: Build System and Binary Distribution](014b-build-system.md) for details.

Core design: declarative `[build]` configuration, precompiled priority / source fallback, supports
cargo / cmake / custom strategies.

### Workspaces

See [RFC-014c: Workspace Support](014c-workspace.md) for details.

Core design: dictionary-style `members` declaration, shared lockfile, path dependencies, Cargo
workspace integration.

## Trade-offs

### Advantages

- Unified import syntax; users don't need to worry about dependency source
- Deterministic builds; lock file guarantees build consistency
- Offline support; downloadable locally for offline development
- Source trait is easy to extend later

### Disadvantages

- Requires extra storage space (`.yaoxiang/vendor` directory)
- Version conflicts need to be resolved manually by the user

## Alternatives

| Option                              | Why Not Chosen                                |
| ----------------------------------- | --------------------------------------------- |
| Live GitHub access                  | Security and cache reuse hard to guarantee    |
| Global cache only ($HOME/.yaoxiang) | Poor isolation, complex version conflicts     |
| Registry only                       | GitHub is the current mainstream code hosting |

## Implementation Strategy

### Phased Plan

| Phase         | Content                                                                                                               | Status                                                   |
| ------------- | --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| **Phase 1**   | toml parsing, local dependencies, lock generation, basic algorithms                                                   | ✅ Completed                                             |
| **Phase 2**   | GitHub support, `.yaoxiang/vendor` management, download tools                                                         | ✅ Completed                                             |
| **Phase 3**   | Global cache, semver crate replacement, CLI polishing                                                                 | ✅ Completed                                             |
| **Phase 3.5** | Source dispatch via enum + native async (RFC-014a decision 4, no async-trait)                                         | ✅ Completed                                             |
| **Phase 4**   | GitHub adapter, `.yxpkg` packaging, `publish --github` (RFC-014a reduced scope; official Registry/auth/yank deferred) | ✅ Completed                                             |
| **Phase 5**   | Build system, precompiled binaries (RFC-014b)                                                                         | ✅ Completed                                             |
| **Phase 6**   | Workspace support (RFC-014c)                                                                                          | ✅ 6a-c + member management + 6d completed (6e deferred) |

**Execution Order Adjustment (2026-09-15)**: `3 → 3.5 → 6 → 4 → 5`.

- Workspace (Phase 6) is moved ahead of the build system—it does not depend on the network or build
  system (purely local path resolution + shared lockfile), and yields the most direct benefit for
  multi-package development.
- Phase 4 scope reduction: **the official Registry server and auth/yank are deferred indefinitely**;
  we first deliver the GitHub Release / Git adapter layer + `.yxpkg` packaging + `publish --github`.
  Cold-starting the ecosystem only needs the git / GitHub channel (same pattern as Go's early days);
  the operational and governance cost of a Registry server is pure liability at the
  no-third-party-packages stage.
- Accompanying constraint: until the official Registry goes live, `yaoxiang add <bare package name>`
  is unavailable; adding dependencies requires an explicit source (`--git` / `--path`).

**Phase 3 Landing Notes (2026-09-28)**:

- Phase 3.5 landing notes (2026-09-29): Source dispatch follows decision 4 with `AnySource`, a
  closed four-source set (Local / Git / Registry / GitHub, with the latter two as Phase 4
  placeholders); `Source` trait's `resolve`/`download` are native `async fn in trait`, driven at the
  command layer by `futures::executor::block_on` (no runtime; Git subprocess remains `std::process`;
  Phase 4 will swap in a real executor when reqwest is wired up); the `futures` dependency is added
  (wasm32-compatible). Parallel install downloads are deferred until a real runtime is introduced.
- Phase 4 landing notes (2026-09-29, commits 234dfea5 / e1873133 / 5ffee636):
  - **GitHub adapter** (4a): github.com git dependencies are routed to `GitHubSource`—version
    resolution goes through the REST API (releases endpoint, falling back to tags if empty),
    downloads prefer the Release `.yxpkg` asset (unpacked and verified into `cache/github/` then
    copied to vendor), falling back to git clone if no asset exists (`SourceKind` truthfully reports
    `Git`). API access uses exponential backoff (1s/2s/4s, Retry-After first) + **ETag conditional
    request caching** (`cache/github/*.etag|body`; 304 doesn't count against the GitHub rate limit);
    403 + `x-ratelimit-remaining: 0` is recognized as the primary rate limit and not retried.
  - **`.yxpkg` package format** (4b): tar.gz + `SHA256SUMS` manifest (coreutils double-space
    format), deterministic packaging (sorted entries, zeroed mtime/uid/gid); unpacking enforces
    verification (missing manifest / tampered manifest / files outside manifest / path escape /
    exceeded total uncompressed size all error); content total cap of 20 MiB (decision 7).
    Exclusions use a blacklist (`.git` / `.yaoxiang` / `target` / `node_modules` / `*.yxpkg` etc.)
    rather than a whitelist—a whitelist would silently miss things, while `[exports]` allows pulling
    files outside src/ into the export surface.
  - **`publish`** (4c): bare `publish` reports an error with guidance (Registry deferred);
    `--dry-run` completes "validation (description required) → pack → SHA-256"; `--github` then
    dedupes Releases → verifies the tag exists (Cargo-style semantics: tagging is the user's job) →
    creates the Release → uploads the asset. The target repository is taken from
    `[package].repository`, falling back to `git remote origin`; authentication reads
    `$YX_GITHUB_TOKEN` (credentials.toml lands with the official Registry). HTTP stack is reqwest
    (rustls) + the package manager's own tokio current_thread runtime (`package::runtime::drive`),
    and the `futures` dependency is removed.
  - Pre-publish test runs (RFC-014a validation list step 3) are wired up in Phase 5 (03929ffa).
- Phase 5 landing notes (2026-09-30, feat/rfc014):
  - **Install decision-tree wiring** (around b6a5b98f across multiple commits): `install`/`update`
    after downloading dependencies runs `build::run_install_build` for packages with `[build]` /
    `[binaries]`—precompiled first (whole-package SHA-256 + safe unpack) → headers (clear error
    before 026b) → strategy execution (cargo real implementation / cmake pending / custom trust
    gate). Packages with no build declaration pass through at zero cost.
  - **cargo strategy**: `[build.cargo]` assembles commands + platform overrides are merged; scratch
    builds are isolated to `.yaoxiang/build/` via `CARGO_TARGET_DIR`, and FFI artifacts are copied
    into vendor's `build/native/<triple>/`; vendor integrity semantics are clearly defined as
    source-tree integrity (`build/` is not included in the checksum).
  - **Trust gate** (RFC-014b decision 1): trust records live in user config `[trust] build-scripts`;
    `--trust` grants and persists; non-interactive environments deny by default.
  - publish's pre-publish tests run by default (RFC-036 discovery mechanism); `--no-test` skips
    them.
  - cmake execution and the yx-bindgen generator (RFC-026b) are pending; see the 014b landing notes
    for the rest.

- The global cache first covers the **git channel** (`cache/git/<url>-<tag|rev|commit>/`; branches
  resolve to a commit via `ls-remote` and a pointer file is written for offline fallback);
  `cache/registry/` and `cache/binaries/` are reserved as directories. The vendor copy strips
  `.git`; directory names use the real version detected from the dependency manifest (vendor / lock
  / cleanup share a single source of truth).
- `semver` and `sha2` crates replace hand-written implementations per the dependency table;
  `is_compatible` is changed from 100k-iteration enumeration to interval-intersection judgment.
- The CLI adds `outdated` / `clean` / `cache clean`, and `add` gains explicit `--git` / `--path`
  sources (the landing of the above constraint). `clean` trims both `.yaoxiang/build/` and stale
  packages in vendor that are not referenced by lock.
- The `[cache] dir` configuration is folded into the existing `~/.config/yaoxiang/config.toml`
  user-config system (the `~/.yaoxiang/config.toml` referenced when this RFC was drafted never
  existed); the default cache data location remains `~/.yaoxiang/cache`.

### Dependencies

- No prerequisites
- Needs to integrate with `ModuleGraph` (`middle/passes/module/`)

### Risks

| Risk                             | Mitigation                                                     |
| -------------------------------- | -------------------------------------------------------------- |
| Dependency resolution is complex | Implement a simple version first, add conflict detection later |
| Unstable Git downloads           | Retry and cache mechanisms                                     |
| Performance issues               | Lazy loading, incremental resolution                           |

## Open Questions

- [x] `dev-dependencies` conditional compilation syntax? → Handled uniformly by the build system in
      RFC-014b
- [x] Integrity verification algorithm (SHA-256 / BLAKE3)? → SHA-256
- [x] Package naming convention (support namespaces like `@org/pkg`)? → Flat package names
      initially, no namespace; `@org/pkg` reserved (2026-09-15 decision)
- [x] Registry API versioning strategy? → URL path `/api/v1/` + response headers carry the protocol
      version; breaking changes bump to v2 with coexistence (2026-09-15 decision, see RFC-014a)
- [ ] `excludes` to skip specific files from download?

---

## Dependencies (New in Cargo.toml)

| Purpose             | crate            | Description                      |
| ------------------- | ---------------- | -------------------------------- |
| Semantic versioning | `semver`         | Replaces hand-written parser     |
| HTTP client         | `reqwest`        | Registry communication (Phase 4) |
| SHA-256             | `sha2`           | Integrity verification           |
| Compression         | `flate2` + `tar` | Package format handling          |

---

## References

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
