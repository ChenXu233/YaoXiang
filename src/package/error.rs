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

    /// Invalid manifest format
    #[error("Invalid yaoxiang.toml format: {0}")]
    InvalidManifest(String),

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
