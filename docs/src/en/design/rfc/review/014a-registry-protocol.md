---
title: 'RFC-014a: Registry Protocol Specification'
status: 'Under Review'
author: '晨煦'
created: '2026-06-11'
updated: '2026-09-15'
group: 'rfc-014'
---

# RFC-014a: Registry Protocol Specification

> This RFC is a sub-RFC of
> [RFC-014: Package Management System Design](../accepted/014-package-manager.md).

## 2026-09-15 Review Resolution

The following resolutions were finalized by the owner on 2026-09-15. The corresponding sections of
the body are retained as the complete specification for "after the official Registry goes live":

1. **Scope Reduction (the most important item in this document)**: The official Registry server,
   authentication (login/logout), and yank are **indefinitely deferred**. Phase 4 actual
   deliverables = GitHub Release/Git adapter layer + Registry trait finalization + `.yxpkg`
   packaging + `publish --github`. Rationale: Cold-starting the ecosystem only requires the
   git/GitHub channel (Go followed the same model early on); the operational, account-system, and
   abuse-governance costs of a Registry server are pure liabilities at the stage with no third-party
   packages.
2. **Bare package name add is unavailable**: Before the official Registry goes live,
   `yaoxiang add <bare-name>` reports an error; adding dependencies must use an explicit source
   (`--git` / `--path`). The "source priority" default lookup chain below takes effect from Registry
   launch onward.
3. **Package format normalization**: `.yxpkg` only contains source code
   (`yaoxiang.toml`/`src/`/`build.yx`/`SHA256SUMS`); the `build/native/` precompiled artifact
   directory is removed. Binary distribution is uniformly handled through the `[binaries]` external
   link in RFC-014b (Release/CDN), to avoid package bloat and global cache expansion.
4. **Source distribution implementation**: The four built-in sources (Local/Git/Registry/GitHub)
   form a closed set; the implementation layer uses an enum for dispatch (avoiding the `Send`
   constraint of dyn-async and the `async-trait` dependency); the `Source` trait definition is
   retained at the semantic layer, and if third-party Sources are opened up in the future, they will
   be integrated via trait objects.
5. **API versioning**: URL path `/api/v1/` + response header carries protocol version; breaking
   changes bump to v2 and coexist, no in-place changes.
6. **Rate limiting**: The GitHub adapter layer uses exponential backoff + ETag conditional request
   caching; Registry-side rate policy is deferred along with the official Registry.
7. **Package size limit**: Source package 20 MiB (initial value, adjustable); the GitHub channel is
   governed by the platform itself.

## Summary

Defines the Registry protocol for the YaoXiang package management system: open interface design,
official Registry specification, GitHub adapter layer, package publish/yank flow, and authentication
model.

## Motivation

The RFC-014 master document defines the overall architecture of the package management system, but
the Registry section is only marked as "reserved". Without a Registry protocol, packages cannot be
distributed — it's like designing a shopping cart without a store.

### Current Problems

- `RegistrySource` is stub code (`source/mod.rs:150-203`); `resolve` directly returns the declared
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
// Existing (sync) → changed to (async)
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

- The Registry requires HTTP requests; blocking would stall the entire install flow
- Parallel download of multiple dependencies (`join_all`) significantly improves install speed
- Git clone is also an I/O operation; async is more natural
- tokio is already a project dependency

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// Publish package
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// Delete a published version (irreversible, version number locked)
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// Query package info
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// Query list of available versions
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// Search packages
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// Download specified version
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// Authenticate
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### Source Priority (Default Lookup Chain)

Default lookup order for `yaoxiang add foo` (no flag):

| Priority | Lookup            | Description                                                |
| -------- | ----------------- | ---------------------------------------------------------- |
| 1        | Global cache      | `~/.yaoxiang/cache/registry/foo-<ver>/`                    |
| 2        | Official Registry | Query version → download                                   |
| 3        | Failure           | Report error, prompt user to check package name or network |

**Explicit Override (bypasses the default chain):**

| flag               | Behavior                                                                           |
| ------------------ | ---------------------------------------------------------------------------------- |
| `--git <url>`      | Skip Registry, directly Git clone (prefer Release assets → fallback to tag/branch) |
| `--path <dir>`     | Skip Registry, directly use local path                                             |
| `--registry <url>` | Skip official Registry, use specified Registry                                     |

### Official Registry

The official Registry is similar to crates.io and is the primary distribution channel for packages.

**API Endpoints:**

| Endpoint                                 | Method | Description        |
| ---------------------------------------- | ------ | ------------------ |
| `/api/v1/packages/{name}`                | GET    | Query package info |
| `/api/v1/packages/{name}/versions`       | GET    | Query version list |
| `/api/v1/packages/{name}/{version}`      | GET    | Download package   |
| `/api/v1/packages`                       | PUT    | Publish package    |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | Yank version       |
| `/api/v1/search?q={query}`               | GET    | Search packages    |
| `/api/v1/login`                          | POST   | Authenticate       |

### GitHub Integration

When using GitHub as a package source, a Go modules-style strategy is adopted:

1. **Prefer Release assets**: Check the GitHub Release page for precompiled artifacts matching the
   current platform
