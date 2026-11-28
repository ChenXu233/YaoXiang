//! F-string lexer 测试 — 基于语言规范 syntax.md §1.6.3 字符串字面量 + RFC-012
//!
//! raw 协议（#402）：花括号序列（含 `{{` / `}}` 转义）逐字保留，
//! 由 parser 切段时解转义；反斜杠转义在 lexer 解码。
//!
//! 插值段 base（#574 / WBS 8.2.3）：`FStringTokenSegment::Interpolation.base`
//! 是 expr 起始字符在源文件中的绝对位置，插值子词法以它为词法起点。
//! 门依据：病史（issue #574——多行/嵌套 f-string 报错位置落到文件首行）+
//! 证明（性质：段 base 必须等于 expr 首字符的源文件绝对位置，与实现选择
//! 无关；文本段转义解码使 raw 偏移与源偏移错位，故只能扫描期锚点记录）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::lexer::tokens::{FStringTokenSegment, TokenKind};
use crate::util::span::Position;

fn assert_fstring_raw(
    source: &str,
    expected: &str,
) {
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("tokenize({source:?}) 失败: {e:?}"));
    match &tokens[0].kind {
        TokenKind::FStringLiteral(raw) => {
            assert_eq!(raw.raw, expected, "f-string raw 内容不符: {source:?}")
        }
        other => panic!("期望 FStringLiteral，实际 {other:?}: {source:?}"),
    }
}

/// 取 source 首个 token 的插值段 base（只支持恰好一个插值段的输入）。
fn interpolation_base(source: &str) -> Position {
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("tokenize({source:?}) 失败: {e:?}"));
    match &tokens[0].kind {
        TokenKind::FStringLiteral(fstring) => {
            let bases: Vec<&Position> = fstring
                .segments
                .iter()
                .filter_map(|segment| match segment {
                    FStringTokenSegment::Interpolation { base, .. } => Some(base),
                    FStringTokenSegment::Text(_) => None,
                })
                .collect();
            assert_eq!(bases.len(), 1, "期望恰好一个插值段: {source:?}");
            *bases[0]
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

// 插值段 base：源文件绝对位置（#574 / WBS 8.2.3）

#[test]
fn test_fstring_interpolation_base_is_absolute_source_position() {
    // Arrange
    // f(1) "(2) {(3) g(4)…（1-indexed；offset 0-based：`{` 在 2，`g` 在 3）
    let source = r#"f"{ghost}""#;

    // Act
    let base = interpolation_base(source);

    // Assert：base 指向 expr 首字符的源文件绝对位置
    assert_eq!(base.line, 1, "单行 f-string 的插值 base 行号");
    assert_eq!(base.column, 4, "单行 f-string 的插值 base 列号");
    assert_eq!(base.offset, 3, "单行 f-string 的插值 base 字节偏移");
}

#[test]
fn test_fstring_interpolation_base_survives_text_escape() {
    // Arrange：文本段 `\n` 两字符解码成一字符——raw 偏移与源偏移错位，
    // 事后从 raw 反推会得到列 6；扫描期锚点必须给出列 7
    let source = r#"f"a\n{ghost}""#;

    // Act
    let base = interpolation_base(source);

    // Assert
    assert_eq!(base.line, 1, "转义相邻插值的 base 行号");
    assert_eq!(base.column, 7, "转义相邻插值的 base 列号（含转义解码错位）");
    assert_eq!(
        base.offset, 6,
        "转义相邻插值的 base 字节偏移（\\n 占两字节）"
    );
}

#[test]
fn test_fstring_interpolation_base_advances_leading_whitespace() {
    // Arrange：插值内前导空白——base 必须从 `{` 后推进到 trim 后首字符
    let source = r#"f"{ spaced }""#;

    // Act
    let base = interpolation_base(source);

    // Assert
    assert_eq!(base.line, 1, "前导空白插值的 base 行号");
    assert_eq!(base.column, 5, "前导空白插值的 base 列号");
    assert_eq!(base.offset, 4, "前导空白插值的 base 字节偏移");
}

#[test]
fn test_fstring_multiline_interpolation_base_points_to_source_line() {
    // Arrange：多行 f-string——插值在第二行
    let source = "f\"\"\"head\nmid {gamma}\ntail\"\"\"";

    // Act
    let base = interpolation_base(source);

    // Assert：base 落在源文件第二行，不是插值文本内部的 1
    assert_eq!(base.line, 2, "多行 f-string 的插值 base 行号");
    assert_eq!(base.column, 6, "多行 f-string 的插值 base 列号");
}
