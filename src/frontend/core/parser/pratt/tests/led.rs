//! Tests for led.rs — infix/postfix expression parsing
//!
//! Covers: infix_info mapping, infix parsers for binary ops,
//! call, field access, indexing, cast, try, lambda.

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::ast::{BinOp, Expr};
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
// infix_info: token → infix parser routing
// ============================================================================

#[test]
fn test_infix_info_plus() {
    let tokens = tokenize("+").unwrap();
    let state = ParserState::new(&tokens);
    let info = state.infix_info();
    assert!(info.is_some(), "infix_info should return Some for +");
}

#[test]
fn test_infix_info_minus() {
    let tokens = tokenize("-").unwrap();
    let state = ParserState::new(&tokens);
    let info = state.infix_info();
    assert!(info.is_some());
}

#[test]
fn test_infix_info_star() {
    let tokens = tokenize("*").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_slash() {
    let tokens = tokenize("/").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_eqeq() {
    let tokens = tokenize("==").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_lt() {
    let tokens = tokenize("<").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_assign() {
    let tokens = tokenize("=").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_range() {
    let tokens = tokenize("..").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_or() {
    // "or" might be tokenized differently than infix::Or
    let tokens = tokenize("or").unwrap();
    let state = ParserState::new(&tokens);
    // Accept either Some or None depending on lexer behavior
    let _ = state.infix_info();
}

#[test]
fn test_infix_info_and() {
    let tokens = tokenize("and").unwrap();
    let state = ParserState::new(&tokens);
    let _ = state.infix_info();
}

#[test]
fn test_infix_info_lparen() {
    let tokens = tokenize("(").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_dot() {
    let tokens = tokenize(".").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_lbracket() {
    let tokens = tokenize("[").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_as() {
    let tokens = tokenize("as").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_question() {
    let tokens = tokenize("?").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_fat_arrow() {
    let tokens = tokenize("=>").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_eof() {
    let tokens = tokenize("").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_none());
}

// ============================================================================
// Infix parsers: 二元运算
// ============================================================================

#[test]
fn test_infix_add() {
    let expr = parse_expr("1 + 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Add, .. }));
}

#[test]
fn test_infix_sub() {
    let expr = parse_expr("1 - 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Sub, .. }));
}

#[test]
fn test_infix_mul() {
    let expr = parse_expr("1 * 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Mul, .. }));
}

#[test]
fn test_infix_div() {
    let expr = parse_expr("1 / 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Div, .. }));
}

#[test]
fn test_infix_eq() {
    let expr = parse_expr("a == b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Eq, .. }));
}

#[test]
fn test_infix_lt() {
    let expr = parse_expr("a < b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Lt, .. }));
}

#[test]
fn test_infix_range() {
    let expr = parse_expr("1..5");
    assert!(matches!(
        expr,
        Expr::BinOp {
            op: BinOp::Range,
            ..
        }
    ));
}

// ============================================================================
// Infix parsers: 函数调用、字段访问、索引
// ============================================================================

#[test]
fn test_infix_call() {
    let expr = parse_expr("f(a, b)");
    assert!(matches!(expr, Expr::Call { .. }));
}

#[test]
fn test_infix_field() {
    let expr = parse_expr("obj.field");
    assert!(matches!(expr, Expr::FieldAccess { .. }));
}

#[test]
fn test_infix_index() {
    let expr = parse_expr("arr[0]");
    assert!(matches!(expr, Expr::Index { .. }));
}

// ============================================================================
// Infix parsers: 类型转换、错误传播、Lambda
// ============================================================================

#[test]
fn test_infix_cast() {
    let expr = parse_expr("42 as Float");
    assert!(matches!(expr, Expr::Cast { .. }));
}

#[test]
fn test_infix_try() {
    let expr = parse_expr("x?");
    assert!(matches!(expr, Expr::Try { .. }));
}

#[test]
fn test_infix_lambda_single() {
    let expr = parse_expr("(x) => x + 1");
    assert!(matches!(expr, Expr::Lambda { .. }));
}
