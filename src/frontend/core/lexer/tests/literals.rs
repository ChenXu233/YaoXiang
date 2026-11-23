//! Lexer 字面量测试 — 基于语言规范 syntax.md §1.6
//!
//! §1.6.1: 整数 Decimal, Octal(0o), Hex(0x), Binary(0b)
//! §1.6.2: 浮点数 (带小数点和指数)
//! §1.6.3: 字符串 (转义序列 \\nrt'"\\, \\x, \\u{})
//! §1.6.4: 集合字面量 (由 parser 处理)
//! RFC-012: F-String 插值

use crate::frontend::core::lexer::{tokenize, LexError, Lexer, TokenKind};

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

// ===== RFC-039 8.8.2：05 阶段 4 登记的四类字面量错误路径补测（2026-11-23）=====
//
// 判据门（coding-rules 7.2，逐条标注在各用例注释上）：
// - **条款门**只用于规范正文确有该形态的判据：SPEC syntax §1.6.1（四基数整数文法、下划线
//   只作数字分隔符）。
// - **证明门 / 病史门**用于「实现已支持、但规范正文未写」的形态：前导点小数（`.5` 等）与
//   `\x` / `\u` 转义。SPEC §1.6.2 的 Float 文法要求整数部分、§1.6.3 的 Escape 产生式
//   只含 `[nrt'"\\]` 与 `UnicodeEscape`，两处都没有前导点与 `\x`，故**不得**按条款门记；
//   本批按「同一输入的 token 流 / 诊断不变」（C5 行为保持）与现成病史（提交 1775f378：
//   `t.0` 被读成 `t + 0.0`）记。补 SPEC 正文留给语言设计流程，本批不做。
// 错误类判据只断言词法错误**变体**与码号 **E0002（InvalidNumber）/ E0012（InvalidEscape）**，
// 不冻消息措辞与 span 数值——那是实现形状（coding-rules 7.1）。
//
// 四类中「四基数溢出」已由 8.2.1 的 test_radix_literals_overflow_rejected_with_literal_text
// 覆盖（四进制 + 十进制各一例，含 i128 收窄那一支），本批不重复造；它「溢出后继续消费
// 输入」的一面由下面的 test_radix_overflow_consumes_all_digits_before_next_token 直驱观察。
// 本段新增 10 条：溢出后继续消费 1 + 前导点 5 + 非法转义 3（字符串）+ 字符 `\x` 1。

#[test]
fn test_radix_overflow_consumes_all_digits_before_next_token() {
    // 条款：SPEC syntax §1.6.1——字面量取值 i128，溢出报 E0002；溢出点之后的数字
    // 仍属同一个字面量，不得被切成第二个 token（否则一条错误会连带一条意外诊断）。
    // 判据形态：tokenize 在首个错误即返回，故用 Lexer::next_token 直驱观察其后 token。
    // Arrange
    let source = format!("0x{}42", "F".repeat(33));
    let mut lexer = Lexer::new(&source);

    // Act
    let first = lexer.next_token().expect("溢出字面量应产出一个错误 token");
    let after = lexer.next_token();

    // Assert：错误 token + E0002，且词法位置已越过全部数字（下一 token 为流结束）
    assert!(
        matches!(first.kind, TokenKind::Error(_)),
        "§1.6.1: 溢出字面量应产出错误 token，实际 {:?}",
        first.kind
    );
    assert!(
        matches!(lexer.error, Some(LexError::InvalidNumber(..))),
        "§1.6.1: 溢出必须以 E0002 记录，实际 {:?}",
        lexer.error
    );
    assert!(
        after.is_none(),
        "§1.6.1: 溢出点之后的数字必须已被消费（下一 token 应为流结束），实际 {:?}",
        after.map(|t| t.kind)
    );
}

