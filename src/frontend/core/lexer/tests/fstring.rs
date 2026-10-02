//! RFC-012: F-string lexer 测试
//!
//! raw 协议（#402）：花括号序列（含 `{{` / `}}` 转义）逐字保留，
//! 由 parser 切段时解转义；反斜杠转义在 lexer 解码。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::lexer::tokens::TokenKind;

// 单行 f-string

#[test]
fn test_fstring_brace_escape_preserved_raw() {
    // {{ → 保留两个字符，parser 负责解成字面 {
    let tokens = tokenize(r#"f"hello{{""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "hello{{"));
}

#[test]
fn test_fstring_close_brace_escape_preserved_raw() {
    let tokens = tokenize(r#"f"a}}b""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "a}}b"));
}

#[test]
fn test_fstring_interpolation_markers_preserved() {
    let tokens = tokenize(r#"f"hello {name}""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "hello {name}"));
}

#[test]
fn test_fstring_mixed_escape_and_interpolation() {
    // `{{` 与插值相邻：f"{{{x}}}" 的 raw 必须原样保留五个花括号
    let tokens = tokenize(r#"f"{{{x}}}""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "{{{x}}}"));
}

#[test]
fn test_fstring_backslash_escape_decoded() {
    // 反斜杠转义在 lexer 解码（与花括号协议相反）
    let tokens = tokenize(r#"f"a\nb""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "a\nb"));
}

// 多行 f-string（f""")

#[test]
fn test_fstring_multiline_raw() {
    let tokens = tokenize("f\"\"\"line1\nline2\"\"\"").unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "line1\nline2"));
}

#[test]
fn test_fstring_multiline_with_interpolation() {
    let tokens = tokenize("f\"\"\"a {x}\nb\"\"\"").unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "a {x}\nb"));
}

#[test]
fn test_fstring_multiline_brace_escape_preserved() {
    let tokens = tokenize("f\"\"\"{{literal}}\"\"\"").unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "{{literal}}"));
}

#[test]
fn test_fstring_multiline_empty() {
    let tokens = tokenize("f\"\"\"\"\"\"").unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s.is_empty()));
}

#[test]
fn test_fstring_multiline_single_quotes_inside() {
    // 内容中的单个 `"` 不终止（结尾引号须转义，与普通多行字符串一致）
    let tokens = tokenize(r#"f"""say \"hi\"""""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "say \"hi\""));
}

#[test]
fn test_fstring_multiline_unterminated() {
    let result = tokenize("f\"\"\"line1\nline2");
    assert!(result.is_err());
}
