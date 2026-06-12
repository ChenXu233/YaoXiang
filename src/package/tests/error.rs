//! 测试 `PackageError` 错误类型与转换
//!
//! 覆盖:
//! - 各错误变体的 Display 输出
//! - IO 错误到 `PackageError::Io` 的转换
//! - TOML 解析错误到 `PackageError::Toml` 的转换

use std::path::PathBuf;

use crate::package::error::PackageError;

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
