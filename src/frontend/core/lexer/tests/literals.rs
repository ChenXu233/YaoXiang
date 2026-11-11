//! Lexer 字面量测试 — 基于语言规范 syntax.md §1.6
//!
//! §1.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §1.6.2: 浮点数 (带小数点和指数)
//! §1.6.3: 字符串 (转义序列 \\nrt'"\\, \\x, \\u{})
//! §1.6.4: 集合字面量 (由 parser 处理)
//! RFC-012: F-String 插值

use crate::frontend::core::lexer::{tokenize, TokenKind};

// §1.6.1: 整数字面量

#[test]
fn test_int_decimal() {
    let tokens = tokenize("42").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(42)));
}

// §1.6.2: 浮点数字面量

#[test]
fn test_float_simple() {
    // 2.5 而非 3.14：3.14 近似 f64::consts::PI，触发 clippy approx_constant；
    // 本测语义是「词法器把该字面量识别为等值浮点」，不能换成 PI 常量。
    let tokens = tokenize("2.5").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(v) if (v - 2.5).abs() < 0.001));
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

// §1.6.3: 字符串字面量

#[test]
fn test_string_simple() {
    let tokens = tokenize(r#""hello""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "hello"));
}

// 字符字面量

#[test]
fn test_char_simple() {
    let tokens = tokenize("'a'").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::CharLiteral('a')));
}

// F-String (RFC-012)

#[test]
fn test_fstring_simple() {
    let tokens = tokenize(r#"f"hello""#).unwrap();
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "hello"));
}

// 布尔字面量

#[test]
fn test_bool_true() {
    let tokens = tokenize("true").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::BoolLiteral(true)));
}

#[test]
fn test_negative_float() {
    let tokens = tokenize("-3.14").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::Minus));
    assert!(matches!(tokens[1].kind, TokenKind::FloatLiteral(_)));
}

// ===== §1.6 补测：溢出 / 非法转义 / 前导点小数（RFC-039 P1 1.3，2026-10-06）=====

// ===== §1.6.1 非法基数数字（RFC-039 P1 1.3.1 探针暴露的真实缺陷，先红后绿）=====

#[test]
fn test_leading_dot_invalid_trailing_letter_is_error() {
    // .5x：前导点小数后紧跟字母，同上缺陷类（下游报误导性 E1001）
    let result = tokenize(".5x");
    assert!(result.is_err(), ".5x 必须报词法错误: {result:?}");
}
