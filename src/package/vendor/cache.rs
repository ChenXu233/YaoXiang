//! 依赖缓存和完整性校验
//!
//! 提供 SHA-256 校验和计算，确保依赖完整性。

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

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

    Ok(hasher.finalize_hex())
}

/// 计算目录的 SHA-256 校验和
///
/// 递归遍历所有文件，按排序后的路径计算组合哈希。
/// 忽略 `.git` 目录。
pub fn compute_directory_checksum(dir: &Path) -> PackageResult<String> {
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
    collect_files(dir, dir, &mut files)?;

    // 计算组合哈希
    let mut hasher = Sha256::new();
    for (rel_path, content) in &files {
        hasher.update(rel_path.as_bytes());
        hasher.update(b"\0");
        hasher.update(content);
        hasher.update(b"\0");
    }

    Ok(hasher.finalize_hex())
}

/// 递归收集目录中的所有文件
fn collect_files(
    base: &Path,
    dir: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
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
            collect_files(base, &path, files)?;
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

// ============================================================
// 内联 SHA-256 实现（避免外部依赖）
// ============================================================

/// SHA-256 哈希计算器
struct Sha256 {
    state: [u32; 8],
    buffer: Vec<u8>,
    total_len: u64,
}

impl Sha256 {
    /// SHA-256 初始哈希值
    const H: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    /// SHA-256 轮常量
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    fn new() -> Self {
        Sha256 {
            state: Self::H,
            buffer: Vec::new(),
            total_len: 0,
        }
    }

    fn update(
        &mut self,
        data: &[u8],
    ) {
        self.total_len += data.len() as u64;
        self.buffer.extend_from_slice(data);

        while self.buffer.len() >= 64 {
            let block: Vec<u8> = self.buffer.drain(..64).collect();
            self.process_block(&block);
        }
    }

    fn process_block(
        &mut self,
        block: &[u8],
    ) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }

        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        for (i, &wi) in w.iter().enumerate() {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(Self::K[i])
                .wrapping_add(wi);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }

    fn finalize_hex(mut self) -> String {
        // Padding
        let bit_len = self.total_len * 8;
        self.buffer.push(0x80);
        while (self.buffer.len() % 64) != 56 {
            self.buffer.push(0);
        }
        self.buffer.extend_from_slice(&bit_len.to_be_bytes());

        // Process remaining blocks
        let remaining = self.buffer.clone();
        for chunk in remaining.chunks(64) {
            self.process_block(chunk);
        }

        // Output
        self.state
            .iter()
            .map(|v| format!("{:08x}", v))
            .collect::<String>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
        let tmp = TempDir::new().unwrap();
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
        let tmp = TempDir::new().unwrap();
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
        let tmp = TempDir::new().unwrap();
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
        let tmp = TempDir::new().unwrap();
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
        let tmp = TempDir::new().unwrap();
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
}
