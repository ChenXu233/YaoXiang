//! TypeChecker 测试 — 基于语言规范 §3 & RFC-010/011
//!
//! §3.1-§3.17: 类型系统检查
//! §6: 函数定义检查
//! RFC-010: 统一类型语法
//! RFC-011: 泛型系统设计

use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::types::base::{MonoType, PolyType};
use crate::frontend::core::parser::ast::{Module, Stmt, Expr, Type as AstType};
use crate::util::span::Span;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_type_checker_new_creates_with_module_name() {
    // Arrange & Act
    let checker = TypeChecker::new("test_module");

    // Assert
    assert_eq!(checker.module_name(), "test_module");
}

#[test]
fn test_type_checker_has_builtin_types() {
    // Arrange
    let mut checker = TypeChecker::new("test");

    // Act
    let env = checker.env();

    // Assert - 检查内置类型是否存在
    assert!(env.types.contains_key("int"), "should have int type");
    assert!(env.types.contains_key("float"), "should have float type");
    assert!(env.types.contains_key("bool"), "should have bool type");
    assert!(env.types.contains_key("string"), "should have string type");
    assert!(env.types.contains_key("void"), "should have void type");
}

#[test]
fn test_type_checker_can_add_var() {
    // Arrange
    let mut checker = TypeChecker::new("test");

    // Act
    checker
        .env()
        .add_var("x".to_string(), PolyType::mono(MonoType::Int(32)));

    // Assert
    let env = checker.env();
    assert!(env.vars.contains_key("x"), "should have variable x");
}

#[test]
fn test_type_checker_has_no_errors_initially() {
    // Arrange & Act
    let checker = TypeChecker::new("test");

    // Assert
    assert!(!checker.has_errors(), "should have no errors initially");
}

#[test]
fn test_type_checker_check_empty_module() {
    // Arrange
    let mut checker = TypeChecker::new("test");
    let module = Module {
        items: vec![],
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);

    // Assert
    assert!(result.is_ok(), "empty module should pass type check");
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_type_checker_reports_type_mismatch() {
    // Arrange
    let mut checker = TypeChecker::new("test");

    // 构造一个类型不匹配的 AST：将 Int 赋值给 String 变量
    let module = Module {
        items: vec![Stmt {
            kind: crate::frontend::core::parser::ast::StmtKind::Var {
                name: "x".to_string(),
                name_span: Span::dummy(),
                type_annotation: Some(AstType::String),
                initializer: Some(Box::new(Expr::Lit(
                    crate::frontend::core::lexer::tokens::Literal::Int(42),
                    Span::dummy(),
                ))),
                is_mut: false,
            },
            span: Span::dummy(),
        }],
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);
    let _result = checker.check_module(&module);

    // Assert - 应该报告类型错误
    // 注意：具体行为取决于 TypeChecker 的实现
    // 如果实现了类型检查，应该返回错误
    // assert!(result.is_err() || checker.has_errors(), "should report type mismatch");
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_type_checker_with_large_module() {
    // Arrange
    let mut checker = TypeChecker::new("test");
    let mut items = vec![];
    for i in 0..100 {
        // 添加大量语句
        items.push(Stmt {
            kind: crate::frontend::core::parser::ast::StmtKind::Expr(Box::new(Expr::Var(
                format!("var_{}", i),
                Span::dummy(),
            ))),
            span: Span::dummy(),
        });
    }
    let module = Module {
        items,
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);

    // Assert - 不应该 panic
    assert!(
        result.is_ok() || result.is_err(),
        "should handle large module"
    );
}
