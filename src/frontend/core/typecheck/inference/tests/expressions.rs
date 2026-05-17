//! 表达式推断测试 — 基于语言规范 §4 & RFC-010
//!
//! §4.1-§4.10: 表达式分类
//! RFC-010: 统一类型语法

use crate::frontend::core::typecheck::inference::expressions::ExpressionInferrer;
use crate::frontend::core::typecheck::inference::scope::ScopeManager;
use crate::frontend::core::types::base::{MonoType, TypeConstraintSolver};
use crate::frontend::core::parser::ast::Expr;
use crate::util::span::Span;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_expression_inferrer_creation() {
    // Arrange
    let mut scope = ScopeManager::new();
    let mut solver = TypeConstraintSolver::default();
    let overload_candidates = std::collections::HashMap::new();

    // Act
    let inferrer = ExpressionInferrer::new(&mut scope, &mut solver, &overload_candidates);
    let _inferrer = ExpressionInferrer::new(&mut scope, &mut solver, &overload_candidates);

    // Assert - 应该成功创建
}

#[test]
fn test_infer_integer_literal() {
    // Arrange
    let mut scope = ScopeManager::new();
    let mut solver = TypeConstraintSolver::default();
    let overload_candidates = std::collections::HashMap::new();
    let mut inferrer = ExpressionInferrer::new(&mut scope, &mut solver, &overload_candidates);
    let expr = Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::Int(42),
        Span::dummy(),
    );

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert
    assert!(result.is_ok(), "should infer integer literal");
    assert_eq!(
        result.unwrap(),
        MonoType::Int(64),
        "integer literal should be Int(64)"
    );
}

#[test]
fn test_infer_string_literal() {
    // Arrange
    let mut scope = ScopeManager::new();
    let mut solver = TypeConstraintSolver::default();
    let overload_candidates = std::collections::HashMap::new();
    let mut inferrer = ExpressionInferrer::new(&mut scope, &mut solver, &overload_candidates);
    let expr = Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::String("hello".to_string()),
        Span::dummy(),
    );

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert
    assert!(result.is_ok(), "should infer string literal");
    assert_eq!(
        result.unwrap(),
        MonoType::String,
        "string literal should be String"
    );
}

#[test]
fn test_infer_bool_literal() {
    // Arrange
    let mut scope = ScopeManager::new();
    let mut solver = TypeConstraintSolver::default();
    let overload_candidates = std::collections::HashMap::new();
    let mut inferrer = ExpressionInferrer::new(&mut scope, &mut solver, &overload_candidates);
    let expr = Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::Bool(true),
        Span::dummy(),
    );

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert
    assert!(result.is_ok(), "should infer bool literal");
    assert_eq!(
        result.unwrap(),
        MonoType::Bool,
        "bool literal should be Bool"
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_infer_undefined_variable() {
    // Arrange
    let mut scope = ScopeManager::new();
    let mut solver = TypeConstraintSolver::default();
    let overload_candidates = std::collections::HashMap::new();
    let mut inferrer = ExpressionInferrer::new(&mut scope, &mut solver, &overload_candidates);
    let expr = Expr::Var("undefined_var".to_string(), Span::dummy());

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert
    assert!(result.is_err(), "should fail for undefined variable");
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_infer_nested_expressions() {
    // Arrange
    let mut scope = ScopeManager::new();
    let mut solver = TypeConstraintSolver::default();
    let overload_candidates = std::collections::HashMap::new();
    let mut inferrer = ExpressionInferrer::new(&mut scope, &mut solver, &overload_candidates);
    // 嵌套表达式：(1 + 2) * 3
    let expr = Expr::BinOp {
        op: crate::frontend::core::parser::ast::BinOp::Mul,
        left: Box::new(Expr::BinOp {
            op: crate::frontend::core::parser::ast::BinOp::Add,
            left: Box::new(Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(1),
                Span::dummy(),
            )),
            right: Box::new(Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(2),
                Span::dummy(),
            )),
            span: Span::dummy(),
        }),
        right: Box::new(Expr::Lit(
            crate::frontend::core::lexer::tokens::Literal::Int(3),
            Span::dummy(),
        )),
        span: Span::dummy(),
    };

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert
    assert!(result.is_ok(), "should handle nested expressions");
}
