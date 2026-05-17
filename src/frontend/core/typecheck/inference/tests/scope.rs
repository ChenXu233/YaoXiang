//! 作用域测试 — 基于语言规范 §5.2
//!
//! §5.2: 变量声明

use crate::frontend::core::typecheck::inference::scope::ScopeManager;
use crate::frontend::core::types::base::{MonoType, PolyType};

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_scope_manager_creation() {
    // Arrange & Act
    let scope = ScopeManager::new();
    let _scope = ScopeManager::new();

    // Assert - 应该成功创建
}

#[test]
fn test_scope_manager_add_var() {
    // Arrange
    let mut scope = ScopeManager::new();

    // Act
    scope.add_var("x".to_string(), PolyType::mono(MonoType::Int(32)));

    // Assert
    let var = scope.get_var("x");
    assert!(var.is_some(), "should contain variable x");
}

#[test]
fn test_scope_manager_get_var() {
    // Arrange
    let mut scope = ScopeManager::new();
    scope.add_var("x".to_string(), PolyType::mono(MonoType::Int(32)));

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
        scope.add_var(format!("var_{}", i), PolyType::mono(MonoType::Int(32)));
    }

    // Assert
    for i in 0..1000 {
        let var = scope.get_var(&format!("var_{}", i));
        assert!(var.is_some(), "should contain variable var_{}", i);
    }
}
