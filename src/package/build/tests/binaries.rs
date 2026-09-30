//! 预编译二进制分发测试 — 基于 RFC-014b §预编译二进制声明 + §安装决策树
//!
//! 覆盖：预编译优先（当前平台条目「下载成功 + SHA-256 校验通过」才命中）、
//! 决策树回退语义（条件不满足 → 源码构建，回退前清理半成品）、相对 URL 相对
//! 包仓库基址解析。决议 3（[binaries] 是唯一二进制分发机制）、决议 5（产物
//! 大小不设上限，仅解压护栏）。

use crate::package::build::{self, TrustDecision};
use crate::package::runtime;
use crate::package::tests::mock_http::{MockApi, MockResp};
use crate::package::vendor::checksum::compute_file_checksum;

use super::write;

/// 构造真实 tar.gz（单文件条目），返回字节与整包 SHA-256
fn tarball(
    dir: &std::path::Path,
    entry: &str,
    content: &[u8],
) -> (Vec<u8>, String) {
    let payload = dir.join("payload.tmp");
    std::fs::write(&payload, content).expect("write tarball payload");
    let tarball = dir.join("artifact.tar.gz");
    let gz = flate2::write::GzEncoder::new(
        std::fs::File::create(&tarball).expect("create tarball file"),
        flate2::Compression::default(),
    );
    let mut builder = tar::Builder::new(gz);
    builder
        .append_path_with_name(&payload, entry)
        .expect("append tarball entry");
    builder.into_inner().unwrap().finish().unwrap();
    let bytes = std::fs::read(&tarball).expect("read tarball bytes");
    let sha = compute_file_checksum(&tarball).expect("checksum tarball");
    (bytes, sha)
}

fn manifest_with_binary(
    pkg: &std::path::Path,
    url_expr: &str,
    sha: &str,
) {
    write(
        &pkg.join("yaoxiang.toml"),
        &format!(
            "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n\n[binaries]\n\"{}\" = {{ url = \"{}\", sha256 = \"{}\" }}\n",
            build::current_triple(),
            url_expr,
            sha
        ),
    );
}

fn run(
    pkg: &std::path::Path,
    source_base: Option<&str>,
) -> build::BuildOutcome {
    let manifest =
        crate::package::manifest::PackageManifest::load(pkg).expect("load fixture manifest");
    runtime::drive(build::run_install_build(
        pkg,
        &manifest,
        &pkg.parent().unwrap().join("scratch"),
        source_base,
        &TrustDecision::default(),
    ))
    .expect("run_install_build should not hard-fail on fallback paths")
}

#[test]
fn test_binaries_prebuilt_downloads_verifies_and_extracts() {
    // Arrange：当前平台条目，mock 返回合法 tarball，sha256 与整包一致
    let tmp = tempfile::tempdir().unwrap();
    let (tarball_bytes, sha) = tarball(tmp.path(), "native_demo.dll", b"BINARY");
    let api = MockApi::spawn(vec![MockResp::bytes(200, tarball_bytes)]);
    let pkg = tmp.path().join("demo-1.0.0");
    manifest_with_binary(&pkg, &format!("{}/artifact.tar.gz", api.url), &sha);

    // Act
    let outcome = run(&pkg, None);

    // Assert：走 prebuilt，产物解包到 build/native/<triple>/
    assert_eq!(outcome.via, "prebuilt");
    let native = outcome.native_dir.expect("应有产物目录");
    assert_eq!(
        std::fs::read(native.join("native_demo.dll")).expect("read extracted artifact"),
        b"BINARY"
    );
    assert_eq!(api.recorded().len(), 1);
    api.handle.join().unwrap();
}

#[test]
fn test_binaries_checksum_mismatch_falls_back_to_source_build() {
    // Arrange：sha256 与实际内容不符（决议：「否则 fallback 到源码构建」）
    let tmp = tempfile::tempdir().unwrap();
    let api = MockApi::spawn(vec![MockResp::bytes(200, b"TAMPERED".to_vec())]);
    let pkg = tmp.path().join("demo-1.0.0");
    manifest_with_binary(&pkg, &format!("{}/a.tar.gz", api.url), &"0".repeat(64));

    // Act
    let outcome = run(&pkg, None);

    // Assert：回退到源码构建（无 [build] → 策略 none），半成品被清理
    assert_eq!(outcome.via, "strategy-none");
    assert!(
        !pkg.join("build").exists(),
        "回退前应清理预编译半成品，避免与源码构建产物混装"
    );
    api.handle.join().unwrap();
}

#[test]
fn test_binaries_relative_url_uses_source_base() {
    // Arrange：相对 URL（releases/...）相对包仓库基址解析
    let tmp = tempfile::tempdir().unwrap();
    let (tarball_bytes, sha) = tarball(tmp.path(), "payload.bin", b"x");
    let api = MockApi::spawn(vec![MockResp::bytes(200, tarball_bytes)]);
    let pkg = tmp.path().join("demo-1.0.0");
    manifest_with_binary(&pkg, "releases/v1/demo.tar.gz", &sha);

    // Act
    let outcome = run(&pkg, Some(&api.url));

    // Assert
    assert_eq!(outcome.via, "prebuilt");
    assert_eq!(
        api.recorded()[0].path,
        "/releases/v1/demo.tar.gz",
        "相对 URL 应拼到仓库基址后"
    );
    api.handle.join().unwrap();
}

#[test]
fn test_binaries_other_platform_falls_back() {
    // Arrange：条目不在当前平台 → 不发请求，直接回退
    let tmp = tempfile::tempdir().unwrap();
    let other = if build::current_triple().contains("windows") {
        "x86_64-unknown-linux-gnu"
    } else {
        "x86_64-pc-windows-msvc"
    };
    let pkg = tmp.path().join("demo-1.0.0");
    write(
        &pkg.join("yaoxiang.toml"),
        &format!(
            "[package]\nname = \"demo\"\nversion = \"1.0.0\"\n\n[binaries]\n\"{other}\" = {{ url = \"https://example.com/a.tar.gz\", sha256 = \"{}\" }}\n",
            "0".repeat(64)
        ),
    );

    // Act
    let outcome = run(&pkg, None);

    // Assert
    assert_eq!(outcome.via, "strategy-none");
}
