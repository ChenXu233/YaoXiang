//! 测试 Vendor 目录管理器
//!
//! 覆盖:
//! - VendorManager 的创建和路径
//! - 确保 vendor 目录存在
//! - 依赖路径计算
//! - 已安装检查
//! - 多版本安装
//! - 列出已安装依赖
//! - 卸载依赖
//! - 清理依赖
//! - vendor 目录名称解析
//! - 空列表查询
//! - 完整性验证

use crate::package::vendor::{VendorManager, cache};
use tempfile::TempDir;

#[test]
fn test_vendor_manager_new() {
    let tmp = TempDir::new().unwrap();
    let manager = VendorManager::new(tmp.path());
    assert_eq!(
        manager.vendor_dir(),
        tmp.path().join(".yaoxiang").join("vendor")
    );
}

#[test]
fn test_ensure_vendor_dir() {
    let tmp = TempDir::new().unwrap();
    let manager = VendorManager::new(tmp.path());
    manager.ensure_vendor_dir().unwrap();
    assert!(manager.vendor_dir().exists());
}

#[test]
fn test_dep_path() {
    let tmp = TempDir::new().unwrap();
    let manager = VendorManager::new(tmp.path());
    let path = manager.dep_path("foo", "1.0.0");
    assert_eq!(
        path,
        tmp.path()
            .join(".yaoxiang")
            .join("vendor")
            .join("foo-1.0.0")
    );
}

#[test]
fn test_is_installed() {
    let tmp = TempDir::new().unwrap();
    let manager = VendorManager::new(tmp.path());
    assert!(!manager.is_installed("foo", "1.0.0"));

    // 创建目录
    manager.ensure_vendor_dir().unwrap();
    std::fs::create_dir_all(manager.dep_path("foo", "1.0.0")).unwrap();
    assert!(manager.is_installed("foo", "1.0.0"));
}

#[test]
fn test_install_multi_version() {
    let tmp = TempDir::new().unwrap();
    let manager = VendorManager::new(tmp.path());
    manager.ensure_vendor_dir().unwrap();

    // 模拟安装两个版本
    std::fs::create_dir_all(manager.dep_path("foo", "1.0.0")).unwrap();
    std::fs::create_dir_all(manager.dep_path("foo", "1.1.0")).unwrap();

    assert!(manager.is_installed("foo", "1.0.0"));
    assert!(manager.is_installed("foo", "1.1.0"));
    assert!(!manager.is_installed("foo", "2.0.0"));
}

#[test]
fn test_list_installed() {
    let tmp = TempDir::new().unwrap();
    let manager = VendorManager::new(tmp.path());
    manager.ensure_vendor_dir().unwrap();

    std::fs::create_dir_all(manager.dep_path("bar", "2.0.0")).unwrap();
    std::fs::create_dir_all(manager.dep_path("foo", "1.0.0")).unwrap();

    let installed = manager.list_installed().unwrap();
    assert_eq!(installed.len(), 2);
    assert_eq!(installed[0], ("bar".to_string(), "2.0.0".to_string()));
    assert_eq!(installed[1], ("foo".to_string(), "1.0.0".to_string()));
}

#[test]
fn test_uninstall_dependency() {
    let tmp = TempDir::new().unwrap();
    let manager = VendorManager::new(tmp.path());
    manager.ensure_vendor_dir().unwrap();

    std::fs::create_dir_all(manager.dep_path("foo", "1.0.0")).unwrap();
    assert!(manager.is_installed("foo", "1.0.0"));

    let removed = manager.uninstall_dependency("foo", "1.0.0").unwrap();
    assert!(removed);
    assert!(!manager.is_installed("foo", "1.0.0"));
}

#[test]
fn test_clean() {
    let tmp = TempDir::new().unwrap();
    let manager = VendorManager::new(tmp.path());
    manager.ensure_vendor_dir().unwrap();

    std::fs::create_dir_all(manager.dep_path("foo", "1.0.0")).unwrap();
    std::fs::create_dir_all(manager.dep_path("bar", "2.0.0")).unwrap();
    std::fs::create_dir_all(manager.dep_path("baz", "3.0.0")).unwrap();

    let keep = vec![("foo".to_string(), "1.0.0".to_string())];
    let removed = manager.clean(&keep).unwrap();

    assert_eq!(removed.len(), 2);
    assert!(manager.is_installed("foo", "1.0.0"));
    assert!(!manager.is_installed("bar", "2.0.0"));
    assert!(!manager.is_installed("baz", "3.0.0"));
}

#[test]
fn test_parse_vendor_dir_name() {
    use crate::package::vendor::parse_vendor_dir_name;
    assert_eq!(
        parse_vendor_dir_name("foo-1.0.0"),
        Some(("foo".to_string(), "1.0.0".to_string()))
    );
    assert_eq!(
        parse_vendor_dir_name("my-lib-2.3.0"),
        Some(("my-lib".to_string(), "2.3.0".to_string()))
    );
    assert_eq!(parse_vendor_dir_name("invalid"), None);
    assert_eq!(parse_vendor_dir_name("-1.0.0"), None);
}

#[test]
fn test_list_installed_empty() {
    let tmp = TempDir::new().unwrap();
    let manager = VendorManager::new(tmp.path());
    let installed = manager.list_installed().unwrap();
    assert!(installed.is_empty());
}

#[test]
fn test_verify_integrity() {
    let tmp = TempDir::new().unwrap();
    let manager = VendorManager::new(tmp.path());
    manager.ensure_vendor_dir().unwrap();

    // 创建测试文件
    let dep_path = manager.dep_path("foo", "1.0.0");
    std::fs::create_dir_all(&dep_path).unwrap();
    std::fs::write(dep_path.join("lib.yx"), "main = { 42 }").unwrap();

    // 计算校验和
    let checksum = cache::compute_directory_checksum(&dep_path).unwrap();

    // 校验应通过
    assert!(manager.verify_integrity("foo", "1.0.0", &checksum).unwrap());

    // 修改文件后校验应失败
    std::fs::write(dep_path.join("lib.yx"), "main = { 0 }").unwrap();
    assert!(!manager.verify_integrity("foo", "1.0.0", &checksum).unwrap());
}
