//! 测试 `yaoxiang install` 命令
//!
//! 覆盖:
//! - 无依赖时安装不报错
//! - 添加依赖后安装更新锁文件
//! - 安装后锁文件版本正确
//! - 本地路径依赖的安装

use crate::package::commands::add;
use crate::package::commands::init;
use crate::package::commands::install::exec_in;
use crate::package::lock::LockFile;
use tempfile::TempDir;

use super::add_path_dep;

fn setup_project() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    init::exec_in(tmp.path(), &init::InitOptions { lib: false }, "test-proj").unwrap();
    let project_dir = tmp.path().join("test-proj");
    (tmp, project_dir)
}

#[test]
fn test_install_updates_lock_correctly() {
    let (_tmp, project_dir) = setup_project();
    add_path_dep(&project_dir, "foo", "1.0.0", false);

    exec_in(&project_dir, &Default::default()).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert_eq!(lock.package["foo"].version, "1.0.0");
}

#[test]
fn test_install_registry_dependency_fails() {
    // 注册表来源（无 git/path）尚未实现（RFC-014a）——必须明确失败，
    // 不再打印"已安装"并返回 Ok（旧行为：静默跳过 + 锁文件记账）
    let (_tmp, project_dir) = setup_project();
    add::exec_in(&project_dir, "foo", Some("1.0.0"), None, None, false, false).unwrap();

    let result = exec_in(&project_dir, &Default::default());

    assert!(
        result.is_err(),
        "registry dependency must fail loudly, got Ok"
    );
}
