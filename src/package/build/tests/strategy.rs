//! 构建策略与安装决策树测试 — 基于 RFC-014b
//!
//! 覆盖：
//! - §构建策略：`BuildStrategy` 枚举（none/cargo/cmake/custom），未知策略硬错误
//! - §安装决策树：预编译优先 → headers → 策略执行；无声明包零构建直过
//! - §平台标识：Rust target triple 形态（arch-vendor-os[-env]）
//! - 2026-09-15 决议 2：声明式先行、任意代码执行殿后（未落地分支如实报错）

use crate::package::build::{self, BuildStrategy, TrustDecision};
use crate::package::error::PackageError;
use crate::package::manifest::PackageManifest;

use super::write;

fn load(dir: &std::path::Path) -> PackageManifest {
    PackageManifest::load(dir).expect("load fixture manifest")
}

fn run_build(pkg: &std::path::Path) -> Result<build::BuildOutcome, PackageError> {
    let manifest = load(pkg);
    let scratch = pkg.join("scratch");
    crate::package::runtime::drive(build::run_install_build(
        pkg,
        &manifest,
        &scratch,
        None,
        &TrustDecision::default(),
    ))
}

/// Fixture: 写满 `[build]` 各段的清单（cargo 策略 / headers / requirements / platforms / binaries）。
fn full_build_manifest_dir() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("create temp dir 应成功");
    write(
        &tmp.path().join("yaoxiang.toml"),
        r#"[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"
headers = ["include/sqlite3.h"]

[build.cargo]
features = ["ffi"]
target = "release"

[build.requirements]
cargo = ">= 1.70"

[build.platforms]
"x86_64-unknown-linux-gnu" = { cargo-features = ["linux-ffi"] }
"x86_64-pc-windows-msvc" = { cargo-features = ["win-ffi"] }

[binaries]
"x86_64-pc-windows-msvc" = { url = "releases/download/v1.0.0/foo-win.tar.gz", sha256 = "abc123" }
"aarch64-apple-darwin" = { url = "https://example.com/foo-mac.tar.gz" }
"#,
    );

    tmp
}
/// Assert: `[build.requirements]` 与 `[build.platforms]` 的平台特化 cargo-features。
fn assert_requirements_and_platforms(build: &crate::package::manifest::BuildConfig) {
    assert_eq!(
        build
            .requirements
            .get("cargo")
            .expect("build.requirements.get(\"cargo\") 应成功"),
        ">= 1.70"
    );
    assert_eq!(
        build
            .platforms
            .get("x86_64-pc-windows-msvc")
            .expect("build.platforms.get(\"x86_64-pc-windows-msvc\") 应成功")
            .cargo_features,
        vec!["win-ffi"]
    );
}

/// Assert: `[binaries]` 的平台条目（Windows 带 sha256、macOS 缺省 sha256）。
fn assert_declared_binaries(manifest: &crate::package::manifest::PackageManifest) {
    assert_eq!(manifest.binaries.len(), 2);
    let win = manifest
        .binaries
        .get("x86_64-pc-windows-msvc")
        .expect("manifest.binaries 应含 x86_64-pc-windows-msvc");
    assert_eq!(win.sha256.as_deref(), Some("abc123"));
    let mac = manifest
        .binaries
        .get("aarch64-apple-darwin")
        .expect("manifest.binaries 应含 aarch64-apple-darwin");
    assert_eq!(mac.sha256, None, "sha256 可缺省（该平台预编译路径不可用）");
}

#[test]
fn test_parses_full_build_declaration() {
    // Arrange — 写满 [build] 各段的清单
    let tmp = full_build_manifest_dir();

    // Act
    let manifest = load(tmp.path());
    let build = manifest.build.as_ref().expect("[build] 段应被解析");

    // Assert — 策略 / headers / cargo 选项 / requirements / platforms / binaries
    assert_eq!(
        build
            .parsed_strategy()
            .expect("build.parsed_strategy() 应成功"),
        BuildStrategy::Cargo
    );
    assert_eq!(build.headers, vec!["include/sqlite3.h"]);
    let cargo = build.cargo.as_ref().expect("build.cargo 应存在");
    assert_eq!(cargo.features, vec!["ffi"]);
    assert_eq!(cargo.target.as_deref(), Some("release"));
    assert_requirements_and_platforms(build);
    assert_declared_binaries(&manifest);
}

#[test]
fn test_no_build_section_defaults_to_none() {
    // Arrange
    let tmp = tempfile::tempdir().unwrap();
    write(
        &tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"pure\"\nversion = \"0.1.0\"\n",
    );

    // Act
    let manifest = load(tmp.path());

    // Assert
    assert!(manifest.build.is_none(), "无 [build] 段时 build 应为 None");
    assert!(
        manifest.binaries.is_empty(),
        "无 [binaries] 段时 binaries 应为空"
    );
}

#[test]
fn test_unknown_strategy_is_hard_error() {
    // Arrange
    let tmp = tempfile::tempdir().unwrap();
    write(
        &tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n\n[build]\nstrategy = \"make\"\n",
    );

    // Act
    let manifest = load(tmp.path());
    let err = manifest
        .build
        .as_ref()
        .unwrap()
        .parsed_strategy()
        .unwrap_err();

    // Assert
    assert!(
        matches!(err, PackageError::InvalidManifest(ref m)
            if m.contains("none / cargo / cmake / custom")),
        "报错应列合法策略值: {err}"
    );
}

#[test]
fn test_current_triple_shape() {
    // Act
    let triple = build::current_triple();

    // Assert：arch 前缀 + 按 OS 的 vendor-os[-env] 形态（RFC-014b §平台标识）
    let arch = std::env::consts::ARCH;
    assert!(triple.starts_with(arch), "triple 应以 arch 开头: {triple}");
    match std::env::consts::OS {
        "windows" => assert!(
            triple.ends_with("-pc-windows-msvc") || triple.ends_with("-pc-windows-gnu"),
            "got: {triple}"
        ),
        "linux" => assert!(
            triple.ends_with("-unknown-linux-gnu") || triple.ends_with("-unknown-linux-musl"),
            "got: {triple}"
        ),
        "macos" => assert!(triple.ends_with("-apple-darwin"), "got: {triple}"),
        _ => {}
    }
}

#[test]
fn test_install_build_skips_plain_packages() {
    // Arrange：无 [build] 且无 [binaries] 的纯 .yx 包
    let tmp = tempfile::tempdir().unwrap();
    write(
        &tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"pure\"\nversion = \"0.1.0\"\n",
    );

    // Act
    let outcome = run_build(tmp.path()).unwrap();

    // Assert
    assert_eq!(outcome.via, "no-build");
    assert!(outcome.native_dir.is_none(), "纯 .yx 包不应有产物目录");
}

#[test]
fn test_install_build_unimplemented_reports_clear_errors() {
    // Arrange：headers 非空（yx-bindgen 随 RFC-026b，尚未实现）
    let tmp = tempfile::tempdir().unwrap();
    write(
        &tmp.path().join("yaoxiang.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n\n[build]\nheaders = [\"a.h\"]\n",
    );

    // Act
    let err = run_build(tmp.path()).unwrap_err();

    // Assert
    assert!(
        matches!(err, PackageError::InvalidManifest(ref m) if m.contains("yx-bindgen")),
        "got: {err}"
    );
}
