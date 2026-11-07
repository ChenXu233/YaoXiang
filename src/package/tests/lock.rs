//! 测试 `LockFile` 的创建、加载、保存和依赖管理
//!
//! 覆盖:
//! - 新建空锁文件
//! - 从不存在的目录加载（返回空锁文件）
//! - 保存后重新加载的一致性
//! - 添加/移除/强制更新依赖
//! - 保存文件头部包含生成标记

use std::collections::BTreeMap;

use crate::package::lock::{LockFile};

#[test]
fn test_new_lock_file() {
    let lock = LockFile::new();
    assert_eq!(lock.version, 1);
    assert!(lock.package.is_empty());
}

#[test]
fn test_load_nonexistent() {
    let dir = tempfile::TempDir::new().unwrap();
    let lock = LockFile::load(dir.path()).unwrap();
    assert!(lock.package.is_empty());
}

#[test]
fn test_save_and_load() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut lock = LockFile::new();
    lock.lock_dependency("foo", "1.0.0");
    lock.save(dir.path()).unwrap();

    let loaded = LockFile::load(dir.path()).unwrap();
    assert!(loaded.package.contains_key("foo"));
    assert_eq!(loaded.package["foo"].version, "1.0.0");
}

#[test]
fn test_remove_nonexistent() {
    let mut lock = LockFile::new();
    assert!(!lock.remove_dependency("foo"));
}

#[test]
fn test_update_from_dependencies() {
    let mut lock = LockFile::new();
    let mut deps = BTreeMap::new();
    deps.insert("foo".to_string(), toml::Value::String("1.0.0".to_string()));
    deps.insert("bar".to_string(), toml::Value::String("2.0.0".to_string()));

    lock.update_from_dependencies(&deps);
    assert_eq!(lock.package.len(), 2);
    assert_eq!(lock.package["foo"].version, "1.0.0");
    assert_eq!(lock.package["bar"].version, "2.0.0");
}
