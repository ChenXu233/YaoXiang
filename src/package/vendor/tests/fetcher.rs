//! 测试依赖下载器
//!
//! 规范来源：RFC-014 §依赖下载（vendor 落盘与 lock 记账、目录名 = 探测版本）；
//! #411 定案——path 依赖复制进 vendor（此前只登记 lock 不下载）
//!
//! 覆盖:
//! - 空依赖列表下载
//! - 本地路径依赖落盘 vendor（目录名 = 清单真实版本）
//! - 本地依赖新鲜度（改源重装即生效）与派生/状态目录排除
//! - 无清单回退 spec 版本；`*` + 无清单明确失败

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
fn test_fetch_local_dep_vendors_copy() {
    // Arrange：本地路径依赖（带清单 + 派生/状态目录）
    let tmp = TempDir::new().unwrap();
    let local_dep = tmp.path().join("local-dep");
    std::fs::create_dir_all(local_dep.join("src")).unwrap();
    std::fs::create_dir_all(local_dep.join("target")).unwrap();
    std::fs::create_dir_all(local_dep.join(".yaoxiang").join("vendor")).unwrap();
    std::fs::write(
        local_dep.join("yaoxiang.toml"),
        "[package]\nname = \"local-dep\"\nversion = \"0.3.0\"\n",
    )
    .unwrap();
    std::fs::write(local_dep.join("lib.yx"), "x = 42\n").unwrap();
    std::fs::write(local_dep.join("target").join("junk.bin"), "x").unwrap();

    let mut deps = BTreeMap::new();
    let mut dep_table = toml::map::Map::new();
    // spec version 与清单版本不同：vendor 目录名以清单真实版本为准
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

    // Assert：进 installed 且落盘 vendor（#411：不再「只登记 lock 不下载」）
    assert_eq!(result.installed.len(), 1, "path 依赖应走安装落盘");
    assert_eq!(
        result.installed[0].version, "0.3.0",
        "vendor 名以清单真实版本为准"
    );
    let vendored = tmp
        .path()
        .join(".yaoxiang")
        .join("vendor")
        .join("local-dep-0.3.0");
    assert!(vendored.join("lib.yx").exists(), "包文件应复制进 vendor");
    assert!(!vendored.join("target").exists(), "派生目录不入副本");
    assert!(
        !vendored.join(".yaoxiang").exists(),
        "vendor 状态目录不入副本"
    );
    // lock：真实版本 + path 来源 + 校验和
    let entry = lock.package.get("local-dep").expect("path 依赖应登记 lock");
    assert_eq!(entry.version, "0.3.0");
    assert_eq!(entry.source, "path");
    assert!(entry.checksum.is_some(), "落盘副本应有完整性校验和");
}

#[test]
fn test_fetch_local_dep_refreshes_on_reinstall() {
    // Arrange：本地依赖 + `*` 版本声明（add 的默认形态）
    let tmp = TempDir::new().unwrap();
    let local_dep = tmp.path().join("local-dep");
    std::fs::create_dir_all(&local_dep).unwrap();
    std::fs::write(
        local_dep.join("yaoxiang.toml"),
        "[package]\nname = \"local-dep\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(local_dep.join("lib.yx"), "x = 1\n").unwrap();

    let mut deps = BTreeMap::new();
    let mut dep_table = toml::map::Map::new();
    dep_table.insert("version".to_string(), toml::Value::String("*".to_string()));
    dep_table.insert(
        "path".to_string(),
        toml::Value::String(local_dep.to_string_lossy().to_string()),
    );
    deps.insert("local-dep".to_string(), toml::Value::Table(dep_table));
    let mut lock = LockFile::new();

    let vendored_lib = tmp
        .path()
        .join(".yaoxiang")
        .join("vendor")
        .join("local-dep-0.1.0")
        .join("lib.yx");

    // Act：首次安装
    let first = crate::package::runtime::drive(fetch_all(
        tmp.path(),
        &deps,
        &mut lock,
        &Default::default(),
    ))
    .unwrap();

    // Assert：vendor 副本反映当前工作副本
    assert_eq!(first.installed.len(), 1, "首次安装应进 installed");
    assert_eq!(
        std::fs::read_to_string(&vendored_lib).unwrap(),
        "x = 1\n",
        "首次安装的副本应为 v1 内容"
    );

    // Act：修改依赖源后再次 install（新鲜度语义：每次重做复制）
    std::fs::write(local_dep.join("lib.yx"), "x = 2\n").unwrap();
    let second = crate::package::runtime::drive(fetch_all(
        tmp.path(),
        &deps,
        &mut lock,
        &Default::default(),
    ))
    .unwrap();

    // Assert：副本更新为最新内容，不残留 v1
    assert_eq!(second.installed.len(), 1, "path 依赖每次 install 都应重装");
    assert_eq!(
        std::fs::read_to_string(&vendored_lib).unwrap(),
        "x = 2\n",
        "vendor 副本应反映工作副本的最新内容"
    );
}

#[test]
fn test_fetch_local_dep_manifestless_spec_version() {
    // Arrange：路径存在但没有 yaoxiang.toml——回退 spec 版本命名 vendor
    // （既有工作空间/安装夹具的纯源码目录形态）
    let tmp = TempDir::new().unwrap();
    let local_dep = tmp.path().join("local-dep");
    std::fs::create_dir_all(&local_dep).unwrap();
    std::fs::write(local_dep.join("lib.yx"), "x = 42\n").unwrap();

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

    // Assert：按 spec 版本落盘并登记 lock
    assert_eq!(result.installed.len(), 1);
    assert!(
        tmp.path()
            .join(".yaoxiang")
            .join("vendor")
            .join("local-dep-0.1.0")
            .join("lib.yx")
            .exists(),
        "纯源码 path 源也应落盘 vendor"
    );
    let entry = lock.package.get("local-dep").expect("应登记 lock");
    assert_eq!(entry.version, "0.1.0");
}

#[test]
fn test_fetch_local_dep_manifestless_star_fails() {
    // Arrange：`*` + 无清单——vendor 目录名无从确定，明确失败
    let tmp = TempDir::new().unwrap();
    let local_dep = tmp.path().join("local-dep");
    std::fs::create_dir_all(&local_dep).unwrap();
    std::fs::write(local_dep.join("lib.yx"), "x = 42\n").unwrap();

    let mut deps = BTreeMap::new();
    let mut dep_table = toml::map::Map::new();
    dep_table.insert("version".to_string(), toml::Value::String("*".to_string()));
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

    // Assert：不落 vendor、不装成功、不登记 lock
    assert_eq!(result.installed.len(), 0);
    assert_eq!(result.failed.len(), 1);
    assert!(
        result.failed[0].1.contains("无法确定版本"),
        "unexpected: {}",
        result.failed[0].1
    );
    assert!(
        !lock.package.contains_key("local-dep"),
        "失败依赖不得登记 lock"
    );
}
