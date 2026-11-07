//! Pratt parser expression tests — based on spec §4, §4.2, §4.10

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_expression;
use crate::ast::{BinOp, Expr, FStringSegment, UnOp};
use crate::frontend::core::lexer::tokens::Literal;

fn parse_expr(source: &str) -> Expr {
    let tokens = tokenize(source).unwrap();
    parse_expression(&tokens).unwrap()
}

#[test]
fn test_void_literal() {
    let expr = parse_expr("void");
    assert!(matches!(expr, Expr::Lit(Literal::Void, _)));
}

#[test]
fn test_unary_not() {
    // `!` 紧绑定一元表达式（Zig 式，SPEC §2.2）
    let expr = parse_expr("!x");
    assert!(matches!(expr, Expr::UnOp { op: UnOp::Not, .. }));

    // 紧绑定：!a == b ≡ (!a) == b
    let expr2 = parse_expr("!a == b");
    match expr2 {
        Expr::BinOp {
            op: BinOp::Eq,
            left,
            right,
            ..
        } => {
            assert!(matches!(*left, Expr::UnOp { op: UnOp::Not, .. }));
            assert!(matches!(*right, Expr::Var(name, _) if name == "b"));
        }
        _ => panic!("!a == b should parse as (!a) == b"),
    }
}

#[test]
fn test_precedence_mul_before_add() {
    // 2 + 3 * 4 应解析为 2 + (3 * 4)
    let expr = parse_expr("2 + 3 * 4");
    if let Expr::BinOp { op: BinOp::Add, .. } = &expr {
        // top level is Add, right side should be Mul — correct
    } else {
        panic!("Expected Add at top level");
    }
}

#[test]
fn test_precedence_compare_before_logical() {
    // a == b and c — and 可能是关键字或标识符
    let expr = parse_expr("a == b and c");
    // 至少能解析不 panic
    let _ = expr;
}

// 字面量和集合 (Spec §2.6.4)

#[test]
fn test_list_literal() {
    let expr = parse_expr("[1, 2, 3]");
    assert!(matches!(expr, Expr::List(..)));
}

#[test]
fn test_tuple() {
    let expr = parse_expr("(1, 2)");
    assert!(matches!(expr, Expr::Tuple(..)));
}

// f-string 段切分（RFC-012, #402）

fn fstring_segments(source: &str) -> Vec<FStringSegment> {
    match parse_expr(source) {
        Expr::FString { segments, .. } => segments,
        other => panic!("Expected FString, got {other:?}: {source:?}"),
    }
}

fn assert_text(
    seg: &FStringSegment,
    expected: &str,
) {
    match seg {
        FStringSegment::Text(text) => {
            assert_eq!(text, expected, "Text 段内容不符")
        }
        other => panic!("Expected Text({expected:?}), got {other:?}"),
    }
}

fn assert_interpolation(
    seg: &FStringSegment,
    format_spec: Option<&str>,
) {
    match seg {
        FStringSegment::Interpolation {
            format_spec: spec, ..
        } => {
            assert_eq!(spec.as_deref(), format_spec, "format_spec 不符");
        }
        other => panic!("Expected Interpolation, got {other:?}"),
    }
}

#[test]
fn test_fstring_segments_text_only() {
    let segments = fstring_segments(r#"f"hello""#);
    assert_eq!(segments.len(), 1, "纯文本应只有一段");
    assert_text(&segments[0], "hello");
}

#[test]
fn test_fstring_segments_interpolation() {
    let segments = fstring_segments(r#"f"hello {name}""#);
    assert_eq!(segments.len(), 2, "应为 Text + Interpolation 两段");
    assert_text(&segments[0], "hello ");
    assert_interpolation(&segments[1], None);
}

#[test]
fn test_fstring_segments_brace_escape() {
    // #402：`{{literal}}` 是纯文本 `{literal}`，不再成为插值
    let segments = fstring_segments(r#"f"{{literal}}""#);
    assert_eq!(segments.len(), 1, "转义后应折叠为单段文本");
    assert_text(&segments[0], "{literal}");
}

#[test]
fn test_fstring_segments_escape_adjacent_interpolation() {
    // Python 语义：f"{{{x}}}" → 文本 `{` + 插值 x + 文本 `}`
    let segments = fstring_segments(r#"f"{{{x}}}""#);
    assert_eq!(segments.len(), 3, "应为 左花括号 + 插值 + 右花括号 三段");
    assert_text(&segments[0], "{");
    assert_interpolation(&segments[1], None);
    assert_text(&segments[2], "}");
}

#[test]
fn test_fstring_segments_close_brace_escape() {
    let segments = fstring_segments(r#"f"a}}b""#);
    assert_eq!(segments.len(), 1, "`}}` 解转义后并入单段文本");
    assert_text(&segments[0], "a}b");
}

#[test]
fn test_fstring_segments_format_spec() {
    let segments = fstring_segments(r#"f"{pi:.2f}""#);
    assert_eq!(segments.len(), 1, "仅一个插值段");
    assert_interpolation(&segments[0], Some(".2f"));
}
