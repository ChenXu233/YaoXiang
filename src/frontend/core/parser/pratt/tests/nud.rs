//! Tests for nud.rs — prefix expression parsing
//!
//! Covers: prefix_info mapping, prefix parsers for literals, identifiers,
//! unary ops, control flow, ref, unsafe, eval block, spawn.

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::ast::{Expr, UnOp};
use crate::frontend::core::parser::parse_expression;
use crate::frontend::core::lexer::tokens::TokenKind;
use crate::frontend::core::parser::ParserState;

fn parse_expr(source: &str) -> Expr {
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens);
    state
        .parse_expression(crate::frontend::core::parser::BP_LOWEST)
        .expect("parse failed")
}

// ============================================================================
// prefix_info: token → prefix parser routing
// ============================================================================

#[test]
fn test_prefix_info_identifier() {
    let tokens = tokenize("x").unwrap();
    let state = ParserState::new(&tokens);
    let info = state.prefix_info();
    assert!(
        info.is_some(),
        "prefix_info should return Some for identifier"
    );
    let (bp, _) = info.unwrap();
    assert_eq!(bp, crate::frontend::core::parser::BP_HIGHEST);
}

#[test]
fn test_prefix_info_int_literal() {
    let tokens = tokenize("42").unwrap();
    let state = ParserState::new(&tokens);
    let info = state.prefix_info();
    assert!(info.is_some());
}

#[test]
fn test_prefix_info_float_literal() {
    let tokens = tokenize("3.14").unwrap();
    let state = ParserState::new(&tokens);
    let info = state.prefix_info();
    assert!(info.is_some());
}

#[test]
fn test_prefix_info_string_literal() {
    let tokens = tokenize(r#""hello""#).unwrap();
    let state = ParserState::new(&tokens);
    let info = state.prefix_info();
    assert!(info.is_some());
}

#[test]
fn test_prefix_info_bool_literal() {
    let tokens = tokenize("true").unwrap();
    let state = ParserState::new(&tokens);
    let info = state.prefix_info();
    assert!(info.is_some());
}

#[test]
fn test_prefix_info_unary_minus() {
    let tokens = tokenize("-").unwrap();
    let state = ParserState::new(&tokens);
    let info = state.prefix_info();
    assert!(info.is_some());
    let (bp, _) = info.unwrap();
    assert_eq!(bp, crate::frontend::core::parser::BP_UNARY);
}

#[test]
fn test_prefix_info_unary_star() {
    let tokens = tokenize("*").unwrap();
    let state = ParserState::new(&tokens);
    let info = state.prefix_info();
    assert!(info.is_some());
    let (bp, _) = info.unwrap();
    assert_eq!(bp, crate::frontend::core::parser::BP_UNARY);
}

#[test]
fn test_prefix_info_lparen() {
    let tokens = tokenize("(").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_lbracket() {
    let tokens = tokenize("[").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_lbrace() {
    let tokens = tokenize("{").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_if() {
    let tokens = tokenize("if").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_while() {
    let tokens = tokenize("while").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_match() {
    let tokens = tokenize("match").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_for() {
    let tokens = tokenize("for").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_ref() {
    let tokens = tokenize("ref").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_unsafe() {
    let tokens = tokenize("unsafe").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_at() {
    let tokens = tokenize("@").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_spawn() {
    let tokens = tokenize("spawn").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_return() {
    let tokens = tokenize("return").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_break() {
    let tokens = tokenize("break").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_continue() {
    let tokens = tokenize("continue").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_some());
}

#[test]
fn test_prefix_info_eof() {
    let tokens = tokenize("").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.prefix_info().is_none());
}

// ============================================================================
// Prefix parsers: 字面量
// ============================================================================

#[test]
fn test_parse_int_literal() {
    let expr = parse_expr("42");
    assert!(matches!(expr, Expr::Lit(..)));
}

#[test]
fn test_parse_float_literal() {
    let expr = parse_expr("3.14");
    assert!(matches!(expr, Expr::Lit(..)));
}

#[test]
fn test_parse_string_literal() {
    let expr = parse_expr(r#""hello""#);
    assert!(matches!(expr, Expr::Lit(..)));
}

#[test]
fn test_parse_bool_true() {
    let expr = parse_expr("true");
    assert!(matches!(expr, Expr::Lit(..)));
}

#[test]
fn test_parse_wildcard() {
    let expr = parse_expr("_");
    assert!(matches!(expr, Expr::Var(..)));
}

// ============================================================================
// Prefix parsers: 一元运算符
// ============================================================================

#[test]
fn test_parse_unary() {
    let expr = parse_expr("-x");
    assert!(matches!(expr, Expr::UnOp { op: UnOp::Neg, .. }));
}

// ============================================================================
// Prefix parsers: 控制流
// ============================================================================

#[test]
fn test_parse_if_expr() {
    let expr = parse_expr("if true { 1 } else { 2 }");
    assert!(matches!(expr, Expr::If { .. }));
}

#[test]
fn test_parse_while_expr() {
    let expr = parse_expr("while true { }");
    assert!(matches!(expr, Expr::While { .. }));
}

#[test]
fn test_parse_for_expr() {
    let expr = parse_expr("for x in items { }");
    assert!(matches!(expr, Expr::For { .. }));
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
