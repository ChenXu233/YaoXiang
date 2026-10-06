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

// §1.6.2: 浮点数字面量

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

// §1.6.3: 字符串字面量

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

// 字符字面量

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

// F-String (RFC-012)

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
    // #402：`{{` 在 raw 中逐字保留，解转义发生在 parser 切段
    let tokens = tokenize(r#"f"hello{{""#).unwrap();
    assert!(
        matches!(&tokens[0].kind, TokenKind::FStringLiteral(s) if s == "hello{{"),
        "`{{` 必须逐字保留为两个字符"
    );
}

// 布尔字面量

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

// 负数（一元运算符 + 数字）

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

// ===== §1.6 补测：溢出 / 非法转义 / 前导点小数（RFC-039 P1 1.3，2026-10-06）=====
// 这些路径此前从未被测试（见 06-cleanup-inventory §A）；按规范与一致性要求补测。

#[test]
fn test_hex_overflow_is_error() {
    // Arrange：超出 i128 上限的十六进制字面量
    // Act
    let result = tokenize("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    // Assert：溢出必须报词法错误，不得静默截断
    assert!(result.is_err(), "超大十六进制字面量必须报错: {result:?}");
}

#[test]
fn test_octal_overflow_is_error() {
    let result = tokenize("0o7777777777777777777777777777777777777777777777");
    assert!(result.is_err(), "超大八进制字面量必须报错: {result:?}");
}

#[test]
fn test_binary_overflow_is_error() {
    let huge = format!("0b{}", "1".repeat(200));
    let result = tokenize(&huge);
    assert!(result.is_err(), "超大二进制字面量必须报错: {result:?}");
}

#[test]
fn test_decimal_overflow_is_error() {
    let result = tokenize("999999999999999999999999999999999999999999999");
    assert!(result.is_err(), "超大十进制字面量必须报错: {result:?}");
}

#[test]
fn test_int_above_i128_max_is_error() {
    // i128::MAX + 1：u128 累加不溢出，但目标类型放不下
    let result = tokenize("170141183460469231731687303715884105728");
    assert!(
        result.is_err(),
        "超过 i128::MAX 的整数字面量必须报错: {result:?}"
    );
}

#[test]
fn test_int_i128_max_is_valid() {
    // Arrange：i128::MAX（正例对照，证明上一个测试的边界判定不是一刀切）
    let result = tokenize("170141183460469231731687303715884105727");
    // Assert
    assert!(result.is_ok(), "i128::MAX 应合法: {result:?}");
    let tokens = result.unwrap();
    assert!(
        matches!(tokens[0].kind, TokenKind::IntLiteral(i128::MAX)),
        "应产出 IntLiteral(i128::MAX)，实际为 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_invalid_hex_escape_is_error() {
    // Arrange：\x 后不是两位十六进制（§1.6.3 转义闭集之外）
    let result = tokenize(r#""\xZZ""#);
    // Assert
    assert!(result.is_err(), "非法 \\x 转义必须报错: {result:?}");
}

#[test]
fn test_short_hex_escape_is_error() {
    // \x 只跟一位十六进制数字
    let result = tokenize(r#""\xF""#);
    assert!(result.is_err(), "不足两位的 \\x 转义必须报错: {result:?}");
}

#[test]
fn test_unicode_escape_out_of_range_is_error() {
    // \u{110000} 超出 Unicode 上限（char::from_u32 返回 None）
    let result = tokenize(r#""\u{110000}""#);
    assert!(result.is_err(), "超范围的 \\u 转义必须报错: {result:?}");
}

#[test]
fn test_empty_unicode_escape_is_error() {
    let result = tokenize(r#""\u{}""#);
    assert!(result.is_err(), "空的 \\u{{}} 转义必须报错: {result:?}");
}

#[test]
fn test_unknown_escape_is_error() {
    // §1.6.3 的转义闭集之外：\q 非法
    let result = tokenize(r#""\q""#);
    assert!(result.is_err(), "未知转义必须报错: {result:?}");
}

#[test]
fn test_valid_hex_and_unicode_escapes() {
    // Arrange：合法转义正例对照（\x41 = 'A'，\u{1F600} = 😀）
    // Act
    let result = tokenize(r#""\x41\u{1F600}""#);
    // Assert
    assert!(result.is_ok(), "合法转义不应报错: {result:?}");
    let tokens = result.unwrap();
    assert!(
        matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "A\u{1F600}"),
        "应解码为 A😀，实际为 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_leading_dot_float() {
    // Arrange：前导点小数 .5（scan_leading_dot 路径）
    // Act
    let tokens = tokenize(".5").unwrap();
    // Assert
    assert!(
        matches!(tokens[0].kind, TokenKind::FloatLiteral(v) if v == 0.5),
        ".5 应产出 FloatLiteral(0.5)，实际为 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_leading_dot_float_with_exponent() {
    let tokens = tokenize(".5e2").unwrap();
    assert!(
        matches!(tokens[0].kind, TokenKind::FloatLiteral(v) if v == 50.0),
        ".5e2 应产出 FloatLiteral(50.0)，实际为 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_leading_dot_trailing_underscore_is_error() {
    // 小数段下划线必须位于数字之间（与浮点段同一规则）
    let result = tokenize(".5_");
    assert!(result.is_err(), "末尾下划线必须报错: {result:?}");
}

// ===== §1.6.1 非法基数数字（RFC-039 P1 1.3.1 探针暴露的真实缺陷，先红后绿）=====
// 现状缺陷：0b102 静默拆分接受（运行得 2）、0o128 同（得 10）、
// 0x1FG / 123abc 报误导性 E1001「未知变量」。文法 §1.6.1 不允许这些形态，
// 词法层必须在非法字母数字邻接处直接报错。

#[test]
fn test_binary_invalid_trailing_digit_is_error() {
    // Arrange：0b 后混入非法数字 2（文法仅允许 [01][01_]*）
    // Act
    let result = tokenize("0b102");
    // Assert：非法数字邻接必须报词法错误，不得静默拆成 0b10 + 2
    assert!(
        result.is_err(),
        "0b102 必须报词法错误（非法二进制数字）: {result:?}"
    );
}

#[test]
fn test_octal_invalid_trailing_digit_is_error() {
    let result = tokenize("0o128");
    assert!(
        result.is_err(),
        "0o128 必须报词法错误（非法八进制数字）: {result:?}"
    );
}

#[test]
fn test_hex_invalid_trailing_letter_is_error() {
    // 0x1FG：G 不是十六进制数字，不得留给下游报「未知变量 G」
    let result = tokenize("0x1FG");
    assert!(
        result.is_err(),
        "0x1FG 必须报词法错误（非法十六进制字符）: {result:?}"
    );
}

#[test]
fn test_decimal_invalid_trailing_letter_is_error() {
    // 123abc：字面量后紧跟字母不得静默拆分（下游 E1001 误导用户）
    let result = tokenize("123abc");
    assert!(
        result.is_err(),
        "123abc 必须报词法错误（整数字面量后紧跟字母）: {result:?}"
    );
}

#[test]
fn test_leading_dot_invalid_trailing_letter_is_error() {
    // .5x：前导点小数后紧跟字母，同上缺陷类（下游报误导性 E1001）
    let result = tokenize(".5x");
    assert!(result.is_err(), ".5x 必须报词法错误: {result:?}");
}
