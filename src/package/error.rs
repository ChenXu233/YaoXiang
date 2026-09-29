//! Package manager error types

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during package management operations.
#[derive(Debug, Error)]
pub enum PackageError {
    /// Project directory already exists
    #[error("Project already exists: {0}")]
    ProjectExists(PathBuf),

    /// Not inside a YaoXiang project (no yaoxiang.toml found)
    #[error("Not a YaoXiang project: yaoxiang.toml not found")]
    NotProject,

    /// Dependency not found in manifest
    #[error("Dependency not found: {0}")]
    DependencyNotFound(String),

    /// Dependency already exists in manifest
    #[error("Dependency already exists: {0}")]
    DependencyAlreadyExists(String),

    /// One or more dependencies could not be installed
    #[error("dependency installation failed: {0}")]
    DependencyInstallFailed(String),

    /// Invalid manifest format
    #[error("Invalid yaoxiang.toml format: {0}")]
    InvalidManifest(String),

    /// Global package cache error (RFC-014 Phase 3)
    #[error("cache error: {0}")]
    Cache(String),

    /// Not inside a YaoXiang workspace (no [workspace] in any yaoxiang.toml, RFC-014c)
    #[error("not a YaoXiang workspace: [workspace] section not found")]
    NotWorkspace,

    /// Workspace member manifest missing (RFC-014c)
    #[error("workspace member '{key}' not found: {path}")]
    MemberMissing { key: String, path: String },

    /// Workspace member manifest invalid (RFC-014c)
    #[error("workspace member '{key}' invalid: {reason}")]
    MemberInvalid { key: String, reason: String },

    /// Nested workspace (forbidden, RFC-014c 2026-09-15 decision 4)
    #[error("nested workspace is not supported: member '{key}' at {path} has its own [workspace] section")]
    NestedWorkspace { key: String, path: String },

    /// Package content exceeds the source-package size limit
    /// (RFC-014a 2026-09-15 decision 7: 20 MiB)
    #[error("package too large: {0}")]
    PackageTooLarge(String),

    /// Checksum mismatch (RFC-014a)
    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// Invalid package archive (missing manifest, path escape, bad entry type, ...)
    #[error("invalid package: {0}")]
    InvalidPackage(String),

    /// Network request failed (GitHub adapter, RFC-014a Phase 4)
    #[error("network error: {0}")]
    Network(String),

    /// API rate limit exhausted (RFC-014a decision 6)
    #[error("rate limited: {0}")]
    RateLimited(String),

    /// Bare `publish` without a channel (official registry deferred, RFC-014a decision 1)
    #[error("official registry is deferred (RFC-014a); use `publish --github` or `--dry-run`")]
    RegistryDeferred,

    /// Release for the version already exists (RFC-014a publish validation)
    #[error("release already exists: {0}")]
    VersionAlreadyExists(String),

    /// Authentication failed (RFC-014a)
    #[error("auth failed: {0}")]
    AuthFailed(String),

    /// Publish target repository could not be resolved
    #[error("publish target unresolved: {0}")]
    PublishTarget(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML serialization/deserialization error
    #[error("TOML parse error: {0}")]
    Toml(String),
}

impl From<toml::de::Error> for PackageError {
    fn from(e: toml::de::Error) -> Self {
        PackageError::Toml(e.to_string())
    }
}

impl From<toml::ser::Error> for PackageError {
    fn from(e: toml::ser::Error) -> Self {
        PackageError::Toml(e.to_string())
    }
}

/// Result type for package operations
pub type PackageResult<T> = Result<T, PackageError>;
