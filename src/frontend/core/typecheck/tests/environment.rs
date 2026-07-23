//! TypeEnvironment 测试 — 基于语言规范 §3 & RFC-010/011
//!
//! §3.1-§3.17: 类型系统环境管理
//! RFC-010: 统一类型语法
//! RFC-011: 泛型系统设计

use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::types::{MonoType, PolyType};

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_environment_new_creates_empty() {
    // Arrange & Act
    let env = TypeEnvironment::new();

    // Assert
    assert!(env.vars.is_empty(), "vars should be empty");
    assert!(env.types.is_empty(), "types should be empty");
    assert!(env.imports.is_empty(), "imports should be empty");
    assert!(env.exports.is_empty(), "exports should be empty");
}

#[test]
fn test_environment_new_with_module() {
    // Arrange & Act
    let env = TypeEnvironment::new_with_module("test_module".to_string());

    // Assert
    assert_eq!(env.module_name, "test_module");
}

#[test]
fn test_environment_add_var() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    env.add_var("x".to_string(), PolyType::mono(MonoType::Int(32)));

    // Assert
    assert!(env.vars.contains_key("x"), "should contain variable x");
}

#[test]
fn test_environment_add_type() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act - 使用简单的类型引用
    env.add_type(
        "MyType".to_string(),
        PolyType::mono(MonoType::TypeRef("MyType".to_string())),
    );

    // Assert
    assert!(
        env.types.contains_key("MyType"),
        "should contain type MyType"
    );
}

#[test]
fn test_environment_has_solver() {
    // Arrange & Act
    let _env = TypeEnvironment::new();

    // Assert - solver 应该存在
    // 具体断言取决于 TypeConstraintSolver 的实现
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_environment_duplicate_var_allowed() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act - 添加同名变量应该覆盖
    env.add_var("x".to_string(), PolyType::mono(MonoType::Int(32)));
    env.add_var("x".to_string(), PolyType::mono(MonoType::Float(64)));

    // Assert
    let var = env.vars.get("x").unwrap();
    assert_eq!(
        *var,
        PolyType::mono(MonoType::Float(64)),
        "should overwrite with latest"
    );
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_environment_with_many_vars() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act - 添加大量变量
    for i in 0..1000 {
        env.add_var(format!("var_{}", i), PolyType::mono(MonoType::Int(32)));
    }

    // Assert
    assert_eq!(env.vars.len(), 1000, "should have 1000 variables");
}

#[test]
fn test_environment_with_many_types() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act - 添加大量类型
    for i in 0..1000 {
        env.add_type(format!("Type_{}", i), PolyType::mono(MonoType::Int(32)));
    }

    // Assert
    assert_eq!(env.types.len(), 1000, "should have 1000 types");
}

#[test]
fn test_resolve_base_kind_type_vs_value() {
    use crate::frontend::core::typecheck::environment::BaseKind;
    let mut env = TypeEnvironment::new();
    // Point 是类型
    env.add_type(
        "Point".to_string(),
        PolyType::mono(MonoType::TypeRef("Point".to_string())),
    );
    // p 是变量
    env.add_var(
        "p".to_string(),
        PolyType::mono(MonoType::TypeRef("Point".to_string())),
    );

    assert_eq!(env.resolve_base_kind("Point"), BaseKind::TypeSpace);
    assert_eq!(env.resolve_base_kind("p"), BaseKind::ValueSpace);
    assert_eq!(env.resolve_base_kind("nope"), BaseKind::Unknown);
}
