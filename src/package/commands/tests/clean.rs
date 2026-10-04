//! 测试 `yaoxiang clean` 命令
//!
//! 覆盖:
//! - vendor 中未被 lock 引用的残留包被裁剪
//! - lock 引用的包保留
//! - `.yaoxiang/build` 构建产物目录被删除
//! - 无可清理内容不报错

use std::path::Path;

use tempfile::TempDir;

use crate::package::commands::clean::exec_in;
use crate::package::commands::init;
use crate::package::lock::{LockFile, LockedDependency};
use crate::package::vendor::VendorManager;

fn setup_project() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    init::exec_in(tmp.path(), &init::InitOptions { lib: false }, "test-proj").unwrap();
    let project_dir = tmp.path().join("test-proj");
    (tmp, project_dir)
}

fn touch_dep(
    project_dir: &Path,
    name: &str,
    version: &str,
) {
    let manager = VendorManager::new(project_dir);
    manager.ensure_vendor_dir().unwrap();
    std::fs::create_dir_all(manager.dep_path(name, version)).unwrap();
}

fn lock_entry(
    project_dir: &Path,
    name: &str,
    version: &str,
) {
    let mut lock = LockFile::load(project_dir).unwrap();
    lock.package.insert(
        name.to_string(),
        LockedDependency {
            version: version.to_string(),
            source: "path".to_string(),
            checksum: None,
        },
    );
    lock.save(project_dir).unwrap();
}

#[test]
fn test_clean_prunes_stale_vendor_packages() {
    let (_tmp, project_dir) = setup_project();

    touch_dep(&project_dir, "foo", "1.0.0"); // 在 lock 中 → 保留
    touch_dep(&project_dir, "bar", "2.0.0"); // 不在 lock → 裁剪
    lock_entry(&project_dir, "foo", "1.0.0");

    exec_in(&project_dir).unwrap();

    let manager = VendorManager::new(&project_dir);
    assert!(manager.is_installed("foo", "1.0.0"));
    assert!(!manager.is_installed("bar", "2.0.0"));
}

#[test]
fn test_clean_removes_build_dir() {
    let (_tmp, project_dir) = setup_project();

    let build_dir = project_dir.join(".yaoxiang").join("build");
    std::fs::create_dir_all(&build_dir).unwrap();
    std::fs::write(build_dir.join("main.42"), b"bytecode").unwrap();

    exec_in(&project_dir).unwrap();

    assert!(!build_dir.exists());
}

#[test]
fn test_clean_nothing_to_do() {
    let (_tmp, project_dir) = setup_project();
    exec_in(&project_dir).unwrap();
    // 不报错即通过（打印 "Nothing to clean."）
}
