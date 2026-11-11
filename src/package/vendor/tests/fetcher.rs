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

/// 夹具：`local-dep` 清单（版本 0.3.0）。
const MANIFEST_0_3_0: &str = "[package]\nname = \"local-dep\"\nversion = \"0.3.0\"\n";

/// Fixture: 建本地依赖 `tmp/local-dep`——写入 `(相对路径, 内容)` 文件（自动建父目录），
/// 并额外创建 `dirs` 中的空目录。返回本地依赖目录。
fn local_dep_fixture(
    tmp: &std::path::Path,
    files: &[(&str, &str)],
    dirs: &[&str],
) -> std::path::PathBuf {
    let local_dep = tmp.join("local-dep");
    for d in dirs {
        std::fs::create_dir_all(local_dep.join(d)).expect("创建 fixture 目录应成功");
    }
    for (rel, content) in files {
        let path = local_dep.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("创建 fixture 父目录应成功");
        }
        std::fs::write(&path, content).expect("写入 fixture 文件应成功");
    }
    local_dep
}

/// Fixture: 组装单条 `local-dep` path 依赖（`version` 为 spec 版本声明，如 `0.3.0` / `*`）。
fn path_dep_of(
    local_dep: &std::path::Path,
    version: &str,
) -> BTreeMap<String, toml::Value> {
    let mut dep_table = toml::map::Map::new();
    dep_table.insert(
        "version".to_string(),
        toml::Value::String(version.to_string()),
    );
    dep_table.insert(
        "path".to_string(),
        toml::Value::String(local_dep.to_string_lossy().to_string()),
    );
    let mut deps = BTreeMap::new();
    deps.insert("local-dep".to_string(), toml::Value::Table(dep_table));
    deps
}

/// 落盘副本路径 `.yaoxiang/vendor/<目录名>`。
fn vendored_path(
    root: &std::path::Path,
    dir_name: &str,
) -> std::path::PathBuf {
    root.join(".yaoxiang").join("vendor").join(dir_name)
}

/// Assert: vendor 副本内容正确——含包文件且不含派生/状态目录。
fn assert_vendored_copy_clean(
    root: &std::path::Path,
    dir_name: &str,
) {
    let vendored = vendored_path(root, dir_name);
    assert!(vendored.join("lib.yx").exists(), "包文件应复制进 vendor");
    assert!(!vendored.join("target").exists(), "派生目录不入副本");
    assert!(
        !vendored.join(".yaoxiang").exists(),
        "vendor 状态目录不入副本"
    );
}

/// Act: 跑一次 `fetch_all`（失败即 panic，带底层错误）。
fn fetch_ok(
    root: &std::path::Path,
    deps: &BTreeMap<String, toml::Value>,
    lock: &mut LockFile,
) -> crate::package::vendor::fetcher::FetchResult {
    crate::package::runtime::drive(fetch_all(root, deps, lock, &Default::default()))
        .expect("fetch_all 应成功")
}

#[test]
fn test_fetch_local_dep_vendors_copy() {
    // Arrange：本地路径依赖（带清单 + 派生/状态目录）
    let tmp = TempDir::new().unwrap();
    let local_dep = local_dep_fixture(
        tmp.path(),
        &[
            ("yaoxiang.toml", MANIFEST_0_3_0),
            ("lib.yx", "x = 42\n"),
            ("target/junk.bin", "x"),
        ],
        &["src", "target", ".yaoxiang/vendor"],
    );
    // spec version 与清单版本不同：vendor 目录名以清单真实版本为准
    let deps = path_dep_of(&local_dep, "0.1.0");
    let mut lock = LockFile::new();

    // Act
    let result = fetch_ok(tmp.path(), &deps, &mut lock);

    // Assert：进 installed 且落盘 vendor（#411：不再「只登记 lock 不下载」）
    assert_eq!(result.installed.len(), 1, "path 依赖应走安装落盘");
    assert_eq!(
        result.installed[0].version, "0.3.0",
        "vendor 名以清单真实版本为准"
    );
    assert_vendored_copy_clean(tmp.path(), "local-dep-0.3.0");
    // lock：真实版本 + path 来源 + 校验和
    let entry = lock.package.get("local-dep").expect("path 依赖应登记 lock");
    assert_eq!(entry.version, "0.3.0");
    assert_eq!(entry.source, "path");
    assert!(entry.checksum.is_some(), "落盘副本应有完整性校验和");
}
