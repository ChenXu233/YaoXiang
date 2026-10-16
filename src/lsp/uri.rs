//! LSP 协议边缘的 URI 换算与语义数据库规范化键（WBS 4.6.1）。
//!
//! 编译器层的语义数据以 **OS 路径**为文件身份；LSP 协议面以 URI 为文件
//! 身份。World 会话库统一以**规范化 file URI**为键——即
//! `path_to_uri(uri_to_path(u))` 的幂等结果：同一文件无论客户端送来
//! 什么等价形式（盘符大小写、%XX 编码差异），也无论数据来自哪次编译，
//! 键都一致，跨文件 `resolves_to` 的 DefId 因此稳定命中。
//!
//! 非 file scheme（`std://`、`builtin://` 等虚拟键）原样保留。

use std::path::{Path, PathBuf};
use std::str::FromStr;

use lsp_types::Uri;

/// 本地 file URI → 磁盘路径（`file:///E:/a/b.yx` → `E:\a\b.yx`）。
///
/// ponytail: 仅处理 file scheme + %XX 解码；其余返回 None。
/// （实现自 handlers/diagnostics.rs 迁入，行为不变）
pub(crate) fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let parsed = Uri::from_str(uri).ok()?;
    if parsed.scheme().map(|s| s.as_str()) != Some("file") {
        return None;
    }
    let decoded = percent_decode(parsed.path().as_str());
    let trimmed = if cfg!(windows) {
        decoded.trim_start_matches('/').to_string()
    } else {
        decoded
    };
    Some(PathBuf::from(trimmed))
}

/// 磁盘路径 → file URI（`E:\a\b.yx` → `file:///E:/a/b.yx`）。
///
/// 分隔符统一为正斜杠；空格、`%`、`#`、`?` 与非 ASCII 字符按 UTF-8
/// %XX 编码——产出是合法 URI（可直接回传给客户端），且与 `uri_to_path`
/// 互逆（规范化键的幂等性来源）。
pub(crate) fn path_to_uri(path: &Path) -> String {
    let mut out = String::from("file:///");
    let display = path.display().to_string().replace('\\', "/");
    for ch in display.chars() {
        if ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '/' | ':' | '-' | '_' | '.' | '~' | '(' | ')' | '@' | '+' | ','
            )
        {
            out.push(ch);
        } else {
            let mut buf = [0u8; 4];
            for b in ch.encode_utf8(&mut buf).as_bytes() {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

/// 语义数据库的规范化键：file URI 归一到 `path_to_uri(uri_to_path(u))`
/// 的幂等形式；非 file scheme（虚拟键）与无法解析的输入原样返回。
pub(crate) fn db_key(uri: &str) -> String {
    match uri_to_path(uri) {
        Some(p) => path_to_uri(&p),
        None => uri.to_string(),
    }
}

/// %XX 解码（UTF-8 安全）。
fn percent_decode(s: &str) -> String {
    fn hex(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if b'%' == bytes[i] && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(high << 4 | low);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
