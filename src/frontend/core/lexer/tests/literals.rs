//! Lexer 字面量测试 — 基于语言规范 §2.6
//!
//! §2.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §2.6.2: 浮点数 (带小数点和指数)
//! §2.6.3: 字符串 (转义序列 \\nrt'"\\, \\x, \\u{})
//! §2.6.4: 集合字面量 (由 parser 处理)
//! RFC-012: F-String 插值

use crate::frontend::core::lexer::{tokenize, TokenKind};

// ============================================================================
// §2.6.1: 整数字面量
// ============================================================================

#[test]
fn test_int_decimal() {
    let tokens = tokenize("42").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));
}

#[test]
fn test_int_decimal_underscore() {
    // 数字分隔符: 1_000_000
    let tokens = tokenize("1_000_000").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(1000000)));
}

#[test]
fn test_int_hex() {
    // 0x 开头
    let tokens = tokenize("0xFF").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(255)));
}

#[test]
fn test_int_hex_underscore() {
    let tokens = tokenize("0xAB_CD").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(43981)));
}

#[test]
fn test_int_octal() {
    // 0o 开头
    let tokens = tokenize("0o77").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(63)));
}

#[test]
fn test_int_binary() {
    // 0b 开头
    let tokens = tokenize("0b1010").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(10)));
}

#[test]
fn test_int_binary_underscore() {
    let tokens = tokenize("0b1111_0000").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(240)));
}

// ============================================================================
// §2.6.2: 浮点数字面量
// ============================================================================

#[test]
fn test_float_simple() {
    let tokens = tokenize("3.14").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(v) if (v - 3.14).abs() < 0.001));
}

#[test]
fn test_float_trailing_dot() {
    let tokens = tokenize("42.").unwrap();
    // 可能是 FloatLiteral 或 IntLiteral + Dot
    let is_float = matches!(tokens[0].kind, TokenKind::FloatLiteral(_));
    let is_int_dot = tokens.len() >= 2
        && matches!(tokens[0].kind, TokenKind::IntLiteral(42))
        && matches!(tokens[1].kind, TokenKind::Dot);
    assert!(is_float || is_int_dot);
}

#[test]
fn test_float_exponent() {
    let tokens = tokenize("1.5e10").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
}

#[test]
fn test_float_negative_exponent() {
    let tokens = tokenize("1e-10").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
}

#[test]
fn test_float_underscore() {
    let tokens = tokenize("1_000.5").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(v) if (v - 1000.5).abs() < 0.001));
}

// ============================================================================
// §2.6.3: 字符串字面量
// ============================================================================

#[test]
fn test_string_simple() {
    let tokens = tokenize(r#""hello""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "hello"));
}

#[test]
fn test_string_empty() {
    let tokens = tokenize(r#""""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s.is_empty()));
}

#[test]
fn test_string_escape_n() {
    // \n → newline
    let tokens = tokenize(r#""a\nb""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "a\nb"));
}

#[test]
fn test_string_escape_t() {
    let tokens = tokenize(r#""a\tb""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "a\tb"));
}

#[test]
fn test_string_escape_quote() {
    let tokens = tokenize(r#""\"""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "\""));
}

#[test]
fn test_string_escape_backslash() {
    let tokens = tokenize(r#""\\""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "\\"));
}

#[test]
fn test_string_unicode_escape() {
    // \u{1F600} → 😀
    let tokens = tokenize(r#""\u{1F600}""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "😀"));
}

#[test]
fn test_string_hex_escape() {
    // \x48 → 'H'
    let tokens = tokenize(r#""\x48""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "H"));
}

#[test]
fn test_string_unicode_in_source() {
    let tokens = tokenize("\"😀\"").unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "😀"));
}

// ============================================================================
// 字符字面量
// ============================================================================

#[test]
fn test_char_simple() {
    let tokens = tokenize("'a'").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::CharLiteral('a')));
}

#[test]
fn test_char_escape() {
    let tokens = tokenize(r"'\n'").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::CharLiteral('\n')));
}

// ============================================================================
// F-String (RFC-012)
// ============================================================================

#[test]
fn test_fstring_simple() {
    let tokens = tokenize(r#"f"hello""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "hello"));
}

#[test]
fn test_fstring_interpolation() {
    let tokens = tokenize(r#"f"hello {name}""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(_)));
}

#[test]
fn test_fstring_escape_brace() {
    // {{ → literal {
    let tokens = tokenize(r#"f"hello{{"#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "hello{"));
}

// ============================================================================
// 布尔字面量
// ============================================================================

#[test]
fn test_bool_true() {
    let tokens = tokenize("true").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::BoolLiteral(true)));
}

#[test]
fn test_bool_false() {
    let tokens = tokenize("false").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::BoolLiteral(false)));
}

// ============================================================================
// 负数（一元运算符 + 数字）
// ============================================================================

#[test]
fn test_negative_int() {
    let tokens = tokenize("-42").unwrap();
    assert_eq!(tokens.len(), 3); // Minus, IntLiteral(42), Eof
    assert!(matches!(tokens[0].kind, TokenKind::Minus));
    assert!(matches!(tokens[1].kind, TokenKind::IntLiteral(42)));
}

#[test]
fn test_negative_float() {
    let tokens = tokenize("-3.14").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::Minus));
    assert!(matches!(tokens[1].kind, TokenKind::FloatLiteral(_)));
}
