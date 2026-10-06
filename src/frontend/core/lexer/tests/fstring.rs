//! F-string lexer 测试 — 基于语言规范 syntax.md §1.6.3 字符串字面量 + RFC-012
//!
//! raw 协议（#402）：花括号序列（含 `{{` / `}}` 转义）逐字保留，
//! 由 parser 切段时解转义；反斜杠转义在 lexer 解码。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::lexer::tokens::TokenKind;

fn assert_fstring_raw(
    source: &str,
    expected: &str,
) {
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("tokenize({source:?}) 失败: {e:?}"));
    match &tokens[0].kind {
        TokenKind::FStringLiteral(raw) => {
            assert_eq!(raw, expected, "f-string raw 内容不符: {source:?}")
        }
        other => panic!("期望 FStringLiteral，实际 {other:?}: {source:?}"),
    }
}

// 单行 f-string

#[test]
fn test_fstring_brace_escape_preserved_raw() {
    // {{ → 保留两个字符，parser 负责解成字面 {
    assert_fstring_raw(r#"f"hello{{""#, "hello{{");
}

#[test]
fn test_fstring_close_brace_escape_preserved_raw() {
    assert_fstring_raw(r#"f"a}}b""#, "a}}b");
}

#[test]
fn test_fstring_interpolation_markers_preserved() {
    assert_fstring_raw(r#"f"hello {name}""#, "hello {name}");
}

#[test]
fn test_fstring_mixed_escape_and_interpolation() {
    // `{{` 与插值相邻：f"{{{x}}}" 的 raw 必须原样保留五个花括号
    assert_fstring_raw(r#"f"{{{x}}}""#, "{{{x}}}");
}

#[test]
fn test_fstring_backslash_escape_decoded() {
    // 反斜杠转义在 lexer 解码（与花括号协议相反）
    assert_fstring_raw(r#"f"a\nb""#, "a\nb");
}

// 多行 f-string（f""")

#[test]
fn test_fstring_multiline_raw() {
    assert_fstring_raw("f\"\"\"line1\nline2\"\"\"", "line1\nline2");
}

#[test]
fn test_fstring_multiline_with_interpolation() {
    assert_fstring_raw("f\"\"\"a {x}\nb\"\"\"", "a {x}\nb");
}

#[test]
fn test_fstring_multiline_brace_escape_preserved() {
    assert_fstring_raw("f\"\"\"{{literal}}\"\"\"", "{{literal}}");
}

#[test]
fn test_fstring_multiline_empty() {
    assert_fstring_raw("f\"\"\"\"\"\"", "");
}

#[test]
fn test_fstring_multiline_single_quotes_inside() {
    // 内容中的单个 `"` 不终止（结尾引号须转义，与普通多行字符串一致）
    assert_fstring_raw(r#"f"""say \"hi\"""""#, "say \"hi\"");
}

#[test]
fn test_fstring_multiline_unterminated() {
    // Arrange：无闭合 """ 的多行 f-string
    let source = "f\"\"\"line1\nline2";

    // Act
    let result = tokenize(source);

    // Assert：未终结必须显式报错（UnterminatedString）
    assert!(result.is_err(), "未终结的 f\"\"\" 必须报错: {result:?}");
}
