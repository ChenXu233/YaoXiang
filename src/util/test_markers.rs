//! 测试文件头部指令解析 — 双 runner 共用文法（RFC-036 §8.2）
//!
//! yx_runner（cargo test 语料 runner）与 yaoxiang test（CLI）共用本模块。
//! 头部指令是判定的唯一权威，全部位于文件前 16 行，`// key: value` 文法：
//!
//! - `// expect: compile-error E1002 [E1003 ...]` — 编译期拒绝测试：check 必须
//!   失败，且全部预期码出现在输出中（run 不执行）
//! - `// expect: runtime-error E6003 [...]` — 运行期失败测试：check 必须通过，
//!   run 必须失败，且全部预期码出现在输出中
//! - 无 expect 指令 = 行为测试（run 退出码 0 = PASS）
//! - `// skip: <原因>` — 跳过执行，计入 skipped
//! - `// mode: embedded|standard|full` — 子进程 `--runtime` 模式（仅 run 步消费）
//!
//! 文法为严格 token 匹配（2026-09-06 定案，弃用 `[test:error]` 布尔标记与
//! 中文 `预期:` 散文抠码）：expect/mode 指令解析失败记录进 [`TestFileSpec::invalid`]，
//! runner 据此直接判 FAIL——指令声明错误的静默退化通道不存在。
//!
//! 已知渲染债：parse 期诊断当前以 Debug 形态输出（`code: "E0012"` 而非
//! `[E0012]`），码扫描对两种形态都接受；诊断渲染统一后应收回严格形态。

use std::path::Path;

use crate::util::diagnostic::emitter::ansi::strip_ansi;

/// 指令扫描窗口（文件头部行数）
const HEADER_LINES: usize = 16;

/// 合法的 `// mode:` 值（与语料历史约定一致）
const MODES: [&str; 3] = ["embedded", "standard", "full"];

/// `// expect:` 声明的期望结果 — 判定路径的唯一权威
#[derive(Debug, Default, PartialEq)]
pub enum Expectation {
    /// 无 expect 指令：行为测试，run 退出码 0 = PASS
    #[default]
    Behavior,
    /// 编译期拒绝：check 必须失败，全部码须出现在输出中（run 不执行）
    CompileError(Vec<String>),
    /// 运行期失败：check 必须通过，run 必须失败，全部码须出现在输出中
    RuntimeError(Vec<String>),
}

/// 测试文件头部声明的指令全集
#[derive(Debug, Default)]
pub struct TestFileSpec {
    /// `// expect:` 期望结果
    pub expectation: Expectation,
    /// expect/mode 指令解析失败的原因（非 None = 测试直接 FAIL，构造期拒绝）
    pub invalid: Option<String>,
    /// `// skip: <原因>` — 跳过执行（不计入通过/失败）
    pub skip_reason: Option<String>,
    /// `// mode: <模式>` — 子进程运行时模式（仅 run 步消费）
    pub mode: Option<String>,
}

impl TestFileSpec {
    /// 解析测试文件头部指令（读不了文件返回默认值——发现不是错误边界）
    pub fn parse(path: &Path) -> Self {
        let Ok(content) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        Self::parse_str(&content)
    }

    fn parse_str(content: &str) -> Self {
        let mut spec = Self::default();
        let mut expect_seen = false;
        for line in content.lines().take(HEADER_LINES) {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("// expect:") {
                if expect_seen {
                    // 一文件一期望；重复声明几乎必然是复制粘贴错误
                    spec.invalid.get_or_insert_with(|| {
                        "duplicate // expect: directive (only one is allowed)".to_string()
                    });
                    continue;
                }
                expect_seen = true;
                match parse_expect(rest) {
                    Ok(expectation) => spec.expectation = expectation,
                    Err(reason) => spec.invalid = Some(reason),
                }
            } else if let Some(reason) = trimmed.strip_prefix("// skip:") {
                let reason = reason.trim();
                spec.skip_reason = Some(if reason.is_empty() {
                    "no reason given".to_string()
                } else {
                    reason.to_string()
                });
            } else if let Some(rest) = trimmed.strip_prefix("// mode:") {
                let mode = rest.trim();
                if MODES.contains(&mode) {
                    spec.mode = Some(mode.to_string());
                } else {
                    spec.invalid = Some(format!(
                        "invalid // mode: '{mode}' (expected one of {MODES:?})"
                    ));
                }
            }
        }
        spec
    }

