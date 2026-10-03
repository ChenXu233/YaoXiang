//! LocalSource 单元测试 — path 依赖落盘 vendor 契约
//!
//! 规范来源：RFC-014 §依赖来源 / §项目模式——vendor 目录布局 `<name>-<version>`、
//! 版本探测与 GitSource「目录名 = 探测版本」同源、lock 记账。
//! #411 定案（用户裁决）：path 依赖复制进 vendor，排除 .git/.yaoxiang/target/build
//! （与 checksum 模块「派生产物不入校验」及 cache copy 排除规则对齐）。

use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;
use crate::package::source::{LocalSource, ResolvedPackage, Source};

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
    let tmp = TempDir::new().expect("create tempdir");
    let dep = tmp.path().join("dep");
    std::fs::create_dir_all(dep.join("src")).expect("create dep src dir");
    std::fs::create_dir_all(dep.join("target")).expect("create dep target dir");
    std::fs::create_dir_all(dep.join(".yaoxiang")).expect("create dep .yaoxiang dir");
    std::fs::write(
        dep.join("yaoxiang.toml"),
        "[package]\nname = \"local-dep\"\nversion = \"0.3.0\"\n",
    )
    .unwrap_or_else(|e| panic!("write dep manifest: {e}"));
    std::fs::write(dep.join("src").join("lib.yx"), "x = 42\n")
        .unwrap_or_else(|e| panic!("write dep source: {e}"));
    std::fs::write(dep.join("target").join("junk.bin"), "x")
        .unwrap_or_else(|e| panic!("write derived junk: {e}"));
    (tmp, dep)
}

/// 执行 LocalSource::download（经包管理运行时驱动）
fn download(
    dep_path: &Path,
    dest: &TempDir,
) -> PackageResult<ResolvedPackage> {
    crate::package::runtime::drive(LocalSource::new().download(&spec(dep_path), dest.path()))
}

#[test]
fn test_download_copies_into_dest_with_probed_version() {
    // Arrange
    let (_tmp, dep) = setup_dep();
    let dest = TempDir::new().expect("create dest tempdir");

    // Act
    let resolved = download(&dep, &dest).expect("落盘复制应成功");

    // Assert：目录名 = 清单真实版本（非 spec 的 `*`），local_path 指向副本
    let vendored = dest.path().join("local-dep-0.3.0");
    assert_eq!(resolved.version, "0.3.0", "vendor 名应为清单真实版本");
    assert_eq!(
        resolved.local_path, vendored,
        "local_path 应指向 vendor 副本"
    );
    assert!(
        vendored.join("yaoxiang.toml").exists(),
        "清单应复制进 vendor"
    );
    assert!(
        vendored.join("src").join("lib.yx").exists(),
        "源文件应复制进 vendor"
    );
}

#[test]
fn test_download_excludes_derived_and_state_dirs() {
    // Arrange
    let (_tmp, dep) = setup_dep();
    let dest = TempDir::new().expect("create dest tempdir");

    // Act
    download(&dep, &dest).expect("落盘复制应成功");

    // Assert
    let vendored = dest.path().join("local-dep-0.3.0");
    assert!(!vendored.join("target").exists(), "派生目录不入副本");
    assert!(
        !vendored.join(".yaoxiang").exists(),
        "vendor 状态目录不入副本"
    );
}

#[test]
fn test_download_replaces_stale_copy() {
    // Arrange：vendor 已有旧副本，依赖源已更新（重装语义：先清后拷）
    let (_tmp, dep) = setup_dep();
    let dest = TempDir::new().expect("create dest tempdir");
    let vendored_lib = dest
        .path()
        .join("local-dep-0.3.0")
        .join("src")
        .join("lib.yx");
    std::fs::create_dir_all(dest.path().join("local-dep-0.3.0").join("src"))
        .expect("create stale copy dir");
    std::fs::write(&vendored_lib, "x = 0\n").expect("write stale copy");
    std::fs::write(dep.join("src").join("lib.yx"), "x = 42\n").expect("update dep source");

    // Act
    download(&dep, &dest).expect("重装应成功");

    // Assert：副本内容以最新工作副本为准
    assert_eq!(
        std::fs::read_to_string(vendored_lib).unwrap(),
        "x = 42\n",
        "重装后 vendor 副本应为最新内容"
    );
}

#[test]
fn test_download_missing_path_errors() {
    // Arrange：声明的路径不存在
    let tmp = TempDir::new().expect("create tempdir");
    let missing = tmp.path().join("no-such-dir");
    let dest = TempDir::new().expect("create dest tempdir");

    // Act
    let result = download(&missing, &dest);

    // Assert：报错且不落盘
    assert!(result.is_err(), "路径不存在应报错");
    assert!(
        !dest.path().join("local-dep-0.3.0").exists(),
        "失败时不应落盘 vendor"
    );
}

#[test]
fn test_download_manifestless_dep_falls_back_to_spec_version() {
    // Arrange：无清单的纯源码 path 源（既有工作空间/安装夹具形态），spec 给具体版本
    let tmp = TempDir::new().expect("create tempdir");
    let dep = tmp.path().join("dep");
    std::fs::create_dir_all(&dep).expect("create dep dir");
    std::fs::write(dep.join("lib.yx"), "x = 42\n").expect("write dep source");
    let dest = TempDir::new().expect("create dest tempdir");
    let mut s = spec(&dep);
    s.version = "1.2.0".to_string();

    // Act
    let resolved = crate::package::runtime::drive(LocalSource::new().download(&s, dest.path()))
        .expect("无清单但有具体版本应可安装");

    // Assert：按 spec 版本命名 vendor 目录
    assert_eq!(resolved.version, "1.2.0", "应回退 spec 版本");
    assert!(
        dest.path().join("local-dep-1.2.0").join("lib.yx").exists(),
        "应按 spec 版本落盘 vendor"
    );
}

#[test]
fn test_download_manifestless_star_version_errors() {
    // Arrange：`*` + 无清单——vendor 目录名无从确定
    let tmp = TempDir::new().expect("create tempdir");
    let dep = tmp.path().join("dep");
    std::fs::create_dir_all(&dep).expect("create dep dir");
    std::fs::write(dep.join("lib.yx"), "x = 42\n").expect("write dep source");
    let dest = TempDir::new().expect("create dest tempdir");

    // Act
    let result = download(&dep, &dest);

    // Assert：明确失败，不落 `<name>-*` 目录
    assert!(result.is_err(), "`*` + 缺清单应报错");
    assert!(
        !dest.path().join("local-dep-*").exists(),
        "不应落无法被识别的 `*` 目录"
    );
}
