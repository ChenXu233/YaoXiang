//! 测试依赖缓存和完整性校验（SHA-256）
//!
//! 覆盖:
//! - SHA-256 空输入哈希
//! - SHA-256 "hello" 哈希
//! - SHA-256 增量计算一致性
//! - 单文件校验和计算
//! - 目录校验和的确定性
//! - 目录修改后校验和变化
//! - `.git` 目录不影响校验和
//! - 校验和验证
//! - 不存在目录返回错误

use std::path::Path;

use crate::package::vendor::cache::{Sha256, compute_directory_checksum, compute_file_checksum, verify_checksum};

#[test]
fn test_sha256_empty() {
    let mut hasher = Sha256::new();
    hasher.update(b"");
    let hash = hasher.finalize_hex();
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_sha256_hello() {
    let mut hasher = Sha256::new();
    hasher.update(b"hello");
    let hash = hasher.finalize_hex();
    assert_eq!(
        hash,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn test_sha256_incremental() {
    let mut h1 = Sha256::new();
    h1.update(b"hello world");
    let hash1 = h1.finalize_hex();

    let mut h2 = Sha256::new();
    h2.update(b"hello ");
    h2.update(b"world");
    let hash2 = h2.finalize_hex();

    assert_eq!(hash1, hash2);
}

#[test]
fn test_compute_file_checksum() {
    let tmp = tempfile::TempDir::new().unwrap();
    let file_path = tmp.path().join("test.txt");
    std::fs::write(&file_path, "hello").unwrap();

    let checksum = compute_file_checksum(&file_path).unwrap();
    assert_eq!(
        checksum,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn test_compute_directory_checksum_deterministic() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().join("pkg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), "aaa").unwrap();
    std::fs::write(dir.join("b.txt"), "bbb").unwrap();

    let hash1 = compute_directory_checksum(&dir).unwrap();
    let hash2 = compute_directory_checksum(&dir).unwrap();
    assert_eq!(hash1, hash2);
}

#[test]
fn test_compute_directory_checksum_changes_on_modification() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().join("pkg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), "aaa").unwrap();

    let hash1 = compute_directory_checksum(&dir).unwrap();

    std::fs::write(dir.join("a.txt"), "modified").unwrap();
    let hash2 = compute_directory_checksum(&dir).unwrap();

    assert_ne!(hash1, hash2);
}

#[test]
fn test_compute_directory_checksum_ignores_git() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().join("pkg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), "aaa").unwrap();

    let hash1 = compute_directory_checksum(&dir).unwrap();

    // 创建 .git 目录不应影响哈希
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    std::fs::write(dir.join(".git").join("HEAD"), "ref").unwrap();

    let hash2 = compute_directory_checksum(&dir).unwrap();
    assert_eq!(hash1, hash2);
}

#[test]
fn test_verify_checksum() {
    let tmp = tempfile::TempDir::new().unwrap();
    let dir = tmp.path().join("pkg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lib.yx"), "main = { 42 }").unwrap();

    let checksum = compute_directory_checksum(&dir).unwrap();
    assert!(verify_checksum(&dir, &checksum).unwrap());

    // 篡改后校验失败
    std::fs::write(dir.join("lib.yx"), "main = { 0 }").unwrap();
    assert!(!verify_checksum(&dir, &checksum).unwrap());
}

#[test]
fn test_directory_checksum_not_found() {
    let result = compute_directory_checksum(Path::new("/nonexistent/path"));
    assert!(result.is_err());
}
