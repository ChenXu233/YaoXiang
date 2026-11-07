//! 测试 `yaoxiang update` 命令
//!
//! 覆盖:
//! - 无依赖时更新不报错
//! - 添加依赖后更新锁文件
//! - 更新后版本刷新
//! - 单个依赖更新

use crate::package::commands::init;
use crate::package::commands::update::{exec_in};
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
fn test_update_with_deps() {
    let (_tmp, project_dir) = setup_project();
    add_path_dep(&project_dir, "foo", "1.0.0", false);
    add_path_dep(&project_dir, "bar", "2.0.0", false);

    exec_in(&project_dir, &Default::default()).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert_eq!(lock.package.len(), 2);
    assert!(lock.package.contains_key("foo"));
    assert!(lock.package.contains_key("bar"));
}
