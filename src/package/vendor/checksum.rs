//! 依赖完整性校验（SHA-256，RFC-014）
//!
//! RFC-014 依赖表指定 `sha2` crate；此前为内联手写实现，已按决议替换。

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::package::error::{PackageError, PackageResult};

/// 计算单个文件的 SHA-256 校验和
pub fn compute_file_checksum(path: &Path) -> PackageResult<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex(&hasher.finalize()))
}

/// 计算内存字节的 SHA-256 校验和（打包的 Inline 条目，如 publish 的 manifest 替换）
pub fn compute_bytes_checksum(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex(&hasher.finalize())
}

/// 计算目录的 SHA-256 校验和
///
/// 递归遍历所有文件，按排序后的路径计算组合哈希。
/// 忽略 `.git` 目录。
pub fn compute_directory_checksum(dir: &Path) -> PackageResult<String> {
    compute_directory_checksum_excluding(dir, &[])
}

/// 同 [`compute_directory_checksum`]，但按目录名排除子树（任意深度）
///
/// vendor 完整性语义 = 源码树完整性：`build/` 是派生产物（本地构建输出或
/// 已独立做整包 SHA-256 的预编译解包），不入校验和——否则 cargo 的
/// 非确定性产物会让每次安装都误判"被改动"而重装重建。
pub fn compute_directory_checksum_excluding(
    dir: &Path,
    skip_dirs: &[&str],
) -> PackageResult<String> {
    if !dir.exists() {
        return Err(PackageError::DependencyNotFound(format!(
            "目录不存在: {}",
            dir.display()
        )));
    }

    if dir.is_file() {
        return compute_file_checksum(dir);
    }

    // 收集所有文件路径（排序以确保确定性）
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    collect_files_excluding(dir, dir, &mut files, skip_dirs)?;

    // 计算组合哈希
    let mut hasher = Sha256::new();
    for (rel_path, content) in &files {
        hasher.update(rel_path.as_bytes());
        hasher.update(b"\0");
        hasher.update(content);
        hasher.update(b"\0");
    }

    Ok(hex(&hasher.finalize()))
}

fn collect_files_excluding(
    base: &Path,
    dir: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
    skip_dirs: &[&str],
) -> PackageResult<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        // 跳过 .git 目录
        if file_name == ".git" {
            continue;
        }

        if path.is_dir() {
            if skip_dirs.contains(&file_name.as_str()) {
                continue;
            }
            collect_files_excluding(base, &path, files, skip_dirs)?;
        } else {
            let rel_path = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let content = std::fs::read(&path)?;
            files.insert(rel_path, content);
        }
    }

    Ok(())
}

/// 验证目录的校验和是否匹配
pub fn verify_checksum(
    dir: &Path,
    expected: &str,
) -> PackageResult<bool> {
    let actual = compute_directory_checksum(dir)?;
    Ok(actual == expected)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
