//! 作用域测试 — 基于语言规范 §5.2
//!
//! §5.2: 变量声明

use crate::frontend::core::typecheck::inference::scope::ScopeManager;
use crate::frontend::core::types::{MonoType, PolyType};

// ===================================================================
// Happy path 测试
// ===================================================================

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

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_scope_manager_get_undefined_var() {
    // Arrange
    let scope = ScopeManager::new();

    // Act
    let var = scope.get_var("undefined");

    // Assert
    assert!(var.is_none(), "should return None for undefined variable");
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_scope_manager_with_many_vars() {
    // Arrange
    let mut scope = ScopeManager::new();

    // Act
    for i in 0..1000 {
        scope.add_var(
            format!("var_{}", i),
            PolyType::mono(MonoType::Int(32)),
            false,
            crate::util::span::Span::default(),
        );
    }

    // Assert
    for i in 0..1000 {
        let var = scope.get_var(&format!("var_{}", i));
        assert!(var.is_some(), "should contain variable var_{}", i);
    }
}

#[test]
fn test_add_var_with_mut_stores_mutability() {
    // Arrange
    let mut scope = ScopeManager::new();

    // Act
    scope.add_var(
        "x".to_string(),
        PolyType::mono(MonoType::Int(32)),
        true,
        crate::util::span::Span::default(),
    );

    // Assert
    assert!(
        scope.var_is_mutable("x").unwrap(),
        "mutable variable should be reported as mutable"
    );
}

#[test]
fn test_add_var_without_mut_stores_immutability() {
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
    assert!(
        !scope.var_is_mutable("x").unwrap(),
        "immutable variable should be reported as not mutable"
    );
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
fn test_var_is_mutable_searches_inner_to_outer() {
    // Arrange
    let mut scope = ScopeManager::new();
    scope.add_var(
        "x".to_string(),
        PolyType::mono(MonoType::Int(32)),
        false,
        crate::util::span::Span::default(),
    );
    scope.enter_scope();
    scope.add_var(
        "x".to_string(),
        PolyType::mono(MonoType::Bool),
        true,
        crate::util::span::Span::default(),
    );

    // Act
    let is_mut = scope.var_is_mutable("x");

    // Assert
    assert!(
        is_mut.unwrap(),
        "inner scope mutable 'x' should shadow outer immutable 'x'"
    );
}

#[test]
fn test_current_scope_vars_only_returns_innermost() {
    // Arrange
    let mut scope = ScopeManager::new();
    scope.add_var(
        "outer".to_string(),
        PolyType::mono(MonoType::Int(32)),
        false,
        crate::util::span::Span::default(),
    );
    scope.enter_scope();
    scope.add_var(
        "inner".to_string(),
        PolyType::mono(MonoType::Bool),
        true,
        crate::util::span::Span::default(),
    );

    // Act
    let vars = scope.current_scope_vars();

    // Assert
    assert_eq!(
        vars.len(),
        1,
        "current_scope_vars should only return innermost scope"
    );
    assert!(vars.contains_key("inner"), "should contain inner variable");
    assert!(
        vars["inner"].is_mut,
        "inner variable should preserve is_mut"
    );
}

#[test]
fn test_update_var_preserves_is_mut() {
    // Arrange
    let mut scope = ScopeManager::new();
    scope.add_var(
        "x".to_string(),
        PolyType::mono(MonoType::Int(32)),
        true,
        crate::util::span::Span::default(),
    );

    // Act
    scope.update_var("x", PolyType::mono(MonoType::String));

    // Assert
    let poly = scope.get_var("x").unwrap();
    assert_eq!(*poly, PolyType::mono(MonoType::String));
    assert!(
        scope.var_is_mutable("x").unwrap(),
        "update_var should preserve existing is_mut"
    );
}

#[test]
fn test_vars_with_mut_preserves_correct_info() {
    // Arrange
    let mut scope = ScopeManager::new();
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
