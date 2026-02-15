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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_exists_error() {
        let err = PackageError::ProjectExists(PathBuf::from("my-project"));
        assert!(err.to_string().contains("my-project"));
    }

    #[test]
    fn test_not_project_error() {
        let err = PackageError::NotProject;
        assert!(err.to_string().contains("yaoxiang.toml"));
    }

    #[test]
    fn test_dependency_not_found_error() {
        let err = PackageError::DependencyNotFound("foo".to_string());
        assert!(err.to_string().contains("foo"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let pkg_err: PackageError = io_err.into();
        assert!(matches!(pkg_err, PackageError::Io(_)));
    }

    #[test]
    fn test_toml_error_conversion() {
        // Test that toml parse errors convert properly
        let result: Result<toml::Value, _> = toml::from_str("invalid = [");
        if let Err(e) = result {
            let pkg_err: PackageError = e.into();
            assert!(matches!(pkg_err, PackageError::Toml(_)));
        }
    }
}
