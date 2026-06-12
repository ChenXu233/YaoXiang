//! 测试 `PackageManifest` 的创建、加载、保存和依赖管理
//!
//! 覆盖:
//! - 新建 manifest 的默认值
//! - 保存后重新加载的一致性
//! - 从非项目目录加载返回错误
//! - 添加/移除/查询依赖
//! - 含 table 形式依赖和空依赖的 TOML 解析

use crate::package::error::PackageError;
use crate::package::manifest::PackageManifest;

#[test]
fn test_new_manifest() {
    let manifest = PackageManifest::new("test-project");
    assert_eq!(manifest.package.name, "test-project");
    assert_eq!(manifest.package.version, "0.1.0");
    assert!(manifest.dependencies.is_empty());
    assert!(manifest.dev_dependencies.is_empty());
}

#[test]
fn test_save_and_load() {
    let dir = tempfile::TempDir::new().unwrap();
    let manifest = PackageManifest::new("test-project");
    manifest.save(dir.path()).unwrap();

    let loaded = PackageManifest::load(dir.path()).unwrap();
    assert_eq!(loaded.package.name, "test-project");
    assert_eq!(loaded.package.version, "0.1.0");
}

#[test]
fn test_load_not_project() {
    let dir = tempfile::TempDir::new().unwrap();
    let result = PackageManifest::load(dir.path());
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PackageError::NotProject));
}

#[test]
fn test_add_dependency() {
    let mut manifest = PackageManifest::new("test");
    manifest.add_dependency("foo", "1.0.0");
    assert!(manifest.dependencies.contains_key("foo"));
    assert_eq!(
        manifest.dependencies["foo"],
        toml::Value::String("1.0.0".to_string())
    );
}

#[test]
fn test_add_dev_dependency() {
    let mut manifest = PackageManifest::new("test");
    manifest.add_dev_dependency("bar", "2.0.0");
    assert!(manifest.dev_dependencies.contains_key("bar"));
}

#[test]
fn test_remove_dependency() {
    let mut manifest = PackageManifest::new("test");
    manifest.add_dependency("foo", "1.0.0");
    assert!(manifest.remove_dependency("foo"));
    assert!(!manifest.dependencies.contains_key("foo"));
}

#[test]
fn test_remove_nonexistent_dependency() {
    let mut manifest = PackageManifest::new("test");
    assert!(!manifest.remove_dependency("nonexistent"));
}

#[test]
fn test_has_dependency() {
    let mut manifest = PackageManifest::new("test");
    manifest.add_dependency("foo", "1.0.0");
    manifest.add_dev_dependency("bar", "2.0.0");
    assert!(manifest.has_dependency("foo"));
    assert!(manifest.has_dependency("bar"));
    assert!(!manifest.has_dependency("baz"));
}

#[test]
fn test_round_trip_with_dependencies() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut manifest = PackageManifest::new("test-project");
    manifest.add_dependency("foo", "1.0.0");
    manifest.add_dev_dependency("bar", "2.0.0");
    manifest.save(dir.path()).unwrap();

    let loaded = PackageManifest::load(dir.path()).unwrap();
    assert!(loaded.dependencies.contains_key("foo"));
    assert!(loaded.dev_dependencies.contains_key("bar"));
}

#[test]
fn test_parse_toml_with_table_dependency() {
    let toml_str = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
foo = "1.0.0"
bar = { version = "2.0.0", git = "https://github.com/example/bar" }
"#;
    let manifest: PackageManifest = toml::from_str(toml_str).unwrap();
    assert_eq!(manifest.package.name, "test");
    assert!(manifest.dependencies.contains_key("foo"));
    assert!(manifest.dependencies.contains_key("bar"));
}

#[test]
fn test_parse_empty_dependencies() {
    let toml_str = r#"
[package]
name = "test"
version = "0.1.0"
"#;
    let manifest: PackageManifest = toml::from_str(toml_str).unwrap();
    assert!(manifest.dependencies.is_empty());
    assert!(manifest.dev_dependencies.is_empty());
}
