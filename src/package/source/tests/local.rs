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
