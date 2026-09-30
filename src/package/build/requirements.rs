//! 系统依赖预检（RFC-014b Phase 5b）
//!
//! 安装构建前核对 `[build.requirements]`：每个条目是 `工具 → 版本要求`
//! （如 `cargo = ">= 1.70"`）。核对方式：跑 `<tool> --version`，从输出抓
//! 第一个版本样式的记号，与要求比对。全部失败一次报齐（RFC 错误样式），
//! 附安装指引（决议 6：预检报错 + 安装指引，无额外机制）。

use std::collections::BTreeMap;
use std::process::Command;

use semver::VersionReq;

use crate::package::error::{PackageError, PackageResult};
use crate::package::source::resolver::parse_version;

/// 已知工具的安装指引（RFC 错误样式的 `Install:` 行）
fn install_hint(tool: &str) -> &'static str {
    match tool {
        "cargo" | "rustc" => "https://rustup.rs",
        "cmake" => "https://cmake.org/download",
        "ninja" => "https://ninja-build.org",
        "clang" | "gcc" => "请安装系统 C 工具链（apt/xcode-select/MSVC BuildTools）",
        _ => "请安装该工具并加入 PATH",
    }
}

/// 预检全部条目；失败一次报齐（每行一条，RFC 错误样式）
pub fn check(reqs: &BTreeMap<String, String>) -> PackageResult<()> {
    let mut failures: Vec<String> = Vec::new();
    for (tool, req) in reqs {
        if let Err(reason) = check_one(tool, req) {
            failures.push(reason);
        }
    }
    if failures.is_empty() {
        return Ok(());
    }
    let mut msg = String::from("构建依赖不满足（[build.requirements]）：");
    for f in &failures {
        msg.push_str("\n  - ");
        msg.push_str(f);
    }
    Err(PackageError::InvalidManifest(msg))
}

/// 单工具核对；Err 的文本面向用户（含安装指引）
fn check_one(
    tool: &str,
    req: &str,
) -> Result<(), String> {
    let output = Command::new(tool).arg("--version").output().map_err(|_| {
        format!(
            "{tool} 未安装或不在 PATH（要求 {req}）；Install: {}",
            install_hint(tool)
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "{tool} --version 执行失败（要求 {req}）；Install: {}",
            install_hint(tool)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let found = detect_version(&stdout)
        .ok_or_else(|| format!("无法从 `{tool} --version` 输出解析版本（要求 {req}）"))?;
    let req_parsed =
        parse_tool_req(req).map_err(|e| format!("{tool}: 非法版本要求 '{req}': {e}"))?;
    if req_parsed.matches(&found) {
        Ok(())
    } else {
        Err(format!(
            "{tool} 版本不满足：要求 {req}，实际 {}；Install: {}",
            found,
            install_hint(tool)
        ))
    }
}

/// 从 `--version` 输出抓第一个版本样式的记号（宽容两段/三段 + v 前缀）
fn detect_version(output: &str) -> Option<semver::Version> {
    for token in output.split_whitespace() {
        let candidate = token.trim_start_matches('v').trim_end_matches(',');
        // 版本记号以数字开头，且主体不含字母（排除 "cargo" 这类词）
        if candidate.chars().next().is_some_and(|c| c.is_ascii_digit())
            && candidate
                .chars()
                .take_while(|c| *c != '-' && *c != '+')
                .all(|c| c.is_ascii_digit() || c == '.')
            && candidate.contains('.')
        {
            if let Ok(v) = parse_version(candidate) {
                return Some(v);
            }
        }
    }
    None
}

/// 工具版本要求解析：比较器操作数宽容补齐（`>= 1.70` → `>=1.70.0`）
///
/// semver crate 的比较器操作数必须三段；RFC 示例（`>= 1.70`）按两段书写，
/// 这里逐操作数补齐后重组。
pub fn parse_tool_req(req: &str) -> PackageResult<VersionReq> {
    let mut comparators = Vec::new();
    for part in req.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (op_str, ver_str) = if let Some(rest) = part.strip_prefix(">=") {
            (">=", rest)
        } else if let Some(rest) = part.strip_prefix("<=") {
            ("<=", rest)
        } else if let Some(rest) = part.strip_prefix("==") {
            ("=", rest)
        } else if let Some(rest) = part.strip_prefix('=') {
            ("=", rest)
        } else if let Some(rest) = part.strip_prefix('>') {
            (">", rest)
        } else if let Some(rest) = part.strip_prefix('<') {
            ("<", rest)
        } else {
            ("", part)
        };
        let version = parse_version(ver_str.trim())?;
        let op = match op_str {
            ">=" => semver::Op::GreaterEq,
            "<=" => semver::Op::LessEq,
            "=" => semver::Op::Exact,
            ">" => semver::Op::Greater,
            "<" => semver::Op::Less,
            _ => {
                // 裸版本 = 精确要求（与 resolver::parse_version_req 口径一致）
                semver::Op::Exact
            }
        };
        comparators.push(semver::Comparator {
            op,
            major: version.major,
            minor: Some(version.minor),
            patch: Some(version.patch),
            pre: version.pre.clone(),
        });
    }
    if comparators.is_empty() {
        return Err(PackageError::InvalidManifest(format!("空版本要求 '{req}'")));
    }
    Ok(VersionReq { comparators })
}
