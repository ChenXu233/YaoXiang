---
title: 'RFC-014a: Registry Protocol Specification'
status: 'Under Review'
author: 'Chen Xu'
created: '2026-06-11'
updated: '2026-07-05'
group: 'rfc-014'
---

# RFC-014a: Registry Protocol Specification

> This RFC is a sub-RFC of [RFC-014: Package Manager Design](../accepted/014-package-manager.md).

## Summary

Defines the Registry protocol for the YaoXiang package management system: open interface design, official Registry specification, GitHub adapter layer, package publishing/withdrawal flows, and authentication model.

## Motivation

RFC-014 defines the overall architecture of the package management system, but the Registry section is only marked as "reserved". Without a Registry protocol, packages cannot be distributed—this is like designing a shopping cart without a store.

### Current Problems

- `RegistrySource` is stub code (`source/mod.rs:150-203`), `resolve` directly returns the declared version, `download` returns an empty path
- No HTTP client (no `reqwest` dependency)
- No package publishing mechanism
- No authentication/authorization

## Proposal

### Core Design: Open Protocol + Adapter Layer

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
   │ Registry│ │ Adapter│ │Registry│
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

All implementations (`LocalSource`, `GitSource`, `RegistrySource`) are uniformly changed to async. CLI entry points are driven by `#[tokio::main]` or `Runtime::block_on`.

**Rationale:**

- Registry requires HTTP requests; blocking would freeze the entire installation process
- Parallel dependency downloads (`join_all`) significantly improve installation speed
- Git clone is also an I/O operation; async is more natural
- tokio is already in the project dependencies

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// Publish a package
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// Remove a published version (irrecoverable, version number is locked)
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// Query package information
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// Query available version list
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// Search packages
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// Download a specific version
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// Authentication
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### Source Priority (Default Resolution Chain)

Default lookup order for `yaoxiang add foo` (no flags):

| Priority | Lookup          | Description                                              |
| -------- | --------------- | -------------------------------------------------------- |
| 1        | Global Cache    | `~/.yaoxiang/cache/registry/foo-<ver>/`                  |
| 2        | Official Registry| Query version → Download                                |
| 3        | Failure         | Report error, prompt user to check package name or network |

**Explicit Override (skip default chain):**

| flag               | Behavior                                                                          |
| ------------------ | --------------------------------------------------------------------------------- |
| `--git <url>`      | Skip Registry, directly Git clone (prefer Release assets → fallback to tag/branch) |
| `--path <dir>`     | Skip Registry, directly use local path                                            |
| `--registry <url>` | Skip official Registry, use specified Registry                                     |

### Official Registry

The official Registry is similar to crates.io and is the primary channel for package distribution.

**API Endpoints:**

| Endpoint                                   | Method | Description            |
| ------------------------------------------ | ------ | ---------------------- |
| `/api/v1/packages/{name}`                  | GET    | Query package info     |
| `/api/v1/packages/{name}/versions`         | GET    | Query version list     |
| `/api/v1/packages/{name}/{version}`        | GET    | Download package       |
| `/api/v1/packages`                         | PUT    | Publish package        |
| `/api/v1/packages/{name}/{version}/yank`  | DELETE | Withdraw version       |
| `/api/v1/search?q={query}`                 | GET    | Search packages        |
| `/api/v1/login`                            | POST   | Authentication         |

### GitHub Integration

When using GitHub as a package source, a Go modules-style strategy is adopted:

1. **Prefer Release assets**: Check if there are precompiled artifacts for matching platforms on the GitHub Release page
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

# Private repository (use GitHub token from credentials.toml)
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
└── SHA256SUMS             # Checksum
```

### Publish Flow

```bash
# Publish to official Registry
yaoxiang publish

# Publish to specified Registry
yaoxiang publish --registry my-company

# Also create GitHub Release
yaoxiang publish --github

