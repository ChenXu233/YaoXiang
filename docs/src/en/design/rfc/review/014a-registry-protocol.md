---
title: "RFC-014a: Registry Protocol Specification"
status: "Under Review"
author: "Chenxu"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014a: Registry Protocol Specification

> This RFC is a sub-RFC of [RFC-014: Package Management System Design](../accepted/014-package-manager.md).

## Summary

Defines the Registry protocol for the YaoXiang package management system: the open interface design, the official Registry specification, the GitHub adapter layer, the package publish/withdraw flow, and the authentication model.

## Motivation

The RFC-014 overview defines the overall architecture of the package management system, but the Registry portion is only marked as "reserved". Without a Registry protocol, packages cannot be distributed — this is like designing a shopping cart without a store.

### Current Problems

- `RegistrySource` is stub code (`source/mod.rs:150-203`); `resolve` directly returns the declared version, and `download` returns an empty path
- There is no HTTP client (no `reqwest` dependency)
- There is no package publishing mechanism
- There is no authentication/authorization

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
   ┌─────────┐ ┌────────┐ ┌────────┐
   │ Official│ │ GitHub │ │ Custom │
   │Registry │ │Adapter │ │Registry│
   └─────────┘ └────────┘ └────────┘
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

All implementations (`LocalSource`, `GitSource`, `RegistrySource`) are uniformly changed to async. The CLI entrypoint is driven via `#[tokio::main]` or `Runtime::block_on`.

**Rationale:**
- The Registry requires HTTP requests; blocking would stall the entire install flow
- Parallel downloading of multiple dependencies (`join_all`) significantly improves install speed
- Git clone is also an I/O operation; async is more natural
- tokio is already a project dependency

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// Publish a package
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// Delete a published version (irrecoverable, version number is locked)
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// Query package information
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// Query the list of available versions
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// Search packages
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// Download a specified version
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// Authenticate
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### Source Priority (Default Lookup Chain)

Default lookup order for `yaoxiang add foo` (without flags):

| Priority | Lookup | Description |
|----------|--------|-------------|
| 1 | Global cache | `~/.yaoxiang/cache/registry/foo-<ver>/` |
| 2 | Official Registry | Query version → download |
| 3 | Failure | Report an error and prompt the user to check the package name or network |

**Explicit Override (bypasses the default chain):**

| Flag | Behavior |
|------|----------|
| `--git <url>` | Skip Registry, git clone directly (prefer Release assets → fallback to tag/branch) |
| `--path <dir>` | Skip Registry, use the local path directly |
| `--registry <url>` | Skip the official Registry, use the specified Registry |

### Official Registry

The official Registry is similar to crates.io and is the primary channel for package distribution.

**API Endpoints:**

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/packages/{name}` | GET | Query package information |
| `/api/v1/packages/{name}/versions` | GET | Query version list |
| `/api/v1/packages/{name}/{version}` | GET | Download package |
| `/api/v1/packages` | PUT | Publish package |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | Yank version |
| `/api/v1/search?q={query}` | GET | Search packages |
| `/api/v1/login` | POST | Authenticate |

### GitHub Integration

When GitHub is used as a package source, a Go modules-style strategy is adopted:

1. **Prefer Release assets**: Check the GitHub Release page for platform-matched prebuilt artifacts
2. **Fallback to the main branch**: If no Release exists, git clone

```toml
[dependencies]
# Basic git dependency
foo = { git = "https://github.com/user/foo" }

# Specify version (matches tag)
bar = { git = "https://github.com/user/bar", version = "^1.0.0" }

# Specify branch
baz = { git = "https://github.com/user/baz", branch = "main" }

# Specify commit
qux = { git = "https://github.com/user/qux", rev = "abc123" }

# Private repository (uses the GitHub token in credentials.toml)
private = { git = "https://github.com/my-org/private-lib" }
```

### Package Format (.yxpkg)

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # Package metadata
├── src/                   # Source code
├── build/                 # Build artifacts (if any)
│   └── native/
│       └── linux-x86_64/
│           └── libfoo.so
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
1. `yaoxiang.toml` must contain `name`, `version`, and `description`
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

- The package is completely deleted and cannot be recovered
- The version number is permanently occupied and cannot be republished with the same version number
- Projects whose existing lockfile references this version will error and need to upgrade to a different version
- **Security purpose**: Prevent npm-style supply chain attacks. Attackers have previously snatched the version numbers of deleted packages to inject malicious code; yank locks the version number to completely close off this path.

### Authentication Model

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**Mapping rule:** `yaoxiang login --registry <url>` matches the `url` field in `[registries.*]` by URL. If there is no match, a new entry is created (with an auto-generated name, such as `reg-1`).

**Priority:** Environment variable > configuration file

| Environment Variable | Purpose |
|----------------------|---------|
| `$YX_GITHUB_TOKEN` | GitHub authentication |
| `$YX_REGISTRY_TOKEN` | Registry authentication (used for the default Registry) |
| `$YX_REGISTRY_URL` | Default Registry address |

**CLI Commands:**

```bash
yaoxiang login --registry https://yxreg.example.com   # Match by URL or create new
yaoxiang login --github                                # GitHub OAuth or token
yaoxiang logout --registry https://yxreg.example.com   # Remove the matching entry
```

**Security Constraints:**
- Tokens are never written into `yaoxiang.toml` or `yaoxiang.lock`
- `credentials.toml` has file permission 600
- Use environment variables in CI scenarios; use the file in development scenarios

## Detailed Design

### RegistrySource Implementation

Replaces the existing stub code (`source/mod.rs:150-203`):

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
        // ... Extract to dest ...

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

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP client |
| `sha2` | SHA-256 verification |
| `flate2` + `tar` | Package format handling |
| `async-trait` | async trait support |

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Package '{0}' does not exist")]
    PackageNotFound(String),

    #[error("Version '{0}' does not exist")]
    VersionNotFound(String),

    #[error("Version '{0}' is already occupied")]
    VersionAlreadyExists(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("SHA-256 verification failed: expected {expected}, actual {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Insufficient permissions: {0}")]
    Forbidden(String),
}
```

## Trade-offs

### Advantages

- Open protocol, not tied to a specific server
- GitHub serves as a lightweight distribution channel, lowering the entry barrier
- Version number lock security model
- Precompiled-first install strategy

### Disadvantages

- The official Registry requires independent operations
- GitHub API has rate limits
- Version number locking may cause version number waste

## Alternatives

| Approach | Why Not Chosen |
|----------|----------------|
| GitHub-only support | Limited to the GitHub ecosystem, cannot self-host a Registry |
| Cargo-style crates.io | Too complex; not needed in the early YaoXiang ecosystem |
| npm-style yank (mark only) | Security risk; known supply chain attack cases |

## Implementation Strategy

### Phased Breakdown

| Phase | Content |
|-------|---------|
| Phase 3.5 | Migrate the Source trait to async + async-trait + migrate all implementations |
| Phase 4a | Registry trait + reqwest integration + local Registry mock |
| Phase 4b | GitHub Release adapter |
| Phase 4c | publish command + package format packaging |
| Phase 4d | Authentication + yank |

### Dependencies

- Depends on RFC-014 Phase 3 (global cache, semver replacement)
- Depends on RFC-014b (build system, for handling the `build/` directory)

## Open Questions

- [ ] Does the Registry API need versioning (`/api/v1/` vs `/api/v2/`)?
- [ ] Should package names support namespaces (e.g., `@org/pkg`)?
- [ ] Rate limiting strategy?
- [ ] Package size limit?

---

## References

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)