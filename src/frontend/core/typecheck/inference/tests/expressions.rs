//! 表达式推断测试 — 基于语言规范 §3.2 & §4 & RFC-010
//!
//! §3.2: 原类型（Int 默认 8 字节 = Int(64)）
//! §4.1-§4.10: 表达式分类
//! RFC-010: 统一类型语法

use crate::frontend::core::typecheck::inference::expressions::ExpressionInferrer;
use crate::frontend::core::typecheck::inference::scope::ScopeManager;
use crate::frontend::core::types::{MonoType, TypeConstraintSolver};
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
    let mut inferrer = ExpressionInferrer::new(&mut scope, &mut solver, &overload_candidates);

    // Assert - 验证创建后能正常推断
    let expr = Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::Int(0),
        Span::dummy(),
    );
    let result = inferrer.infer_expr(&expr);
    assert!(
        result.is_ok(),
        "newly created inferrer should handle basic expressions"
    );
}

/// §3.2: 整数字面量默认推断为 Int(64)（8 字节）
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

    // Assert - 规范 §3.2：Int 默认 8 字节 = Int(64)
    assert!(result.is_ok(), "should infer integer literal");
    assert_eq!(
        result.unwrap(),
        MonoType::Int(64),
        "integer literal should be Int(64) per spec §3.2"
    );
}

/// §3.2: 字符串字面量推断为 String
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

/// §3.2: 布尔字面量推断为 Bool
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

/// §4: 未定义变量应报错
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

/// §3.2 & §4: 嵌套表达式 (1 + 2) * 3 应推断为 Int(64)
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

    // Assert - 断言推断出的具体类型为 Int(64)，而非仅检查 is_ok()
    assert!(result.is_ok(), "should handle nested expressions");
    assert_eq!(
        result.unwrap(),
        MonoType::Int(64),
        "nested int arithmetic expression should infer to Int(64)"
    );
}