#[test]
fn test_leading_dot_float_literal_value() {
    // 证明门（C5 行为保持）：`.5` 与 `0.5` 必须产出同一个浮点值——同一输入在 8.3–8.6 迁移
    // 前后 token 流不变。病史：提交 1775f378 为让 `t.0`（元组下标）不再读成 `t + 0.0`，把
    // `.` 后的判定改为看上一个 token；`.5` 在行首/运算符后仍须是浮点字面量。
    // Arrange
    let source = ".5";

    // Act
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("tokenize({source:?}) 失败: {e:?}"));

    // Assert
    assert!(
        matches!(tokens[0].kind, TokenKind::FloatLiteral(v) if (v - 0.5).abs() < f64::EPSILON),
        "证明门（C5）：.5 应词法化为 0.5，实际 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_leading_dot_exponent_decoded() {
    // 证明门（C5 行为保持）：指数部分不改变前导点形态的取值——`.5e3` 与 `0.5e3` 同值。
    // 病史同上一用例（1775f378 的 `t.0` 判定改动未回退前导点路径）。
    // Arrange
    let source = ".5e3";

    // Act
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("tokenize({source:?}) 失败: {e:?}"));

    // Assert
    assert!(
        matches!(tokens[0].kind, TokenKind::FloatLiteral(v) if (v - 500.0).abs() < f64::EPSILON),
        "证明门（C5）：.5e3 应词法化为 500.0，实际 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_leading_dot_underscore_between_digits_accepted() {
    // 证明门（C5 行为保持）：分隔符夹在数字之间时必须被跳过——`.5_5` 与 `.55` 同值。
    // 条款门只覆盖整数形态（SPEC §1.6.1）；前导点小数不在 SPEC §1.6.2 的文法内，故按 C5 记。
    // Arrange
    let source = ".5_5";

    // Act
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("tokenize({source:?}) 失败: {e:?}"));

    // Assert
    assert!(
        matches!(tokens[0].kind, TokenKind::FloatLiteral(v) if (v - 0.55).abs() < 1e-12),
        "证明门（C5）：.5_5 应词法化为 0.55，实际 {:?}",
        tokens[0].kind
    );
}

#[test]
fn test_leading_dot_underscore_before_nondigit_rejected() {
    // **特征化（不是门禁）**：本条钉的是现行为——前导点路径要求分隔符两侧都有数字，
    // 而整数路径不要求（实测 `1_` / `0x1_` / `1__0` / `0x1__F` 全部接受）。两种口径不一致，
    // 已按缺陷登记（09a §P8 8.8 六 D5）；修那边时本条同步改为「与整数路径同判」。
    // Arrange
    let source = ".5_";

    // Act
    let result = tokenize(source);

    // Assert
    assert!(
        matches!(result, Err(LexError::InvalidNumber(..))),
        "特征化（D5 现行为）：.5_ 目前被拒绝（同族的 1_ / 0x1_ 却被接受），实际 {result:?}"
    );
}

#[test]
fn test_leading_dot_exponent_without_digits_rejected() {
    // 证明门（C5 行为保持）：指数符号后必须有数字——缺数字时不得静默丢弃 `e` 或产出浮点值
    // （否则同一输入会从「报错」变成「另一个值」）。前导点形态不在 SPEC §1.6.2 文法内。
    // Arrange
    let source = ".5e";

    // Act
    let result = tokenize(source);

    // Assert
    assert!(
        matches!(result, Err(LexError::InvalidNumber(..))),
        "证明门（C5）：.5e 必须以 E0002 拒绝，实际 {result:?}"
    );
}

