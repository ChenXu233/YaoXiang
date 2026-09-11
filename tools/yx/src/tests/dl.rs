//! 解包与校验辅助测试（RFC-037：版本目录 = 发行包解压根目录）

use std::fs;
use sha2::Digest;
use tempfile::TempDir;

use crate::dl::{entry_path_is_safe, strip_first, verify_sha256};

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
    let digest = format!("{:x}", hasher.finalize());

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

#[test]
fn test_entry_path_is_safe_rejects_parent_traversal() {
    // Act & Assert: 带 .. 的条目不安全（纵深防御，zip 侧由 enclosed_name 保护）
    assert!(
        !entry_path_is_safe(std::path::Path::new("pkg/../../evil.yx")),
        "parent traversal entry must be flagged unsafe"
    );
    assert!(
        entry_path_is_safe(std::path::Path::new("pkg/bin/yaoxiang-rs")),
        "normal entry must be considered safe"
    );
}

/// 把未压缩 tar 字节流 gzip 后写到 `archive`
fn write_gz_tar(
    archive: &std::path::Path,
    tar_bytes: Vec<u8>,
) {
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    std::io::Write::write_all(&mut gz, &tar_bytes).unwrap();
    fs::write(archive, gz.finish().unwrap()).unwrap();
}

#[test]
fn test_unpack_rejects_tar_symlink_entry() {
    // Arrange: 构造含符号链接条目的归档（链接目标是写穿解包目录的向量；
    // 发行包由 package-dist.sh 产出，永远不含链接条目）
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("a.tar.gz");
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(0);
    header.set_mode(0o777);
    header.set_entry_type(tar::EntryType::Symlink);
    header
        .set_link_name(std::path::Path::new("../../outside"))
        .unwrap();
    header.set_cksum();
    builder
        .append_data(&mut header, "pkg/evil", std::io::empty())
        .unwrap();
    write_gz_tar(&archive, builder.into_inner().unwrap());

    // Act
    let result = crate::dl::unpack(&archive, "linux", &dir.path().join("out"));

    // Assert
    assert!(
        matches!(result, Err(crate::error::Error::Message(ref m)) if m.contains("symlink")),
        "symlink entry must be rejected, got {result:?}"
    );
}

#[test]
fn test_unpack_rejects_tar_hardlink_entry() {
    // Arrange: 构造含硬链接条目的归档（hard link 可把包外既有文件链入解包树）
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("a.tar.gz");
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(0);
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::Link);
    header
        .set_link_name(std::path::Path::new("pkg/real.bin"))
        .unwrap();
    header.set_cksum();
    builder
        .append_data(&mut header, "pkg/alias", std::io::empty())
        .unwrap();
    write_gz_tar(&archive, builder.into_inner().unwrap());

    // Act
    let result = crate::dl::unpack(&archive, "linux", &dir.path().join("out"));

    // Assert
    assert!(
        matches!(result, Err(crate::error::Error::Message(ref m)) if m.contains("hardlink")),
        "hardlink entry must be rejected, got {result:?}"
    );
}

#[test]
fn test_unpack_tar_regular_files_still_extract() {
    // Arrange: 正常归档（目录 + 普通文件），链接拒绝不得误伤常规解包路径
    let dir = TempDir::new().unwrap();
    let archive = dir.path().join("a.tar.gz");
    let mut builder = tar::Builder::new(Vec::new());
    let mut dir_header = tar::Header::new_gnu();
    dir_header.set_size(0);
    dir_header.set_mode(0o755);
    dir_header.set_entry_type(tar::EntryType::Directory);
    dir_header.set_cksum();
    builder
        .append_data(&mut dir_header, "pkg/bin", std::io::empty())
        .unwrap();
    let mut file_header = tar::Header::new_gnu();
    file_header.set_size(5);
    file_header.set_mode(0o644);
    file_header.set_entry_type(tar::EntryType::Regular);
    file_header.set_cksum();
    builder
        .append_data(&mut file_header, "pkg/bin/yx", b"hello".as_slice())
        .unwrap();
    write_gz_tar(&archive, builder.into_inner().unwrap());
    let out = dir.path().join("out");

    // Act
    crate::dl::unpack(&archive, "linux", &out).unwrap();

    // Assert
    assert_eq!(
        fs::read(out.join("bin/yx")).unwrap(),
        b"hello",
        "regular file must extract with content intact under stripped path"
    );
}
