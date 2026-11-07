//! 作用域测试 — 基于语言规范 §5.2
//!
//! §5.2: 变量声明

use crate::frontend::core::sema::inference::scope::ScopeManager;
use crate::frontend::core::types::{MonoType, PolyType};

// Happy path 测试

#[test]
fn test_scope_manager_creation() {
    // Arrange & Act
    let _scope = ScopeManager::new();

    // Assert - 应该成功创建
}

#[test]
fn test_scope_manager_add_var() {
    // Arrange
    let mut scope = ScopeManager::new();

    // Act
    scope.add_var(
        "x".to_string(),
        PolyType::mono(MonoType::Int(32)),
        false,
        crate::util::span::Span::default(),
    );

    // Assert
    let var = scope.get_var("x");
    assert!(var.is_some(), "should contain variable x");
}

#[test]
fn test_scope_manager_get_var() {
    // Arrange
    let mut scope = ScopeManager::new();
    scope.add_var(
        "x".to_string(),
        PolyType::mono(MonoType::Int(32)),
        false,
        crate::util::span::Span::default(),
    );

    // Act
    let var = scope.get_var("x");

    // Assert
    assert!(var.is_some(), "should get variable x");
    assert_eq!(*var.unwrap(), PolyType::mono(MonoType::Int(32)));
}

// Error path 测试

#[test]
fn test_scope_manager_get_undefined_var() {
    // Arrange
    let scope = ScopeManager::new();

    // Act
    let var = scope.get_var("undefined");

    // Assert
    assert!(var.is_none(), "should return None for undefined variable");
}

#[test]
fn test_var_is_mutable_returns_none_for_undefined() {
    // Arrange
    let scope = ScopeManager::new();

    // Act
    let result = scope.var_is_mutable("undefined");

    // Assert
    assert!(
        result.is_none(),
        "undefined variable should return None for var_is_mutable"
    );
}

#[test]
fn test_vars_with_mut_preserves_correct_info() {
    // Arrange
    let mut scope = ScopeManager::new();
    // 三链模型（#295）：局部变量需在函数/块层内 add（否则进 globals，vars_with_mut 不返回）
    scope.enter_block();
    scope.add_var(
        "a".to_string(),
        PolyType::mono(MonoType::Int(32)),
        true,
        crate::util::span::Span::default(),
    );
    scope.add_var(
        "b".to_string(),
        PolyType::mono(MonoType::Bool),
        false,
        crate::util::span::Span::default(),
    );

    // Act
    let vars = scope.vars_with_mut();

    // Assert
    assert!(
        vars["a"].is_mut,
        "mutable variable should be marked as mutable"
    );
    assert!(
        !vars["b"].is_mut,
        "immutable variable should be marked as not mutable"
    );
}
