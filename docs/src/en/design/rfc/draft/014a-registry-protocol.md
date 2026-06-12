---
title: "RFC-014a: Registry Protocol Specification"
status: "Draft"
author: "Chenxu"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014a: Registry Protocol Specification

> This RFC is a sub-RFC of [RFC-014: Package Management System Design](../accepted/014-package-manager.md).

## Summary

Defines the Registry protocol for the YaoXiang package management system: open interface design, official Registry specification, GitHub adaptation layer, package publish/yank workflow, and authentication model.

## Motivation

The RFC-014 master document defines the overall architecture of the package management system, but the Registry section is merely marked as "reserved". Without a Registry protocol, packages cannot be distributed — it's like designing a shopping cart without a store.

### Current Problems

- `RegistrySource` is stub code (`source/mod.rs:150-203`), where `resolve` directly returns the declared version and `download` returns an empty path
- No HTTP client (no `reqwest` dependency)
- No package publishing mechanism
- No authentication/authorization

## Proposal

### Core Design: Open Protocol + Adaptation Layer

```
┌──────────────────────────────────────────┐
│         yaoxiang publish/install         │  ← CLI Layer
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│          Registry Trait                  │  ← Protocol Layer (Open Interface)
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

### Asynchronous Architecture Decision

The `Source` trait is uniformly changed to async, fully embracing tokio:

```rust
// Existing (synchronous) → Changed to (asynchronous)
#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String>;
    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage>;
}
```

All implementations (`LocalSource`, `GitSource`, `RegistrySource`) are uniformly converted to async. The CLI entry point is driven via `#[tokio::main]` or `Runtime::block_on`.

**Rationale:**
- Registry requires HTTP requests; blocking would freeze the entire install workflow
- Parallel download of multiple dependencies (`join_all`) significantly improves install speed
- Git clone is also an I/O operation, and async is more natural
- tokio is already in the project's dependencies

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// Publish a package
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// Yank a published version (irrecoverable, version number is locked)
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// Query package information
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// Query the list of available versions
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

Default lookup order when running `yaoxiang add foo` (no flag):

| Priority | Lookup | Description |
|----------|--------|-------------|
| 1 | Global cache | `~/.yaoxiang/cache/registry/foo-<ver>/` |
| 2 | Official Registry | Query version → Download |
| 3 | Failure | Report error, prompt user to check package name or network |

**Explicit override (does not follow the default chain):**

| Flag | Behavior |
|------|----------|
| `--git <url>` | Skip Registry, directly Git clone (prefer Release assets → fallback to tag/branch) |
| `--path <dir>` | Skip Registry, directly use local path |
| `--registry <url>` | Skip official Registry, use the specified Registry |

### Official Registry

The official Registry, similar to crates.io, is the main distribution channel for packages.

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

When GitHub is used as a package source, the Go modules-style strategy is adopted:

1. **Prefer Release assets**: Check the GitHub Release page for matching pre-built artifacts for the platform
2. **Fallback to main branch**: If no Release exists, git clone

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

# Private repository (uses GitHub token from credentials.toml)
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

### publish Workflow

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
1. `yaoxiang.toml` must contain `name`, `version`, `description`
2. The version number must not already exist
3. Run tests (optional, skip with `--no-test`)
4. Compute SHA-256 of all files
5. Pack into `.yxpkg` (tar.gz)
6. Upload to the Registry

### yank Semantics

```bash
yaoxiang yank foo@1.2.3
```

**Deletion + version number lock:**

- The package is completely deleted and cannot be recovered
- The version number is permanently occupied; the same version number cannot be republished
- Projects that already have a lockfile referencing this version will fail and need to upgrade to another version
- **Security purpose**: To prevent npm-style supply chain attacks. Attackers have previously re-registered deleted package version numbers to inject malicious code; yank's version-number-lock mechanism completely closes off this attack vector.

### Authentication Model

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**Mapping rule:** `yaoxiang login --registry <url>` matches the `url` field in `[registries.*]` by URL. If no match is found, a new entry is created (with an auto-generated name, such as `reg-1`).

**Priority:** Environment variable > Configuration file

| Environment Variable | Purpose |
|----------------------|---------|
| `$YX_GITHUB_TOKEN` | GitHub authentication |
| `$YX_REGISTRY_TOKEN` | Registry authentication (for the default Registry) |
| `$YX_REGISTRY_URL` | Default Registry address |

**CLI commands:**

```bash
yaoxiang login --registry https://yxreg.example.com   # Match by URL or create new
yaoxiang login --github                                # GitHub OAuth or token
yaoxiang logout --registry https://yxreg.example.com   # Remove matching entry
```

**Security constraints:**
- Tokens are never written to `yaoxiang.toml` or `yaoxiang.lock`
- `credentials.toml` file permissions are 600
- Use environment variables in CI scenarios, use files in development scenarios

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

    #[error("Version '{0}' is already taken")]
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
- GitHub as a lightweight distribution channel lowers the entry barrier
- Version-number-lock security model
- Pre-compilation-first install strategy

### Disadvantages

- The official Registry requires independent operations and maintenance
- GitHub API has rate limits
- Version-number-lock may lead to version number waste

## Alternatives

| Approach | Why Not Chosen |
|----------|----------------|
| GitHub-only | Limited to the GitHub ecosystem, cannot self-host a Registry |
| Cargo-style crates.io | Too complex, not needed in the early YaoXiang ecosystem |
| npm-style yank (mark only) | Security risk, known supply chain attack cases |

## Implementation Strategy

### Phase Breakdown

| Phase | Content |
|-------|---------|
| Phase 3.5 | Convert Source trait to async + async-trait + migrate all implementations |
| Phase 4a | Registry trait + reqwest integration + local Registry mock |
| Phase 4b | GitHub Release adaptation |
| Phase 4c | publish command + package format packaging |
| Phase 4d | Authentication + yank |

### Dependencies

- Depends on RFC-014 Phase 3 (global cache, semver replacement)
- Depends on RFC-014b (build system, for `build/` directory handling)

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