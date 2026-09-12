//! 解包与校验辅助测试（RFC-037：版本目录 = 发行包解压根目录）

use std::fs;
use sha2::Digest;
use tempfile::TempDir;

use crate::dl::{strip_first, verify_sha256};

#[test]
fn test_strip_first_removes_top_level_package_dir() {
    // Arrange: 发行包内路径 yaoxiang-0.7.14-x86_64-unknown-linux-gnu/bin/yaoxiang-rs
    let entry = std::path::Path::new("yaoxiang-0.7.14-x86_64-unknown-linux-gnu/bin/yaoxiang-rs");
    let dest = TempDir::new().unwrap();

    // Act
    let stripped = strip_first(entry, dest.path()).unwrap();

    // Assert
    assert_eq!(
        stripped,
        dest.path().join("bin/yaoxiang-rs"),
        "first component must be stripped so versions/<ver>/ is the extraction root"
    );
}

#[test]
fn test_strip_first_rejects_toplevel_loose_entry() {
    // Arrange: 没有顶层目录的散文件
    let entry = std::path::Path::new("README.md");
    let dest = TempDir::new().unwrap();

    // Act
    let stripped = strip_first(entry, dest.path());

    // Assert: 安全起见拒绝，不写出归档根之外的路径
    assert!(
        stripped.is_none(),
        "entry without top-level dir must be rejected"
    );
}

#[test]
fn test_verify_sha256_accepts_matching_digest_file_content() {
    // Arrange: 内容 "hex  filename" 格式（sha256sum 输出）
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("a.tar.gz");
    fs::write(&file, b"payload").unwrap();
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"payload");
    let digest = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    // Act
    let result = verify_sha256(&file, &format!("{digest}  a.tar.gz"));

    // Assert
    assert!(result.is_ok(), "matching digest must pass, got {result:?}");
}

#[test]
fn test_verify_sha256_rejects_tampered_content() {
    // Arrange
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("a.tar.gz");
    fs::write(&file, b"payload").unwrap();

    // Act
    let result = verify_sha256(&file, "deadbeef  a.tar.gz");

    // Assert
    assert!(
        matches!(result, Err(crate::error::Error::ChecksumMismatch { .. })),
        "tampered content must fail checksum, got {result:?}"
    );
}
