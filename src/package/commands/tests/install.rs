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
use crate::package::manifest::PackageManifest;
use tempfile::TempDir;

use super::add_path_dep;

fn setup_project() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    init::exec_in(tmp.path(), &init::InitOptions { lib: false }, "test-proj").unwrap();
    let project_dir = tmp.path().join("test-proj");
    (tmp, project_dir)
}

#[test]
fn test_install_empty() {
    let (_tmp, project_dir) = setup_project();
    exec_in(&project_dir).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert!(lock.package.is_empty());
}

#[test]
fn test_install_with_deps() {
    let (_tmp, project_dir) = setup_project();
    add_path_dep(&project_dir, "foo", "1.0.0", false);
    add_path_dep(&project_dir, "bar", "2.0.0", true);

    exec_in(&project_dir).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert!(lock.package.contains_key("foo"));
    assert!(lock.package.contains_key("bar"));
}

#[test]
fn test_install_updates_lock_correctly() {
    let (_tmp, project_dir) = setup_project();
    add_path_dep(&project_dir, "foo", "1.0.0", false);

    exec_in(&project_dir).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert_eq!(lock.package["foo"].version, "1.0.0");
}

#[test]
fn test_install_registry_dependency_fails() {
    // 注册表来源（无 git/path）尚未实现（RFC-014a）——必须明确失败，
    // 不再打印"已安装"并返回 Ok（旧行为：静默跳过 + 锁文件记账）
    let (_tmp, project_dir) = setup_project();
    add::exec_in(&project_dir, "foo", Some("1.0.0"), false).unwrap();

    let result = exec_in(&project_dir);

    assert!(
        result.is_err(),
        "registry dependency must fail loudly, got Ok"
    );
}

#[test]
fn test_install_local_dependency() {
    let (_tmp, project_dir) = setup_project();

    // 创建本地依赖目录
    let local_dep_dir = project_dir.join("local-dep");
    std::fs::create_dir_all(&local_dep_dir).unwrap();
    std::fs::write(local_dep_dir.join("lib.yx"), "export x = 42").unwrap();

    // 添加本地依赖
    let mut manifest = PackageManifest::load(&project_dir).unwrap();
    let mut dep_table = toml::map::Map::new();
    dep_table.insert(
        "version".to_string(),
        toml::Value::String("0.1.0".to_string()),
    );
    dep_table.insert(
        "path".to_string(),
        toml::Value::String("./local-dep".to_string()),
    );
    manifest
        .dependencies
        .insert("local-dep".to_string(), toml::Value::Table(dep_table));
    manifest.save(&project_dir).unwrap();

    exec_in(&project_dir).unwrap();

    let lock = LockFile::load(&project_dir).unwrap();
    assert!(lock.package.contains_key("local-dep"));
    assert_eq!(lock.package["local-dep"].source, "path");
}