#[test]
fn test_string_hex_escape_requires_two_digits() {
    // 证明门：非法转义序列不得被静默接受或丢弃（若吞掉 `\x`，源码含义会被静默改写为
    // 另一个字符）；这是与实现选择无关的性质——任何正确实现都只能报错或按显式规则解码。
    // 病史：`\x` / `\u` 在 8.8.2 之前从未被任何用例覆盖（P1 1.3.2 只补了 `.5x` 一条）。
    // SPEC §1.6.3 的 Escape 产生式不含 `\x`，故不按条款门记。码号 E0012（InvalidEscape）。
    // Arrange
    let cases = [r#""\x1""#, r#""\x""#, r#""\xZZ""#];

    // Act + Assert
    for source in cases {
        let result = tokenize(source);
        assert!(
            matches!(result, Err(LexError::InvalidEscape { .. })),
            "证明门：非法 \\x 转义必须以 E0012（InvalidEscape）拒绝，字面量 {source:?} 实际 {result:?}"
        );
    }
}

#[test]
fn test_string_unicode_escape_requires_braced_codepoint() {
    // 证明门：`\u` 后必须跟 `{HexDigit+}` 且码点必须是 Unicode 标量值——
    // ①缺花括号/空码点：转义形态不完整，任何实现都只能报错（不得把 `\u` 当普通字符吞掉）；
    // ②越界码点 `\u{110000}`：按 Unicode 标量语义（0..=0x10FFFF 且非代理区）不是合法标量，
    // 拒绝与实现无关。SPEC §1.6.3 只写了 `u{HexDigit+}` 的形态，未写越界语义，故本条记证明门。
    // Arrange：缺花括号 / 空码点 / 越界码点各一例
    let cases = [r#""\u41""#, r#""\u{}""#, r#""\u{110000}""#];

    // Act + Assert
    for source in cases {
        let result = tokenize(source);
        assert!(
            matches!(result, Err(LexError::InvalidEscape { .. })),
            "证明门：非法 \\u 转义必须以 E0012（InvalidEscape）拒绝，字面量 {source:?} 实际 {result:?}"
        );
    }
}

#[test]
fn test_string_unknown_escape_rejected() {
    // 条款门：SPEC syntax §1.6.3——Escape 产生式只含 `[nrt'"\\]` 与 `UnicodeEscape`，
    // 单行字符串的其余转义序列非法（本条是唯一由条款门覆盖的转义用例：`\q` 不在文法内）。
    // Arrange
    let source = r#""\q""#;

    // Act
    let result = tokenize(source);

    // Assert
    assert!(
        matches!(result, Err(LexError::InvalidEscape { .. })),
        "§1.6.3 条款门：未知转义必须以 E0012（InvalidEscape）拒绝，实际 {result:?}"
    );
}

#[test]
fn test_char_hex_escape_decodes_two_digits_and_rejects_one() {
    // 证明门（C5 行为保持）：字符字面量与字符串对 `\x` 的口径一致——两位十六进制解码成该
    // 字符，不足两位必须拒绝（不得静默产出字符 token）。「未知转义取字符本身」只覆盖非
    // `\x` / `\u` 的序列（见 test_char_literal_unknown_escape_kept_as_char，两者不混）。
    // SPEC §1.6.3 的 Escape 产生式不含 `\x`，故不按条款门记。
    // Arrange：合法两例（\x41 → 'A'）与不足两位一例
    let ok_source = r"'\x41'";
    let bad_source = r"'\x1'";

    // Act
    let ok = tokenize(ok_source);
    let bad = tokenize(bad_source);

    // Assert：合法形态解码成字符；非法形态必须被拒绝（不得静默产出字符 token）。
    // 现树对非法形态以 Empty character literal 收尾、未保留 InvalidEscape（E0012）——
    // 诊断保真偏差已登记（09a §P8 8.8 实录），此断言只守「必须拒绝」这条性质。
    assert!(
        matches!(&ok, Ok(tokens) if matches!(tokens[0].kind, TokenKind::CharLiteral('A'))),
        "证明门（C5）：'\\x41' 应解码为字符 A，实际 {ok:?}"
    );
    assert!(
        bad.is_err(),
        "证明门（C5）：不足两位的 \\x 转义必须拒绝，实际 {bad:?}"
    );
}