    /// 期望码（Behavior 期望 → 空切片）
    pub fn expected_codes(&self) -> &[String] {
        match &self.expectation {
            Expectation::CompileError(codes) | Expectation::RuntimeError(codes) => codes,
            Expectation::Behavior => &[],
        }
    }

    /// 类别标签（人类报告与 JSON `files[].kind` 共用；invalid 由 runner 单独标注）
    pub fn kind_label(&self) -> &'static str {
        match self.expectation {
            Expectation::Behavior => "behavior",
            Expectation::CompileError(_) => "compile-error",
            Expectation::RuntimeError(_) => "runtime-error",
        }
    }

    /// RFC-036 §8.2 结构化预期码比对：每个预期码都须出现在输出中（`[EXXXX]`
    /// 或 parse 期 Debug 形态 `code: "EXXXX"`），任一缺失即 Err，并附实际出现的
    /// 码以便定位。
    pub fn check_expected_codes(
        &self,
        output: &str,
    ) -> Result<(), String> {
        let codes = self.expected_codes();
        if codes.is_empty() {
            return Ok(());
        }
        let emitted = emitted_error_codes(&strip_ansi(output));
        let missing: Vec<&String> = codes.iter().filter(|c| !emitted.contains(c)).collect();
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
            "[expect] 预期错误码 {missing} 未在输出中出现（实际输出含: {emitted_desc}）"
        ))
    }
}

/// 解析 `// expect:` 之后的 token 序列：`<kind> <EXXXX>+`，多余 token 一律拒绝。
fn parse_expect(rest: &str) -> Result<Expectation, String> {
    let mut tokens = rest.split_whitespace();
    let kind = tokens
        .next()
        .ok_or("missing expectation kind (expected compile-error|runtime-error)")?;
    let codes: Vec<String> = tokens.map(str::to_string).collect();
    if kind != "compile-error" && kind != "runtime-error" {
        return Err(format!(
            "unknown expectation kind '{kind}' (expected compile-error|runtime-error)"
        ));
    }
    if codes.is_empty() {
        return Err(format!("expect '{kind}' requires at least one EXXXX code"));
    }
    for code in &codes {
        if !is_error_code(code) {
            return Err(format!(
                "expect '{kind}' has invalid code token '{code}' (expected EXXXX)"
            ));
        }
    }
    Ok(if kind == "compile-error" {
        Expectation::CompileError(codes)
    } else {
        Expectation::RuntimeError(codes)
    })
}

/// `EXDDD` 形态判定（严格大小写）
fn is_error_code(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 5 && bytes[0] == b'E' && bytes[1..].iter().all(|b| b.is_ascii_digit())
}

