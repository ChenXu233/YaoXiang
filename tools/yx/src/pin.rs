//! `yx-toolchain.toml` 项目级 pin（先例 rust-toolchain.toml）
//!
//! ```toml
//! toolchain = "0.7.14"
//! ```
//!
//! 不进 `yaoxiang.toml`——包清单不该把工具链版本强加给库使用者（RFC-037）。

use std::path::Path;

/// 从 `start` 逐级向上查找 `yx-toolchain.toml`，返回 pin 的版本。
///
/// 文件存在但读不了/格式坏时**不静默跳过**：stderr 告警并按无 pin 处理
/// （显式失败优于静默回退——坏 pin 被忽略意味着派发版本与项目预期不符）。
pub fn find_pin_from(start: &Path) -> Option<String> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let candidate = d.join("yx-toolchain.toml");
        if candidate.is_file() {
            return match std::fs::read_to_string(&candidate) {
                Ok(content) => match parse(&content) {
                    Some(version) => Some(version),
                    None => {
                        eprintln!(
                            "yx: warning: {} 缺少有效的 toolchain = \"<version>\" 条目，pin 被忽略",
                            candidate.display()
                        );
                        None
                    }
                },
                Err(e) => {
                    eprintln!(
                        "yx: warning: 无法读取 {}: {e}，pin 被忽略",
                        candidate.display()
                    );
                    None
                }
            };
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
