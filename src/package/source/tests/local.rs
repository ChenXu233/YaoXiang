//! LocalSource 单元测试 — path 依赖落盘 vendor 契约（#411）

use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::package::dependency::DependencySpec;
use crate::package::source::{LocalSource, Source};

fn spec(path: &Path) -> DependencySpec {
    DependencySpec {
        name: "local-dep".to_string(),
        version: "*".to_string(),
        git: None,
        path: Some(path.to_string_lossy().to_string()),
    }
}

/// 搭一个最小依赖包：清单 + 源文件 + 派生/状态目录
fn setup_dep() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let dep = tmp.path().join("dep");
    std::fs::create_dir_all(dep.join("src")).unwrap();
    std::fs::create_dir_all(dep.join("target")).unwrap();
    std::fs::create_dir_all(dep.join(".yaoxiang")).unwrap();
    std::fs::write(
        dep.join("yaoxiang.toml"),
        "[package]\nname = \"local-dep\"\nversion = \"0.3.0\"\n",
    )
    .unwrap();
    std::fs::write(dep.join("src").join("lib.yx"), "x = 42\n").unwrap();
    std::fs::write(dep.join("target").join("junk.bin"), "x").unwrap();
    (tmp, dep)
}

#[test]
fn download_copies_into_dest_with_probed_version() {
    // Arrange
    let (_tmp, dep) = setup_dep();
    let dest = TempDir::new().unwrap();

    // Act
    let resolved =
        crate::package::runtime::drive(LocalSource::new().download(&spec(&dep), dest.path()))
            .unwrap();

    // Assert：目录名 = 清单真实版本（非 spec 的 `*`），local_path 指向副本
    let vendored = dest.path().join("local-dep-0.3.0");
    assert_eq!(resolved.version, "0.3.0");
    assert_eq!(resolved.local_path, vendored);
    assert!(vendored.join("yaoxiang.toml").exists());
    assert!(vendored.join("src").join("lib.yx").exists());
}

#[test]
fn download_excludes_derived_and_state_dirs() {
    let (_tmp, dep) = setup_dep();
    let dest = TempDir::new().unwrap();

    crate::package::runtime::drive(LocalSource::new().download(&spec(&dep), dest.path())).unwrap();

    let vendored = dest.path().join("local-dep-0.3.0");
    assert!(!vendored.join("target").exists(), "派生目录不入副本");
    assert!(
        !vendored.join(".yaoxiang").exists(),
        "vendor 状态目录不入副本"
    );
}

#[test]
fn download_replaces_stale_copy() {
    // 重装语义：目标已存在（旧副本）时先清后拷，内容以最新工作副本为准
    let (_tmp, dep) = setup_dep();
    let dest = TempDir::new().unwrap();
    let vendored_lib = dest
        .path()
        .join("local-dep-0.3.0")
        .join("src")
        .join("lib.yx");
    std::fs::create_dir_all(dest.path().join("local-dep-0.3.0").join("src")).unwrap();
    std::fs::write(&vendored_lib, "x = 0\n").unwrap();

    std::fs::write(dep.join("src").join("lib.yx"), "x = 42\n").unwrap();
    crate::package::runtime::drive(LocalSource::new().download(&spec(&dep), dest.path())).unwrap();

    assert_eq!(std::fs::read_to_string(vendored_lib).unwrap(), "x = 42\n");
}

#[test]
fn download_missing_path_errors() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("no-such-dir");
    let dest = TempDir::new().unwrap();

    let result =
        crate::package::runtime::drive(LocalSource::new().download(&spec(&missing), dest.path()));
    assert!(result.is_err(), "路径不存在应报错");
    assert!(!dest.path().join("local-dep-0.3.0").exists());
}

#[test]
fn download_manifestless_dep_falls_back_to_spec_version() {
    // 无清单的纯源码 path 源（既有工作空间/安装夹具形态）：回退 spec 版本
    let tmp = TempDir::new().unwrap();
    let dep = tmp.path().join("dep");
    std::fs::create_dir_all(&dep).unwrap();
    std::fs::write(dep.join("lib.yx"), "x = 42\n").unwrap();
    let dest = TempDir::new().unwrap();

    let mut s = spec(&dep);
    s.version = "1.2.0".to_string();
    let resolved = crate::package::runtime::drive(LocalSource::new().download(&s, dest.path()))
        .expect("无清单但有具体版本应可安装");
    assert_eq!(resolved.version, "1.2.0");
    assert!(dest.path().join("local-dep-1.2.0").join("lib.yx").exists());
}

#[test]
fn download_manifestless_star_version_errors() {
    // `*` + 无清单：vendor 目录名无从确定，明确失败，不落 `<name>-*` 目录
    let tmp = TempDir::new().unwrap();
    let dep = tmp.path().join("dep");
    std::fs::create_dir_all(&dep).unwrap();
    std::fs::write(dep.join("lib.yx"), "x = 42\n").unwrap();
    let dest = TempDir::new().unwrap();

    let result =
        crate::package::runtime::drive(LocalSource::new().download(&spec(&dep), dest.path()));
    assert!(result.is_err(), "`*` + 缺清单应报错");
    assert!(!dest.path().join("local-dep-*").exists());
}
