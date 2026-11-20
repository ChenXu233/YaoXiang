//! Lexer 字面量测试 — 基于语言规范 syntax.md §1.6
//!
//! §1.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §1.6.2: 浮点数 (带小数点和指数)
//! §1.6.3: 字符串 (转义序列 \\nrt'"\\, \\x, \\u{})
//! §1.6.4: 集合字面量 (由 parser 处理)
//! RFC-012: F-String 插值

use crate::frontend::core::lexer::{tokenize, LexError, TokenKind};

/// 取单个整数 token 的值；非整数或词法错误直接失败并带上源码上下文。
fn single_int_literal(source: &str) -> i128 {
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("tokenize({source:?}) 失败: {e:?}"));
    match &tokens[0].kind {
        TokenKind::IntLiteral(value) => *value,
        other => panic!("期望 IntLiteral，实际 {other:?}: {source:?}"),
    }
}

/// 取词法错误携带的非法字面量文本；不是 E0002 的 InvalidNumber 就失败。
fn invalid_number_literal(source: &str) -> String {
    match tokenize(source) {
        Err(LexError::InvalidNumber(literal, _)) => literal,
        other => panic!("期望 InvalidNumber，实际 {other:?}: {source:?}"),
    }
}

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
    assert!(matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s.raw == "hello"));
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

// ===== §1.6.1 四基数扫描器的差异面（RFC-039 8.2.1 收敛后回归）=====

#[test]
fn test_radix_literals_normal_values() {
    // 规范 §1.6.1: 四进制整数 BNF（Decimal / Octal / Hex / Binary）
    let cases = [("0x1F", 31), ("0o17", 15), ("0b101", 5), ("42", 42)];

    for (source, expected) in cases {
        assert_eq!(single_int_literal(source), expected, "字面量 {source:?}");
    }
}

#[test]
fn test_radix_literals_underscore_separator_accepted() {
    // 规范 §1.6.1: 下划线只作数字分隔符
    let cases = [("0x1_F", 31), ("0o1_7", 15), ("0b1_01", 5), ("1_000", 1000)];

    for (source, expected) in cases {
        assert_eq!(single_int_literal(source), expected, "字面量 {source:?}");
    }
}

#[test]
fn test_radix_literals_uppercase_prefix_accepted() {
    // 规范 §1.6.1: 进制前缀大小写均可
    let cases = [("0X1F", 31), ("0O17", 15), ("0B101", 5)];

    for (source, expected) in cases {
        assert_eq!(single_int_literal(source), expected, "字面量 {source:?}");
    }
}

#[test]
fn test_radix_literals_trailing_alnum_rejected_with_literal_text() {
    // 规范 §1.6.1: 字面量后不得紧跟字母 / 数字 / 下划线（E0002）；
    // 语料只有 hex / binary / decimal 三条夹具，octal 的两条由本测兜住。
    // Arrange
    let cases = [
        ("0x1FG", "0x1FG"),
        ("0o1G", "0o1G"),
        ("0o128", "0o128"),
        ("0b102", "0b102"),
        ("123abc", "123a"),
    ];

    // Act + Assert：错误参数是「已扫描字面量 + 首个非法字符」
    for (source, expected) in cases {
        assert_eq!(
            invalid_number_literal(source),
            expected,
            "字面量 {source:?}"
        );
    }
}

#[test]
fn test_radix_literals_overflow_rejected_with_literal_text() {
    // 规范 §1.6.1: 整数字面量取值 i128；u128 累积溢出后仍继续消费输入。
    // Arrange
    let cases = [
        format!("0x{}", "F".repeat(33)),
        format!("0x8{}", "0".repeat(31)),
        format!("0o{}", "7".repeat(43)),
        format!("0b1{}", "0".repeat(128)),
        "9".repeat(40),
    ];

    // Act + Assert：溢出错误携带完整字面量
    for source in cases {
        assert_eq!(
            invalid_number_literal(&source),
            source,
            "溢出字面量必须原样进诊断"
        );
    }
}

#[test]
fn test_multiline_string_unknown_escape_preserved() {
    // 规范 §1.6.3: 多行字符串只解码 n / t / r / 引号 / 反斜杠，其余转义原样保留。
    // Arrange
    let source = "\"\"\"a\\qb\"\"\"";

    // Act
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("tokenize 失败: {e:?}"));

    // Assert
    assert!(
        matches!(&tokens[0].kind, TokenKind::StringLiteral(value) if value == "a\\qb"),
        "多行串未知转义必须原样保留: {tokens:?}"
    );
}

#[test]
fn test_multiline_fstring_escape_decoded_strictly() {
    // RFC-012: f-string 的反斜杠转义在词法期解码，多行与单行同口径。
    // Arrange
    let source = "f\"\"\"a\\x41\"\"\"";

    // Act
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("tokenize 失败: {e:?}"));

    // Assert
    assert!(
        matches!(&tokens[0].kind, TokenKind::FStringLiteral(value) if value.raw == "aA"),
        "多行 f-string 必须解码十六进制转义: {tokens:?}"
    );
}

#[test]
fn test_char_literal_unknown_escape_kept_as_char() {
    // 规范 §1.6.3: 字符字面量的未知转义取该字符本身（历史行为）。
    // Arrange
    let source = "'\\q'";

    // Act
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("tokenize 失败: {e:?}"));

    // Assert
    assert!(
        matches!(tokens[0].kind, TokenKind::CharLiteral('q')),
        "字符字面量未知转义应取字符本身: {tokens:?}"
    );
}
