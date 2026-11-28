//! Tests for nud.rs — prefix expression parsing
//!
//! Covers: prefix_info mapping, prefix parsers for literals, identifiers,
//! unary ops, control flow, ref, unsafe, eval block, spawn, f-string
//! interpolation spans.

use crate::ast::{Expr, FStringSegment, UnOp};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::{NullProbe, ParserState};
use crate::util::span::Span;

fn parse_expr(source: &str) -> Expr {
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens, &NullProbe);
    state
        .parse_expression(crate::frontend::core::parser::BP_LOWEST)
        .expect("parse failed")
}

/// 递归收集表达式内全部 Var 节点的 (名, span)。只走本文件涉及的形状
/// （FString 插值段、二元运算）；测试需要更多形状时再扩。
fn collect_var_spans<'a>(
    expr: &'a Expr,
    out: &mut Vec<(&'a str, Span)>,
) {
    match expr {
        Expr::Var(name, span) => out.push((name, *span)),
        Expr::FString { segments, .. } => {
            for segment in segments {
                if let FStringSegment::Interpolation { expr, .. } = segment {
                    collect_var_spans(expr, out);
                }
            }
        }
        Expr::BinOp { left, right, .. } => {
            collect_var_spans(left, out);
            collect_var_spans(right, out);
        }
        _ => {}
    }
}

#[test]
fn test_prefix_info_at() {
    // Phase 1: @ 不再是前缀表达式（eval block 已移除）
    let tokens = tokenize("@").unwrap();
    let state = ParserState::new(&tokens, &NullProbe);
    assert!(state.prefix_info().is_none(), "@ 不再是有效的前缀表达式");
}

// #314：break/continue 仅语句位置（Python 风格定案），不再注册为 pratt 前缀表达式

#[test]
fn test_prefix_info_break_removed() {
    let tokens = tokenize("break").unwrap();
    let state = ParserState::new(&tokens, &NullProbe);
    assert!(
        state.prefix_info().is_none(),
        "break 不应注册为 pratt 前缀表达式（#314 仅语句位置）"
    );
}

#[test]
fn test_prefix_info_continue_removed() {
    let tokens = tokenize("continue").unwrap();
    let state = ParserState::new(&tokens, &NullProbe);
    assert!(
        state.prefix_info().is_none(),
        "continue 不应注册为 pratt 前缀表达式（#314 仅语句位置）"
    );
}

#[test]
fn test_prefix_info_eof() {
    let tokens = tokenize("").unwrap();
    let state = ParserState::new(&tokens, &NullProbe);
    assert!(state.prefix_info().is_none());
}

// Prefix parsers: 字面量

#[test]
fn test_parse_int_literal() {
    let expr = parse_expr("42");
    assert!(matches!(expr, Expr::Lit(..)));
}

#[test]
fn test_parse_wildcard() {
    let expr = parse_expr("_");
    assert!(matches!(expr, Expr::Var(..)));
}

// Prefix parsers: 一元运算符

#[test]
fn test_parse_unary() {
    let expr = parse_expr("-x");
    assert!(matches!(expr, Expr::UnOp { op: UnOp::Neg, .. }));
}

#[test]
fn test_parse_return_expr() {
    let expr = parse_expr("return 42");
    assert!(matches!(expr, Expr::Return(..)));
}

#[test]
fn test_parse_block() {
    let expr = parse_expr("{ 42 }");
    assert!(matches!(expr, Expr::Block(..)));
}

// Prefix parsers: f-string（RFC-012）
//
// 插值内 AST 节点的 span 必须是源文件绝对位置（#574 / WBS 8.2.3）。
// 门依据：病史（issue #574——多行/嵌套 f-string 报错位置落到文件首行）+
// 证明（性质：插值子解析的 token span 以插值段 base 为原点，AST span 继承
// 自 token span——诊断与 LSP 语义 token 都按 span 定位，相对位置必然报错；
// 与实现选择无关）。四类登记形态逐条覆盖。

