//! 语法面收口（SPEC syntax.md §2.8 / §2.10 / §3.1 / §3.2.1）——过渡 Pratt 解析器面。
//!
//! 守的性质（规范形，非实现选择）：
//! - 分支箭头 `->`；`=>` 只用于 lambda（§2.8 / §2.10）；
//! - 解构赋值只接受括号式 `(a, b) = e`（§3.2.1）；
//! - 相邻语句必须以换行或 `;` 分隔（§3.1）；
//! - lambda 保留 `=>`，括号式与裸单参式并存（§2.10）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::{parse_with_probe, NullProbe};

/// 解析夹具；返回是否零诊断（断言留在各用例体内，便于失败定位）。
fn parses_cleanly(source: &str) -> bool {
    let tokens = tokenize(source).expect("tokenize failed");
    let parsed = parse_with_probe(&tokens, &NullProbe);
    if parsed.has_errors {
        panic!("该源码应解析成功：{source:?}，实际诊断 {:?}", parsed.errors);
    }
    true
}

/// 解析应失败的夹具；返回诊断列表。
fn parse_errors(source: &str) -> Vec<crate::util::diagnostic::Diagnostic> {
    let tokens = tokenize(source).expect("tokenize failed");
    let parsed = parse_with_probe(&tokens, &NullProbe);
    assert!(
        parsed.has_errors,
        "该源码应被拒绝：{source:?}（未产生任何诊断）"
    );
    parsed.errors
}

#[test]
fn test_match_arm_dash_arrow_parses() {
    // Arrange / Act：分支箭头 `->`（§2.8 规范形）。
    let source = "m = match 1 { 0 -> 0, _ -> 1 }\n";

    // Assert：无诊断。
    assert!(parses_cleanly(source), "该源码应零诊断");
}

#[test]
fn test_match_arm_fat_arrow_reports_e0011_with_dash_suggestion() {
    // Arrange：旧面分支箭头 `=>`——应报 E0011，且消息给出 `->` 建议。
    let source = "m = match 1 { 0 => 0, _ -> 1 }\n";

    // Act：解析并取首条诊断。
    let errors = parse_errors(source);

    // Assert：错误码 + 可操作建议（消息含 `->`）。
    let first = errors.first().expect("至少一条诊断");
    assert_eq!(first.code, "E0011", "分支 `=>` 必须报 E0011：{errors:?}");
    assert!(
        first.message.contains("->"),
        "诊断消息必须给出 `->` 建议，实际：{:?}",
        first.message
    );
}

#[test]
fn test_destructure_paren_form_parses() {
    // Arrange / Act：括号式解构（§3.2.1 唯一合法形态）。
    let source = "(a, b) = (1, 2)\n";

    // Assert：无诊断。
    assert!(parses_cleanly(source), "该源码应零诊断");
}

#[test]
fn test_bare_destructure_reports_syntax_error() {
    // Arrange：裸形态 `a, b = e` 已取消（§3.2.1）。
    let source = "a, b = (1, 2)\n";

    // Act：解析并取诊断。
    let errors = parse_errors(source);

    // Assert：语法类诊断（E0012 = 无效语法），且消息给出括号码。
    let first = errors.first().expect("至少一条诊断");
    assert_eq!(first.code, "E0012", "裸解构必须报语法错误：{errors:?}");
    assert!(
        first.message.contains("(a, b) = e"),
        "诊断消息必须给出括号式写法，实际：{:?}",
        first.message
    );
}

#[test]
fn test_adjacent_statements_without_separator_reports_error() {
    // Arrange：同行相邻语句缺分隔（§3.1 要求换行或 `;`）。
    let source = "x = 1 y = 2\n";

    // Act：解析并取诊断。
    let errors = parse_errors(source);

    // Assert：语法类诊断，且指向语句分隔规则。
    let first = errors.first().expect("至少一条诊断");
    assert_eq!(first.code, "E0012", "缺语句分隔必须报语法错误：{errors:?}");
    assert!(
        first.message.contains("分隔"),
        "诊断消息必须点明语句分隔，实际：{:?}",
        first.message
    );
}

#[test]
fn test_adjacent_statements_with_semicolon_parse() {
    // Arrange / Act：同行用 `;` 分隔（§3.1 合法形态）。
    let source = "x = 1; y = 2\n";

    // Assert：无诊断。
    assert!(parses_cleanly(source), "该源码应零诊断");
}

#[test]
fn test_lambda_bare_and_paren_forms_parse() {
    // Arrange / Act：lambda 保留 `=>`，裸单参与括号式并存（§2.10）。
    let source = "f = x => x + 1\ng = (y) => y + 2\n";

    // Assert：无诊断。
    assert!(parses_cleanly(source), "该源码应零诊断");
}
