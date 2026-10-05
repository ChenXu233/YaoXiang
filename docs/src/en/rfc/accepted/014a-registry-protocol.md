---
title: 'RFC-014a: Registry Protocol Specification'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-11'
updated: '2026-09-29'
group: 'rfc-014'
---

# RFC-014a: Registry Protocol Specification

> This RFC is a sub-RFC of
> [RFC-014: Package Manager System Design](014-package-manager.md).

## 2026-09-15 Review Resolution

The following resolutions were finalized by the owner on 2026-09-15; the corresponding sections of
the main text are retained as the complete specification for the period "after the official Registry
goes online":

1. **Scope reduction (the most important item in this document)**: The official Registry server,
   authentication (login/logout), and yank are **indefinitely deferred**. Phase 4 actual
   deliverables = GitHub Release/Git adapter layer + Registry trait finalization + `.yxpkg`
   packaging + `publish --github`. Rationale: Cold-starting the ecosystem only needs the git/GitHub
   channel (same model as Go's early days); the operations, account system, and abuse-mitigation
   costs of a Registry server are pure liabilities at the stage with no third-party packages.
2. **Bare package-name add is unavailable**: Before the official Registry goes online,
   `yaoxiang add <bare-package-name>` errors out; adding dependencies requires an explicit source
   (`--git` / `--path`). The default lookup chain in "Source Priority" below takes effect once the
   Registry is online.
3. **Package format normalization**: `.yxpkg` only contains source code
   (`yaoxiang.toml`/`src/`/`build.yx`/`SHA256SUMS`); remove the `build/native/` prebuilt artifact
   directory; binary distribution always goes through RFC-014b's `[binaries]` external links
   (Release/CDN), to avoid package bloat and global cache inflation.
4. **Source distribution implementation**: The built-in four sources (Local/Git/Registry/GitHub)
   form a closed set; the implementation layer uses enum dispatch (to avoid the Send bound of
   dyn-async and the `async-trait` dependency); the `Source` trait definition is retained at the
   semantic layer, and if third-party Sources are opened up in the future, they will be wired in via
   trait objects.
5. **API versioning**: URL path `/api/v1/` + protocol version carried in response headers; breaking
   changes bump to v2 and coexist, no in-place changes.
6. **Rate limiting**: The GitHub adapter layer uses exponential backoff + ETag conditional request
   caching; the Registry-side rate policy is deferred together with the official Registry.
7. **Package size limit**: Source package 20 MiB (initial value, adjustable); the GitHub channel is
   self-managed by the platform.

## Summary

Defines the Registry protocol for the YaoXiang package manager system: open interface design,
official Registry specification, GitHub adapter layer, package publish/retract flow, and
authentication model.

## Motivation

The RFC-014 master document defines the overall architecture of the package manager system, but the
Registry part is only marked as "reserved." Without a Registry protocol, packages cannot be
distributed — this is like designing a shopping cart without a store.

### Current Problems

- `RegistrySource` is stub code (`source/mod.rs:150-203`), `resolve` directly returns the declared
  version, and `download` returns an empty path
- No HTTP client (no `reqwest` dependency)
- No package publish mechanism
- No authentication/authorization

## Proposal

### Core Design: Open Protocol + Adapter Layer

```
┌──────────────────────────────────────────┐
│         yaoxiang publish/install         │  ← CLI layer
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│          Registry Trait                  │  ← Protocol layer (open interface)
│  ┌─────────┬──────────┬────────────┐    │
│  │ .publish│ .search  │ .download  │    │
│  │ .yank   │ .info    │ .versions  │    │
│  └─────────┴──────────┴────────────┘    │
└──────────────────┬───────────────────────┘
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
   ┌─────────┐ ┌────────┐ ┌─────────┐
   │ Official│ │ GitHub │ │ Custom  │
   │Registry │ │Adapter │ │Registry │
   └─────────┘ └────────┘ └─────────┘
```

### Async Architecture Decision

The `Source` trait is uniformly changed to async, fully embracing tokio:

```rust
// Existing (sync) → Changed to (async)
#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String>;
    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage>;
}
```

All implementations (`LocalSource`, `GitSource`, `RegistrySource`) are uniformly changed to async.
The CLI entry is driven via `#[tokio::main]` or `Runtime::block_on`.

**Rationale:**

- The Registry requires HTTP requests; blocking will stall the entire install flow
- Parallel multi-dependency download (`join_all`) significantly boosts install speed
- Git clone is also I/O, and async is more natural
- tokio is already a project dependency

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// Publish a package
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// Delete a published version (irrecoverable, version number locked)
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// Query package info
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// Query list of available versions
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// Search packages
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// Download a specific version
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// Authenticate
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### Source Priority (Default Lookup Chain)

Default lookup order for `yaoxiang add foo` (no flag):

| Priority | Lookup            | Description                                     |
| -------- | ----------------- | ----------------------------------------------- |
| 1        | Global cache      | `~/.yaoxiang/cache/registry/foo-<ver>/`         |
| 2        | Official Registry | Query version → download                        |
| 3        | Failure           | Error out, prompt user to check name or network |

**Explicit Override (bypasses default chain):**

| flag               | Behavior                                                                           |
| ------------------ | ---------------------------------------------------------------------------------- |
| `--git <url>`      | Skip Registry, directly Git clone (prefer Release assets → fallback to tag/branch) |
| `--path <dir>`     | Skip Registry, directly use local path                                             |
| `--registry <url>` | Skip official Registry, use specified Registry                                     |

### Official Registry

The official Registry is similar to crates.io and is the main distribution channel for packages.

**API Endpoints:**

| Endpoint                                 | Method | Description        |
| ---------------------------------------- | ------ | ------------------ |
| `/api/v1/packages/{name}`                | GET    | Query package info |
| `/api/v1/packages/{name}/versions`       | GET    | Query version list |
| `/api/v1/packages/{name}/{version}`      | GET    | Download package   |
| `/api/v1/packages`                       | PUT    | Publish package    |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | Retract version    |
| `/api/v1/search?q={query}`               | GET    | Search packages    |
| `/api/v1/login`                          | POST   | Authenticate       |

### GitHub Integration

When GitHub is used as a package source, a Go-modules-style strategy is adopted:

1. **Prefer Release assets**: Check the GitHub Release page for prebuilt artifacts matching the
   platform
2. **Fallback to main branch**: If no Release, git clone

```toml
[dependencies]
# Basic git dependency
foo = { git = "https://github.com/user/foo" }

# Specify version (match tag)
bar = { git = "https://github.com/user/bar", version = "^1.0.0" }

# Specify branch
baz = { git = "https://github.com/user/baz", branch = "main" }

# Specify commit
qux = { git = "https://github.com/user/qux", rev = "abc123" }

# Private repository (uses GitHub token in credentials.toml)
private = { git = "https://github.com/my-org/private-lib" }
```

### Package Format (.yxpkg)

> 2026-09-15 Resolution: Only contains source code; `build/` prebuilt artifacts removed — binaries
> are always distributed via RFC-014b `[binaries]` external links.

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # Package metadata
├── src/                   # Source code
├── build.yx               # Build script (if any)
└── SHA256SUMS             # Checksums
```

### publish Flow

```bash
# Publish to the official Registry
yaoxiang publish

# Publish to a specified Registry
yaoxiang publish --registry my-company

# Also create a GitHub Release
yaoxiang publish --github

# Dry run
yaoxiang publish --dry-run
```

Pre-publish validation:

1. `yaoxiang.toml` must have `name`, `version`, `description`
2. The version number must not already exist
3. Run tests (optional, skip with `--no-test`)
4. Compute SHA-256 of all files
5. Pack into `.yxpkg` (tar.gz)
6. Upload to the Registry

### yank Semantics

```bash
yaoxiang yank foo@1.2.3
```

**Delete + version number lock:**

- The package is thoroughly deleted and irrecoverable
- The version number is permanently occupied; the same version number cannot be re-published
- Projects that already have a lockfile referencing this version will error and need to upgrade to
  another version
- **Security purpose**: Prevent npm-style supply-chain attacks. Attackers have previously
  re-registered deleted package version numbers to inject malicious code; locking the version number
  on yank completely closes off that path.

### Authentication Model

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**Mapping rule:** `yaoxiang login --registry <url>` matches the `url` field in `[registries.*]` by
URL. If there is no match, a new entry is created (with an auto-generated name, such as `reg-1`).

**Priority:** Environment variable > Configuration file

| Environment Variable | Purpose                                        |
| -------------------- | ---------------------------------------------- |
| `$YX_GITHUB_TOKEN`   | GitHub authentication                          |
| `$YX_REGISTRY_TOKEN` | Registry authentication (for default Registry) |
| `$YX_REGISTRY_URL`   | Default Registry address                       |

**CLI commands:**

```bash
yaoxiang login --registry https://yxreg.example.com   # Match by URL or create new
yaoxiang login --github                                # GitHub OAuth or token
yaoxiang logout --registry https://yxreg.example.com   # Delete matching entry
```

**Security constraints:**

- Tokens are never written to `yaoxiang.toml` or `yaoxiang.lock`
- `credentials.toml` file permission 600
- CI scenarios use environment variables, development scenarios use files

## Detailed Design

### RegistrySource Implementation

Replace the existing stub code (`source/mod.rs:150-203`):

```rust
pub struct RegistrySource {
    client: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl Source for RegistrySource {
    fn name(&self) -> &str { "registry" }
    fn kind(&self) -> SourceKind { SourceKind::Registry }

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String> {
        let url = format!("{}/api/v1/packages/{}/versions", self.base_url, spec.name);
        let versions: Vec<Version> = self.client.get(&url).send().await?.json().await?;
        let req = parse_version_req(&spec.version)?;
        select_best(&req, &versions)
            .map(|v| v.to_string())
            .ok_or(PackageError::DependencyNotFound(spec.name.clone()))
    }

    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage> {
        let version = self.resolve(spec).await?;
        let url = format!("{}/api/v1/packages/{}/{}/download", self.base_url, spec.name, version);
        let bytes = self.client.get(&url).send().await?.bytes().await?;

        // SHA-256 verification
        let actual_hash = sha256_hex(&bytes);
        // ... extract to dest ...

        Ok(ResolvedPackage {
            name: spec.name.clone(),
            version,
            source_kind: SourceKind::Registry,
            source_url: self.base_url.clone(),
            local_path: dest.to_path_buf(),
            checksum: Some(actual_hash),
        })
    }
}
```

### Dependencies

| crate             | Purpose                                                                                       |
| ----------------- | --------------------------------------------------------------------------------------------- |
| `reqwest`         | HTTP client                                                                                   |
| `sha2`            | SHA-256 verification                                                                          |
| `flate2` + `tar`  | Package format processing                                                                     |
| ~~`async-trait`~~ | Deprecated — Resolution 4 adopts enum dispatch + native async fn in trait, no this dependency |

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("package '{0}' does not exist")]
    PackageNotFound(String),

    #[error("version '{0}' does not exist")]
    VersionNotFound(String),

    #[error("version '{0}' is already taken")]
    VersionAlreadyExists(String),

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("SHA-256 verification failed: expected {expected}, actual {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("insufficient permission: {0}")]
    Forbidden(String),
}
```

## Trade-offs

### Pros

- Open protocol, not tied to a specific server
- GitHub as a lightweight distribution channel lowers the entry barrier
- Version-number-locked security model
- Prebuilt-first install strategy

### Cons

- The official Registry requires independent operations
- GitHub API has rate limits
- Version-number lock may cause version number waste

## Alternatives

| Option                     | Why not chosen                                               |
| -------------------------- | ------------------------------------------------------------ |
| GitHub only                | Limited to the GitHub ecosystem, cannot self-host a Registry |
| Cargo-style crates.io      | Too complex, not needed for YaoXiang's early ecosystem       |
| npm-style yank (mark only) | Security risk, known supply-chain attack cases               |

## Implementation Strategy

### Phases

| Phase     | Content                                                                                                                                                                                                                                                                               | Status                                                           |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Phase 3.5 | Source distribution enum-ification (Resolution 4) + native async fn in trait + migration of all implementations                                                                                                                                                                       | ✅ Completed                                                     |
| Phase 4a  | GitHub adapter layer (originally "Registry trait + reqwest + local mock" reduced per Resolution 1: github.com git dependency routing to `GitHubSource`, API parsing + `.yxpkg` asset download + git fallback; exponential backoff + ETag conditional cache implementing Resolution 6) | ✅ Completed                                                     |
| Phase 4b  | `.yxpkg` package format (tar.gz + SHA256SUMS + 20 MiB limit, Resolutions 3/7; deterministic packaging, mandatory verification on unpack)                                                                                                                                              | ✅ Completed                                                     |
| Phase 4c  | publish command (`--dry-run` runs full local chain; `--github` dedup check → tag validation → Release → asset upload; 6d workspace reference substitution materialized at pack time)                                                                                                  | ✅ Completed                                                     |
| Phase 4d  | Authentication (login/logout/credentials.toml) + yank                                                                                                                                                                                                                                 | Indefinitely deferred (with the official Registry, Resolution 1) |

**Landing notes (2026-09-29)**:

- HTTP stack: reqwest (rustls, no OpenSSL cross-compilation) + package-manager's own tokio
  current_thread runtime (`package::runtime::drive`); POST-class requests (creating
  Release/uploading assets) do not auto-retry — non-idempotent; on 5xx, resending may create
  duplicates.
- Target repository resolution for publish: `[package].repository` takes priority, falls back to
  `git remote origin`; the tag is required to already exist (same Cargo semantics: publish does not
  tag on the user's behalf).
- Pre-publish test run (step 3 of the validation checklist above) has been wired up (2026-09-30,
  alongside RFC-014b): by default runs tests discovered by `[tool.test]`, aborts publish on failure,
  skip with `--no-test`.
- `credentials.toml` and `login`/`logout`/`yank` commands are deferred with the official Registry;
  current authentication only uses the `$YX_GITHUB_TOKEN` environment variable (priority rules
  unchanged: environment variable > configuration file).

### Dependencies

- Depends on RFC-014 Phase 3 (global cache, semver substitution)
- Depends on RFC-014b (build system, for `build/` directory handling)

## Open Questions

- [x] Does the Registry API need versioning (`/api/v1/` vs `/api/v2/`)? → URL `/api/v1/` + version
      response header, breaking changes bump to v2 and coexist (2026-09-15)
- [x] Do package names support namespace (e.g., `@org/pkg`)? → Not supported in the early stage,
      flat package names (2026-09-15, see master document)
- [x] Rate-limiting strategy? → GitHub adapter layer backoff + cache; Registry side deferred with
      the official Registry (2026-09-15)
- [x] Package size limit? → Source package 20 MiB initial value (2026-09-15, adjustable)

---

## References

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
