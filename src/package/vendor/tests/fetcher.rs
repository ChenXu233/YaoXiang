//! 测试依赖下载器
//!
//! 覆盖:
//! - 空依赖列表下载
//! - 本地路径依赖下载（跳过）

use std::collections::BTreeMap;

use crate::package::lock::LockFile;
use crate::package::vendor::fetcher::fetch_all;
use tempfile::TempDir;

#[test]
fn test_fetch_empty_deps() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let deps = BTreeMap::new();
    let mut lock = LockFile::new();

    // Act
    let result = crate::package::runtime::drive(fetch_all(
        tmp.path(),
        &deps,
        &mut lock,
        &Default::default(),
    ))
    .unwrap();

    // Assert：空依赖 → 三个结果桶全空
    assert!(result.installed.is_empty(), "不应有安装项");
    assert!(result.skipped.is_empty(), "不应有跳过项");
    assert!(result.failed.is_empty(), "不应有失败项");
}

#[test]
fn test_fetch_local_dep() {
    // Arrange：本地路径依赖（不需要下载）
    let tmp = TempDir::new().unwrap();
    let local_dep = tmp.path().join("local-dep");
    std::fs::create_dir_all(&local_dep).unwrap();
    std::fs::write(local_dep.join("lib.yx"), "export x = 42").unwrap();

    let mut deps = BTreeMap::new();
    let mut dep_table = toml::map::Map::new();
    dep_table.insert(
        "version".to_string(),
        toml::Value::String("0.1.0".to_string()),
    );
    dep_table.insert(
        "path".to_string(),
        toml::Value::String(local_dep.to_string_lossy().to_string()),
    );
    deps.insert("local-dep".to_string(), toml::Value::Table(dep_table));
    let mut lock = LockFile::new();

    // Act
    let result = crate::package::runtime::drive(fetch_all(
        tmp.path(),
        &deps,
        &mut lock,
        &Default::default(),
    ))
    .unwrap();

    // Assert：本地依赖登记为 skipped，不进 installed
    assert_eq!(result.skipped.len(), 1);
    assert_eq!(result.skipped[0].0, "local-dep");
}
