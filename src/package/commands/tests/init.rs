//! 测试 `yaoxiang init` 命令
//!
//! 覆盖:
//! - 项目目录结构创建
//! - manifest 内容正确性
//! - main.yx 模板内容
//! - 已存在项目目录时返回错误

use std::fs;

use crate::package::commands::init::exec_in;
use crate::package::error::PackageError;
use crate::package::manifest::PackageManifest;
use tempfile::TempDir;

#[test]
fn test_init_creates_project() {
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), "test-project").unwrap();

    let project_path = tmp.path().join("test-project");
    assert!(project_path.join("yaoxiang.toml").exists());
    assert!(project_path.join("yaoxiang.lock").exists());
    assert!(project_path.join("src/main.yx").exists());
    assert!(project_path.join(".gitignore").exists());
    // 标准库接口文件
    assert!(project_path.join(".yaoxiang/std/io.yx").exists());
    assert!(project_path.join(".yaoxiang/std/list.yx").exists());
    assert!(project_path.join(".yaoxiang/std/math.yx").exists());
}

#[test]
fn test_init_manifest_content() {
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), "my-app").unwrap();

    let manifest = PackageManifest::load(&tmp.path().join("my-app")).unwrap();
    assert_eq!(manifest.package.name, "my-app");
    assert_eq!(manifest.package.version, "0.1.0");
}

#[test]
fn test_init_main_yx_content() {
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), "my-app").unwrap();

    let content = fs::read_to_string(tmp.path().join("my-app/src/main.yx")).unwrap();
    assert!(content.contains("my-app"));
    // YaoXiang 使用 `main = {...}` 语法而非 `fn main() {}`
    assert!(content.contains("main ="));
}

#[test]
fn test_init_existing_project_fails() {
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), "existing").unwrap();

    let result = exec_in(tmp.path(), "existing");
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PackageError::ProjectExists(_)
    ));
}