/// 扫描输出中全部错误码（保序去重），接受两种形态：
/// - pretty 诊断的 `[EXXXX]`
/// - parse 期 Debug 形态的 `code: "EXXXX"`（渲染债，见模块文档）
fn emitted_error_codes(output: &str) -> Vec<String> {
    let bytes = output.as_bytes();
    let mut codes = Vec::new();
    let mut i = 0;
    while i + 5 <= bytes.len() {
        if bytes[i] == b'E' && bytes[i + 1..i + 5].iter().all(|b| b.is_ascii_digit()) {
            let bracketed =
                i > 0 && bytes[i - 1] == b'[' && i + 5 < bytes.len() && bytes[i + 5] == b']';
            let debug_field = i >= 7
                && &bytes[i - 7..i] == b"code: \""
                && i + 5 < bytes.len()
                && bytes[i + 5] == b'"';
            if bracketed || debug_field {
                let code = output[i..i + 5].to_string();
                if !codes.contains(&code) {
                    codes.push(code);
                }
                i += 5;
                continue;
            }
        }
        i += 1;
    }
    codes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(content: &str) -> TestFileSpec {
        TestFileSpec::parse_str(content)
    }

    // RFC-036 §8.2：指令文法解析

    #[test]
    fn behavior_when_no_directives() {
        let spec = parse("// 标题\n\nmain = {\n    x = 1\n}\n");
        assert_eq!(spec.expectation, Expectation::Behavior);
        assert!(spec.invalid.is_none());
        assert!(spec.skip_reason.is_none());
        assert!(spec.mode.is_none());
    }

    #[test]
    fn compile_error_with_codes() {
        let spec = parse("// expect: compile-error E1002 E1003\n");
        assert_eq!(
            spec.expectation,
            Expectation::CompileError(vec!["E1002".into(), "E1003".into()])
        );
        assert!(spec.invalid.is_none());
    }

    #[test]
    fn runtime_error_single_code() {
        let spec = parse("// expect: runtime-error E6003\n");
        assert_eq!(
            spec.expectation,
            Expectation::RuntimeError(vec!["E6003".into()])
        );
    }

    #[test]
    fn unknown_kind_is_invalid() {
        let spec = parse("// expect: ok\n");
        assert_eq!(spec.expectation, Expectation::Behavior);
        assert!(spec.invalid.unwrap().contains("unknown expectation kind"));
    }

    #[test]
    fn missing_code_is_invalid() {
        let spec = parse("// expect: compile-error\n");
        assert!(spec.invalid.unwrap().contains("requires at least one"));
    }

    #[test]
    fn trailing_prose_is_invalid() {
        // 严格文法：码后跟任何非 EXDDD token 都拒绝（构造期拒绝散文回潮）
        let spec = parse("// expect: compile-error E1002（注解）\n");
        assert!(spec.invalid.unwrap().contains("invalid code token"));
    }

    #[test]
    fn duplicate_expect_is_invalid() {
        let spec = parse("// expect: compile-error E1002\n// expect: compile-error E1003\n");
        assert!(spec.invalid.unwrap().contains("duplicate"));
    }

    #[test]
    fn lowercase_code_is_invalid() {
        let spec = parse("// expect: compile-error e1002\n");
        assert!(spec.invalid.unwrap().contains("invalid code token"));
    }

    #[test]
    fn skip_directive() {
        let spec = parse("// skip: 等待错误恢复落地\n");
        assert_eq!(spec.skip_reason.as_deref(), Some("等待错误恢复落地"));
    }

    #[test]
    fn mode_directive_valid_and_invalid() {
        let spec = parse("// mode: standard\n");
        assert_eq!(spec.mode.as_deref(), Some("standard"));
        let bad = parse("// mode: turbo\n");
        assert!(bad.invalid.unwrap().contains("invalid // mode:"));
    }

    #[test]
    fn chinese_prose_is_immune() {
        // 旧文法的散文 `预期:` 行不再被识别（行为测试不受污染）
        let spec = parse("// 预期: 编译并运行通过，g(99) 返回 42\n");
        assert_eq!(spec.expectation, Expectation::Behavior);
        assert!(spec.invalid.is_none());
    }

    #[test]
    fn directives_outside_header_window_ignored() {
        let mut content = String::from("// 标题\n");
        for i in 0..20 {
            content.push_str(&format!("// filler {i}\n"));
        }
        content.push_str("// expect: compile-error E1002\n");
        let spec = parse(&content);
        assert_eq!(spec.expectation, Expectation::Behavior);
    }

    // RFC-036 §8.2：输出码扫描（pretty 与 parse-Debug 双形态）

    #[test]
    fn scan_pretty_and_debug_forms() {
        // 括号形态与 Debug 形态都抓取；非 EXDDD 形态（如 3 位数）不抓
        let codes = emitted_error_codes("error [E1002] bad\nParse error: Diagnostic { code: \"E0012\", message: \"see [E999] note\" }\n");
        assert_eq!(codes, vec!["E1002", "E0012"]);
    }

    #[test]
    fn scan_ignores_bare_mentions() {
        let codes = emitted_error_codes("see E1002 or (E6003) for details");
        assert!(codes.is_empty());
    }

    #[test]
    fn code_comparison_reports_missing() {
        let spec = parse("// expect: runtime-error E6003\n");
        let err = spec
            .check_expected_codes("error [E6001] divides by zero")
            .unwrap_err();
        assert!(err.contains("E6003"));
        assert!(err.contains("E6001"));
        assert!(spec.check_expected_codes("error [E6003] oob").is_ok());
    }
}