# Dry run
yaoxiang publish --dry-run
```

Pre-publish validation:

1. `yaoxiang.toml` must have `name`, `version`, `description`
2. Version number must not already exist
3. Run tests (optional, skip with `--no-test`)
4. Calculate SHA-256 for all files
5. Package as `.yxpkg` (tar.gz)
6. Upload to Registry

### Yank Semantics

```bash
yaoxiang yank foo@1.2.3
```

**Deletion + Version Number Locking:**

- Package is completely deleted and unrecoverable
- Version number is permanently occupied; cannot republish the same version
- Projects with existing lockfiles referencing this version will report errors and need to upgrade to another version
- **Security purpose**: Prevent npm-style supply chain attacks. Attackers have previously registered deleted package version numbers to inject malicious code; yank locks the version number to completely close this avenue.

### Authentication Model

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**Mapping Rule:** `yaoxiang login --registry <url>` matches the `url` field in `[registries.*]` by URL. If no match is found, a new entry is created (auto-generated name, such as `reg-1`).

**Priority:** Environment variables > Configuration file

| Environment Variable       | Purpose                                |
| -------------------------- | -------------------------------------- |
| `$YX_GITHUB_TOKEN`         | GitHub authentication                  |
| `$YX_REGISTRY_TOKEN`       | Registry authentication (for default Registry) |
| `$YX_REGISTRY_URL`         | Default Registry address               |

**CLI Commands:**

```bash
yaoxiang login --registry https://yxreg.example.com   # Match by URL or create new
yaoxiang login --github                                # GitHub OAuth or token
yaoxiang logout --registry https://yxreg.example.com   # Delete matched entry
```

**Security Constraints:**

- Tokens are never written to `yaoxiang.toml` or `yaoxiang.lock`
- `credentials.toml` file permissions are 600
- CI scenarios use environment variables; development scenarios use files

## Detailed Design

### RegistrySource Implementation

Replace existing stub code (`source/mod.rs:150-203`):

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

| crate            | Purpose             |
| ---------------- | ------------------- |
| `reqwest`        | HTTP client         |
| `sha2`           | SHA-256 verification|
| `flate2` + `tar` | Package format handling |
| `async-trait`    | async trait support  |

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Package '{0}' not found")]
    PackageNotFound(String),

    #[error("Version '{0}' not found")]
    VersionNotFound(String),

    #[error("Version '{0}' already exists")]
    VersionAlreadyExists(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("SHA-256 verification failed: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Permission denied: {0}")]
    Forbidden(String),
}
```

## Trade-offs

### Advantages

- Open protocol, not bound to specific servers
- GitHub as a lightweight distribution channel lowers the barrier to entry
- Version number locking security model
- Precompiled-first installation strategy

### Disadvantages

- Official Registry requires independent operations and maintenance
- GitHub API has rate limits
- Version number locking may lead to version number waste

## Alternative Solutions

| Solution                | Why Not Chosen                            |
| ----------------------- | ----------------------------------------- |
| GitHub-only support     | Limited to GitHub ecosystem, cannot build self-hosted Registry |
| Cargo-style crates.io   | Too complex, not needed at YaoXiang ecosystem early stage |
| npm-style yank (mark only) | Security risk, known supply chain attack cases |

## Implementation Strategy

### Phase Division

| Phase     | Content                                                         |
| --------- | --------------------------------------------------------------- |
| Phase 3.5 | Source trait change to async + async-trait + migrate all implementations |
| Phase 4a  | Registry trait + reqwest integration + local Registry mock     |
| Phase 4b  | GitHub Release adapter                                          |
| Phase 4c  | publish command + package format packaging                      |
| Phase 4d  | Authentication + yank                                           |

### Dependencies

- Depends on RFC-014 Phase 3 (global cache, semver resolution)
- Depends on RFC-014b (build system, used for `build/` directory handling)

## Open Questions

- [ ] Should Registry API be versioned (`/api/v1/` vs `/api/v2/`)?
- [ ] Should package names support namespaces (such as `@org/pkg`)?
- [ ] Rate limiting strategy?
- [ ] Package size limit?

---

## References

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
