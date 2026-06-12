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
    let tmp = TempDir::new().unwrap();
    let deps = BTreeMap::new();
    let mut lock = LockFile::new();

    let result = fetch_all(tmp.path(), &deps, &mut lock).unwrap();
    assert!(result.installed.is_empty());
    assert!(result.skipped.is_empty());
    assert!(result.failed.is_empty());
}

#[test]
fn test_fetch_local_dep() {
    let tmp = TempDir::new().unwrap();

    // 创建本地依赖目录
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
    let result = fetch_all(tmp.path(), &deps, &mut lock).unwrap();
    assert_eq!(result.skipped.len(), 1);
    assert_eq!(result.skipped[0].0, "local-dep");
}