2. **Fallback to main branch**: Git clone if no Release is found

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

# Private repository (use GitHub token from credentials.toml)
private = { git = "https://github.com/my-org/private-lib" }
```

### Package Format (.yxpkg)

> 2026-09-15 Resolution: Contains only source code; `build/` precompiled artifacts removed —
> binaries are distributed uniformly via RFC-014b `[binaries]` external links.

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # Package metadata
├── src/                   # Source code
├── build.yx               # Build script (if any)
└── SHA256SUMS             # Checksums
```

### publish Flow

```bash
# Publish to official Registry
yaoxiang publish

# Publish to a specified Registry
yaoxiang publish --registry my-company

# Also create a GitHub Release
yaoxiang publish --github

# Dry run
yaoxiang publish --dry-run
```

Pre-publish validation:

1. `yaoxiang.toml` must contain `name`, `version`, `description`
2. Version number must not already exist
3. Run tests (optional, skip with `--no-test`)
4. Compute SHA-256 of all files
5. Pack as `.yxpkg` (tar.gz)
6. Upload to Registry

### yank Semantics

```bash
yaoxiang yank foo@1.2.3
```

**Deletion + version number lockout:**

- The package is permanently deleted, irreversible
- The version number is permanently occupied; republishing the same version number is not allowed
- Existing lockfiles that reference this version will report an error and need to be upgraded to
  another version
- **Security purpose**: Prevents npm-style supply chain attacks. Attackers have previously hijacked
  deleted package version numbers to inject malicious code; yank version lockout completely closes
  this path.

### Authentication Model

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**Mapping rule:** `yaoxiang login --registry <url>` matches by URL against the `url` field in
`[registries.*]`. If no match is found, a new entry is created (with an auto-generated name, such as
`reg-1`).

**Priority:** Environment variable > config file

| Environment Variable | Purpose                                            |
| -------------------- | -------------------------------------------------- |
| `$YX_GITHUB_TOKEN`   | GitHub authentication                              |
| `$YX_REGISTRY_TOKEN` | Registry authentication (for the default Registry) |
| `$YX_REGISTRY_URL`   | Default Registry address                           |

**CLI commands:**

```bash
yaoxiang login --registry https://yxreg.example.com   # Match by URL or create new
yaoxiang login --github                                # GitHub OAuth or token
yaoxiang logout --registry https://yxreg.example.com   # Remove the matching entry
```

**Security constraints:**

- Tokens are never written to `yaoxiang.toml` or `yaoxiang.lock`
- `credentials.toml` has file permission 600
- Use environment variables in CI; use the file in development

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

| crate            | Purpose                   |
| ---------------- | ------------------------- |
| `reqwest`        | HTTP client               |
| `sha2`           | SHA-256 verification      |
| `flate2` + `tar` | Package format processing |
| `async-trait`    | async trait support       |

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("package '{0}' does not exist")]
    PackageNotFound(String),

    #[error("version '{0}' does not exist")]
    VersionNotFound(String),

    #[error("version '{0}' is already occupied")]
    VersionAlreadyExists(String),

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("SHA-256 verification failed: expected {expected}, actual {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("insufficient permissions: {0}")]
    Forbidden(String),
}
```

## Trade-offs

### Pros

- Open protocol, not bound to a specific server
- GitHub as a lightweight distribution channel lowers the entry barrier
- Version-number-lockout security model
- Precompiled-first install strategy

### Cons

- The official Registry requires independent operations
- GitHub API has rate limits
- Version-number lockout may cause version number waste

## Alternatives

| Alternative                | Why not chosen                                               |
| -------------------------- | ------------------------------------------------------------ |
| GitHub only                | Constrained by the GitHub ecosystem; no self-hosted Registry |
| Cargo-style crates.io      | Too complex; not needed in the early YaoXiang ecosystem      |
| npm-style yank (mark only) | Security risk; known supply chain attack cases               |

## Implementation Strategy

### Phase Division

| Phase     | Content                                                                      |
| --------- | ---------------------------------------------------------------------------- |
| Phase 3.5 | Source trait converted to async + async-trait + all implementations migrated |
| Phase 4a  | Registry trait + reqwest integration + local Registry mock                   |
| Phase 4b  | GitHub Release adapter                                                       |
| Phase 4c  | publish command + package format packaging                                   |
| Phase 4d  | Authentication + yank                                                        |

### Dependencies

- Depends on RFC-014 Phase 3 (global cache, semver replacement)
- Depends on RFC-014b (build system, for `build/` directory handling)

## Open Questions

- [x] Does the Registry API need versioning (`/api/v1/` vs `/api/v2/`)? → URL `/api/v1/` + version
      response header; breaking changes bump to v2 and coexist (2026-09-15)
- [x] Do package names support namespaces (such as `@org/pkg`)? → Not supported initially; flat
      package names (2026-09-15, see master document)
- [x] Rate limiting policy? → GitHub adapter layer uses backoff + caching; Registry side is deferred
      along with the official Registry (2026-09-15)
- [x] Package size limit? → Source package 20 MiB initial value (2026-09-15, adjustable)

---

## References

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
