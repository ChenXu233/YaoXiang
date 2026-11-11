//! Tests for nud.rs — prefix expression parsing
//!
//! Covers: prefix_info mapping, prefix parsers for literals, identifiers,
//! unary ops, control flow, ref, unsafe, eval block, spawn.

use crate::frontend::core::lexer::tokenize;
use crate::ast::{Expr, UnOp};
use crate::frontend::core::parser::{NullProbe, ParserState};

fn parse_expr(source: &str) -> Expr {
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens, &NullProbe);
    state
        .parse_expression(crate::frontend::core::parser::BP_LOWEST)
        .expect("parse failed")
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