#[test]
fn test_fstring_interpolation_var_span_is_absolute() {
    // Arrange：形态①单插值——f(1) "(2) {(3)，ghost 从列 4 起
    let source = r#"f"{ghost}""#;

    // Act
    let expr = parse_expr(source);
    let mut vars = Vec::new();
    collect_var_spans(&expr, &mut vars);

    // Assert
    assert_eq!(vars.len(), 1, "单插值应恰好收集到一个 Var");
    let (name, span) = vars[0];
    assert_eq!(name, "ghost", "Var 名");
    assert_eq!(span.start.line, 1, "插值内 Var 的源文件行号");
    assert_eq!(span.start.column, 4, "插值内 Var 的源文件列号");
    assert_eq!(span.start.offset, 3, "插值内 Var 的源文件字节偏移");
}

#[test]
fn test_fstring_multi_interpolation_var_spans_are_absolute() {
    // Arrange：形态②多插值——alpha 列 4；beta 列 16（{ 在 15，中间文本 ` mid `）
    let source = r#"f"{alpha} mid {beta}""#;

    // Act
    let expr = parse_expr(source);
    let mut vars = Vec::new();
    collect_var_spans(&expr, &mut vars);

    // Assert
    assert_eq!(vars.len(), 2, "两个插值段各收集一个 Var");
    assert_eq!(vars[0].0, "alpha", "第一个插值的 Var 名");
    assert_eq!(vars[0].1.start.line, 1, "第一个 Var 的源文件行号");
    assert_eq!(vars[0].1.start.column, 4, "第一个 Var 的源文件列号");
    assert_eq!(vars[1].0, "beta", "第二个插值的 Var 名");
    assert_eq!(vars[1].1.start.line, 1, "第二个 Var 的源文件行号");
    assert_eq!(vars[1].1.start.column, 16, "第二个 Var 的源文件列号");
}

#[test]
fn test_fstring_nested_interpolation_var_span_is_absolute() {
    // Arrange：形态③嵌套 f-string——内层插值的 base 由外层子词法
    // 已含 base 的 position 组合而来：f(1) "(2) in(3,4) sp(5) {(6) sp(7)
    // f(8) "(9) {(10)，deep 从列 11 起
    let source = r#"f"in { f"{deep}" } tail""#;

    // Act
    let expr = parse_expr(source);
    let mut vars = Vec::new();
    collect_var_spans(&expr, &mut vars);

    // Assert
    assert_eq!(vars.len(), 1, "外层插值只包一个内层插值 Var");
    let (name, span) = vars[0];
    assert_eq!(name, "deep", "内层 Var 名");
    assert_eq!(span.start.line, 1, "嵌套插值内 Var 的源文件行号");
    assert_eq!(span.start.column, 11, "嵌套插值内 Var 的源文件列号");
}

#[test]
fn test_fstring_multiline_interpolation_var_span_is_absolute() {
    // Arrange：形态④多行 f-string——gamma 在源文件第二行第 6 列
    let source = "f\"\"\"head\nmid {gamma}\ntail\"\"\"";

    // Act
    let expr = parse_expr(source);
    let mut vars = Vec::new();
    collect_var_spans(&expr, &mut vars);

    // Assert
    assert_eq!(vars.len(), 1, "多行 f-string 应收集到一个 Var");
    let (name, span) = vars[0];
    assert_eq!(name, "gamma", "多行插值的 Var 名");
    assert_eq!(
        span.start.line, 2,
        "多行插值内 Var 的源文件行号（修前落第 1 行）"
    );
    assert_eq!(span.start.column, 6, "多行插值内 Var 的源文件列号");
}

#[test]
fn test_fstring_interpolation_spanning_lines_var_spans_are_absolute() {
    // Arrange：插值表达式自身跨行——子词法从 base 起继续数行
    let source = "f\"{alpha +\nbeta}\"";

    // Act
    let expr = parse_expr(source);
    let mut vars = Vec::new();
    collect_var_spans(&expr, &mut vars);

    // Assert：alpha 在第 1 行列 4；beta 在表达式内的第 2 行列 1
    assert_eq!(vars.len(), 2, "加法两端各收集一个 Var");
    assert_eq!(vars[0].1.start.line, 1, "跨行插值第一个 Var 的源文件行号");
    assert_eq!(vars[0].1.start.column, 4, "跨行插值第一个 Var 的源文件列号");
    assert_eq!(vars[1].1.start.line, 2, "跨行插值第二个 Var 的源文件行号");
    assert_eq!(vars[1].1.start.column, 1, "跨行插值第二个 Var 的源文件列号");
}
