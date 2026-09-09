//! `yx-toolchain.toml` 项目级 pin（先例 rust-toolchain.toml）
//!
//! ```toml
//! toolchain = "0.7.14"
//! ```
//!
//! 不进 `yaoxiang.toml`——包清单不该把工具链版本强加给库使用者（RFC-037）。

use std::path::Path;

/// 从 `start` 逐级向上查找 `yx-toolchain.toml`，返回 pin 的版本
pub fn find_pin_from(start: &Path) -> Option<String> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let candidate = d.join("yx-toolchain.toml");
        if candidate.is_file() {
            let content = std::fs::read_to_string(&candidate).ok()?;
            return parse(&content);
        }
        dir = d.parent();
    }
    None
}

/// pin 文件内容解析：只认 `toolchain = "<version>"`
fn parse(content: &str) -> Option<String> {
    let value = content.parse::<toml::Table>().ok()?;
    value
        .get("toolchain")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

/// 规整版本字符串：剥掉可选的 `v` 前缀（接受 `v0.7.14` 与 `0.7.14`）
pub fn normalize_version(input: &str) -> String {
    input
        .trim()
        .strip_prefix('v')
        .unwrap_or(input.trim())
        .to_string()
}

#[cfg(test)]
mod tests;
