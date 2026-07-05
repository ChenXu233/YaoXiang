//! 测试 `yaoxiang rm` 命令
//!
//! 覆盖:
//! - 移除普通依赖
//! - 移除开发依赖
//! - 移除后更新锁文件
//! - 移除不存在的依赖返回错误

use crate::package::commands::add;
use crate::package::commands::init;
use crate::package::commands::rm::exec_in;
use crate::package::error::PackageError;
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use tempfile::TempDir;

fn setup_project_with_deps() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    init::exec_in(tmp.path(), &init::InitOptions { lib: false }, "test-proj").unwrap();
    let project_dir = tmp.path().join("test-proj");
    add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();
    add::exec_in(&project_dir, "bar", Some("2.0.0"), true).unwrap();
    (tmp, project_dir)
}

#[test]
fn test_rm_dependency() {
    let (_tmp, project_dir) = setup_project_with_deps();
    exec_in(&project_dir, "foo", false).unwrap();

    let manifest = PackageManifest::load(&project_dir).unwrap();
    assert!(!manifest.dependencies.contains_key("foo"));
}

#[test]
fn test_rm_dev_dependency() {
    let (_tmp, project_dir) = setup_project_with_deps();
    exec_in(&project_dir, "bar", true).unwrap();

    let manifest = PackageManifest::load(&project_dir).unwrap();
    assert!(!manifest.dev_dependencies.contains_key("bar"));
}

#[test]
fn test_rm_updates_lock() {
    let (_tmp, project_dir) = setup_project_with_deps();
    exec_in(&project_dir, "foo", false).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert!(!lock.package.contains_key("foo"));
}

#[test]
fn test_rm_nonexistent_fails() {
    let (_tmp, project_dir) = setup_project_with_deps();
    let result = exec_in(&project_dir, "nonexistent", false);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PackageError::DependencyNotFound(_)
    ));
}
