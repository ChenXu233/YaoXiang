//! 内容指纹单源设施（RFC-027 §14.4 `signature_hash` / B.4 H6）。
//!
//! 本模块是 SHA-256 摘要与十六进制渲染的**唯一实现**：payload 的
//! `signature_hash`、逐库来源指纹、以及包校验的 `compute_bytes_checksum`
//! 都经这里；不得另写第二份 hex/摘要（禁令三 A）。

use std::fmt;

use sha2::{Digest, Sha256};

/// 32 字节内容指纹（SHA-256），Display = 64 位小写十六进制。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// 对字节序列取 SHA-256。
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        Self(out)
    }

    /// 对 UTF-8 文本取 SHA-256（规范文本指纹的入口）。
    pub fn of_text(text: &str) -> Self {
        Self::of_bytes(text.as_bytes())
    }

    /// 64 位小写十六进制。
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            use std::fmt::Write as _;
            let _ = write!(out, "{:02x}", byte);
        }
        out
    }
}

impl fmt::Display for ContentHash {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::ContentHash;

    #[test]
    fn test_content_hash_display_is_64_lowercase_hex() {
        // Arrange + Act
        let hash = ContentHash::of_text("abc");
        let hex = hash.to_hex();

        // Assert
        assert_eq!(hex.len(), 64, "SHA-256 十六进制必须是 64 位；实际: {hex}");
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "只允许小写十六进制；实际: {hex}"
        );
        assert_eq!(hash.to_string(), hex, "Display 必须与 to_hex 同形");
    }

    #[test]
    fn test_content_hash_of_text_matches_known_sha256() {
        // Arrange + Act
        let hash = ContentHash::of_text("abc");

        // Assert
        assert_eq!(
            hash.to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "ContentHash 必须是标准 SHA-256"
        );
    }
}
