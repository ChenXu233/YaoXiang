//! 测试 `yaoxiang update` 命令
//!
//! 覆盖:
//! - 无依赖时更新不报错
//! - 添加依赖后更新锁文件
//! - 更新后版本刷新
//! - 单个依赖更新

use crate::package::commands::add;
use crate::package::commands::init;
use crate::package::commands::update::{exec_in, exec_single_in};
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use tempfile::TempDir;

fn setup_project() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    init::exec_in(tmp.path(), &init::InitOptions { lib: false }, "test-proj").unwrap();
    let project_dir = tmp.path().join("test-proj");
    (tmp, project_dir)
}

#[test]
fn test_update_empty() {
    let (_tmp, project_dir) = setup_project();
    exec_in(&project_dir).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert!(lock.package.is_empty());
}

#[test]
fn test_update_with_deps() {
    let (_tmp, project_dir) = setup_project();
    add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
    add::exec_in(&project_dir, "bar", Some("2.0.0"), false).unwrap();

    exec_in(&project_dir).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert_eq!(lock.package.len(), 2);
    assert!(lock.package.contains_key("foo"));
    assert!(lock.package.contains_key("bar"));
}

#[test]
fn test_update_refreshes_versions() {
    let (_tmp, project_dir) = setup_project();
    add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();

    // Manually modify the manifest to simulate version bump
    let mut manifest = PackageManifest::load(&project_dir).unwrap();
    manifest.add_dependency("foo", "2.0.0");
    manifest.save(&project_dir).unwrap();

    exec_in(&project_dir).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert_eq!(lock.package["foo"].version, "2.0.0");
}

#[test]
fn test_update_single_dependency() {
    let (_tmp, project_dir) = setup_project();
    add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
    add::exec_in(&project_dir, "bar", Some("2.0.0"), false).unwrap();

    // 先安装
    crate::package::commands::install::exec_in(&project_dir).unwrap();

    // 修改 foo 的版本
    let mut manifest = PackageManifest::load(&project_dir).unwrap();
    manifest.add_dependency("foo", "1.1.0");
    manifest.save(&project_dir).unwrap();

    // 只更新 foo
    exec_single_in(&project_dir, "foo").unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert_eq!(lock.package["foo"].version, "1.1.0");
    // bar 不受影响
    assert!(lock.package.contains_key("bar"));
}
