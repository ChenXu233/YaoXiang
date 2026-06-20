//! Control flow parsing tests — based on spec §5.3–§5.9
//!
//! Note: while/return/break/continue are Expr variants wrapped in StmtKind::Expr.

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::parser::ast::{Expr, StmtKind};

fn parse_stmt(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors);
    assert_eq!(result.module.items.len(), 1);
    result.module.items.into_iter().next().unwrap().kind
}

fn unwrap_expr(kind: &StmtKind) -> &Expr {
    if let StmtKind::Expr(expr) = kind {
        expr.as_ref()
    } else {
        panic!("Expected StmtKind::Expr, got {:?}", kind);
    }
}

// ============================================================================
// if 语句 (Spec §5.6)
// ============================================================================

#[test]
fn test_if_simple() {
    let kind = parse_stmt("if true { }");
    assert!(matches!(&kind, StmtKind::If { .. }));
}

#[test]
fn test_if_else() {
    let kind = parse_stmt("if true { } else { }");
    assert!(matches!(&kind, StmtKind::If { .. }));
}

#[test]
fn test_if_elif_else() {
    let kind = parse_stmt("if a { } elif b { } else { }");
    assert!(matches!(&kind, StmtKind::If { .. }));
}

// ============================================================================
// while 循环 (Spec §5.8)
// ============================================================================

#[test]
fn test_while() {
    let kind = parse_stmt("while cond { }");
    let expr = unwrap_expr(&kind);
    assert!(matches!(expr, Expr::While { .. }));
}

// ============================================================================
// for 循环 (Spec §5.9)
// ============================================================================

#[test]
fn test_for_immutable() {
    let kind = parse_stmt("for x in iter { }");
    // for 可以是 StmtKind::For 或 Expr::For
    let is_stmt_for = matches!(&kind, StmtKind::For { .. });
    let is_expr_for = matches!(&kind, StmtKind::Expr(e) if matches!(e.as_ref(), Expr::For { .. }));
    assert!(is_stmt_for || is_expr_for);
}

#[test]
fn test_for_mutable() {
    let kind = parse_stmt("for mut x in iter { }");
    let is_mut_for = matches!(&kind, StmtKind::For { var_mut, .. } if *var_mut);
    assert!(is_mut_for);
}

// ============================================================================
// return 语句 (Spec §5.3)
// ============================================================================

#[test]
fn test_return_no_value() {
    let kind = parse_stmt("return");
    let expr = unwrap_expr(&kind);
    assert!(matches!(expr, Expr::Return(None, _)));
}

#[test]
fn test_return_with_value() {
    let kind = parse_stmt("return 42");
    let expr = unwrap_expr(&kind);
    assert!(matches!(expr, Expr::Return(Some(..), _)));
}

// ============================================================================
// break / continue (Spec §5.4, §5.5)
// ============================================================================

#[test]
fn test_break() {
    let kind = parse_stmt("break");
    let expr = unwrap_expr(&kind);
    assert!(matches!(expr, Expr::Break(..)));
}

#[test]
fn test_continue() {
    let kind = parse_stmt("continue");
    let expr = unwrap_expr(&kind);
    assert!(matches!(expr, Expr::Continue(..)));
}

// ============================================================================
// 块语句
// ============================================================================

#[test]
fn test_block_expr_as_stmt() {
    let kind = parse_stmt("{ }");
    assert!(matches!(&kind, StmtKind::Expr(..)));
}
