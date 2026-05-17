//! 子类型测试 — 基于语言规范 §3 & RFC-010
//!
//! §3: 类型系统
//! RFC-010: 统一类型语法

use crate::frontend::core::typecheck::inference::subtyping::SubtypeChecker;
use crate::frontend::core::types::base::MonoType;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_subtype_checker_creation() {
    // Arrange & Act
    let checker = SubtypeChecker::new();
    let _checker = SubtypeChecker::new();

    // Assert - 应该成功创建
}

#[test]
fn test_subtype_int_is_subtype_of_int() {
    // Arrange
    let checker = SubtypeChecker::new();

    // Act
    let result = checker.is_subtype(&MonoType::Int(32), &MonoType::Int(32));

    // Assert
    assert!(result, "Int(32) should be subtype of Int(32)");
}

#[test]
fn test_subtype_int_is_subtype_of_float() {
    // Arrange
    let checker = SubtypeChecker::new();

    // Act
    let result = checker.is_subtype(&MonoType::Int(32), &MonoType::Float(64));

    // Assert
    assert!(result, "Int should be subtype of Float");
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_subtype_float_is_not_subtype_of_int() {
    // Arrange
    let checker = SubtypeChecker::new();

    // Act
    let result = checker.is_subtype(&MonoType::Float(64), &MonoType::Int(32));

    // Assert
    assert!(!result, "Float should not be subtype of Int");
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_subtype_with_complex_types() {
    // Arrange
    let checker = SubtypeChecker::new();
    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
        is_async: false,
    };

    // Act
    let result = checker.is_subtype(&fn_type, &fn_type);

    // Assert
    assert!(result, "function type should be subtype of itself");
}
