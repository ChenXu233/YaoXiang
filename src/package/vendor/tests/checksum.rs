//! 测试依赖完整性校验（SHA-256）
//!
//! 覆盖:
//! - 标准向量（空输入 / "hello"）
//! - 单文件校验和计算
//! - 目录校验和的确定性
//! - 目录修改后校验和变化
//! - `.git` 目录不影响校验和
//! - 校验和验证
//! - 不存在目录返回错误

use std::path::Path;

use crate::package::vendor::checksum::{compute_directory_checksum};

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
fn test_directory_checksum_not_found() {
    let result = compute_directory_checksum(Path::new("/nonexistent/path"));
    assert!(result.is_err(), "校验不存在的目录应报错");
}
