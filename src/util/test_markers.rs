//! 测试文件头部标记解析 — 双 runner 共用约定（RFC-036 §8.2）
//!
//! yx_runner（cargo test 语料 runner）与 yaoxiang test（CLI）共用本模块：
//! 标记是判定的唯一权威，06-compile-errors 的目录约定已废弃（2026-09-03 收口，#319）。
//! 全部标记均须出现在文件头部前 16 行内。

use std::path::Path;

use crate::util::diagnostic::emitter::ansi::strip_ansi;

/// 标记扫描窗口（文件头部行数）
const HEADER_LINES: usize = 16;

/// 合法的 `[test:runtime]` 模式（与语料历史约定一致）
const RUNTIME_MODES: [&str; 3] = ["embedded", "standard", "full"];

/// 测试文件头部声明的标记全集
#[derive(Debug, Default, PartialEq)]
pub struct TestFileSpec {
    /// `// [test:ignore]: <原因>` — 跳过执行（不计入通过/失败）
    pub ignore_reason: Option<String>,
    /// `// [test:runtime]: <模式>` — 子进程运行时模式（非法值忽略）
    pub runtime_mode: Option<String>,
    /// `// [test:error]: <原因>` — 反向判定：退出码非 0 = PASS
    pub expect_error: bool,
    /// `// 预期: 编译错误 EXXXX` / `// 预期: 运行时错误 EXXXX` — 结构化预期码
    pub expected_codes: Vec<String>,
}

impl TestFileSpec {
    /// 解析测试文件头部标记（读不了文件返回默认值——发现不是错误边界）
    pub fn parse(path: &Path) -> Self {
        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let mut spec = Self::default();
        for line in content.lines().take(HEADER_LINES) {
            let trimmed = line.trim();
            if trimmed.contains("[test:error]") {
                spec.expect_error = true;
            }
            if let Some(reason) = trimmed.strip_prefix("// [test:ignore]:") {
                let reason = reason.trim();
                spec.ignore_reason = Some(if reason.is_empty() {
                    "no reason given".to_string()
                } else {
                    reason.to_string()
                });
            }
            if let Some(mode) = trimmed.strip_prefix("// [test:runtime]:") {
                let mode = mode.trim();
                if RUNTIME_MODES.contains(&mode) {
                    spec.runtime_mode = Some(mode.to_string());
                }
            }
            if let Some(code) = extract_expected_code(line) {
                if !spec.expected_codes.contains(&code) {
                    spec.expected_codes.push(code);
                }
            }
        }
        spec
    }

    /// RFC-036 §8.2 结构化预期码比对：每个预期码都须以 `[EXXXX]` 形态出现在
    /// 编译器/运行时输出中，任一缺失即 Err，并附实际出现的码以便定位。
    pub fn check_expected_codes(
        &self,
        output: &str,
    ) -> Result<(), String> {
        let emitted = emitted_error_codes(&strip_ansi(output));
        let missing: Vec<&String> = self
            .expected_codes
            .iter()
            .filter(|c| !emitted.contains(c))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let missing = missing
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let emitted_desc = if emitted.is_empty() {
            "无".to_string()
        } else {
            emitted.join(", ")
        };
        Err(format!(
            "[test:error] 预期错误码 {missing} 未在输出中出现（实际输出含: {emitted_desc}）"
        ))
    }
}

/// 提取一行中 `预期:` 之后的首个 `EXXXX` 码（无码形态返回 None）。
fn extract_expected_code(line: &str) -> Option<String> {
    let rest = line.split_once("预期:")?.1;
    let bytes = rest.as_bytes();
    for i in 0..bytes.len().saturating_sub(4) {
        if bytes[i] == b'E' && bytes[i + 1..i + 5].iter().all(|b| b.is_ascii_digit()) {
            return Some(rest[i..i + 5].to_string());
        }
    }
    None
}

/// 扫描输出中全部 `[EXXXX]` 形态的错误码（保序去重）。
fn emitted_error_codes(output: &str) -> Vec<String> {
    let bytes = output.as_bytes();
    let mut codes = Vec::new();
    let mut i = 0;
    while i + 7 <= bytes.len() {
        if bytes[i] == b'['
            && bytes[i + 1] == b'E'
            && bytes[i + 2..i + 6].iter().all(|b| b.is_ascii_digit())
            && bytes[i + 6] == b']'
        {
            let code = output[i + 1..i + 6].to_string();
            if !codes.contains(&code) {
                codes.push(code);
            }
            i += 7;
        } else {
            i += 1;
        }
    }
    codes
}
