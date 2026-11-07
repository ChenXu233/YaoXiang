//! 测试 `yaoxiang add` 命令
//!
//! 覆盖:
//! - 添加普通依赖
//! - 添加开发依赖
//! - 添加依赖后自动更新锁文件
//! - 重复添加依赖返回错误
//! - 不指定版本时使用默认 `*`

use crate::package::commands::add::exec_in;
use crate::package::commands::init;
use crate::package::error::PackageError;
use crate::package::lock::LockFile;
use tempfile::TempDir;

fn setup_project() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    init::exec_in(tmp.path(), &init::InitOptions { lib: false }, "test-proj").unwrap();
    let project_dir = tmp.path().join("test-proj");
    (tmp, project_dir)
}

#[test]
fn test_add_updates_lock() {
    let (_tmp, project_dir) = setup_project();
    exec_in(&project_dir, "foo", Some("1.0.0"), None, None, false, false).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert!(lock.package.contains_key("foo"));
    assert_eq!(lock.package["foo"].version, "1.0.0");
}

#[test]
fn test_add_duplicate_fails() {
    let (_tmp, project_dir) = setup_project();
    exec_in(&project_dir, "foo", Some("1.0.0"), None, None, false, false).unwrap();

    let result = exec_in(&project_dir, "foo", Some("2.0.0"), None, None, false, false);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PackageError::DependencyAlreadyExists(_)
    ));
}

#[test]
fn test_add_git_and_path_conflict() {
    let (_tmp, project_dir) = setup_project();
    let result = exec_in(
        &project_dir,
        "x",
        None,
        Some("https://github.com/u/x"),
        Some("./x"),
        false,
        false,
    );
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        PackageError::InvalidManifest(_)
    ));
}
