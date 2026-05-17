//! 类型兼容性测试 — 基于语言规范 §3 & RFC-010
//!
//! §3: 类型系统
//! RFC-010: 统一类型语法

use crate::frontend::core::typecheck::inference::compatibility::CompatibilityChecker;
use crate::frontend::core::types::base::MonoType;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_compatibility_checker_creation() {
    // Arrange & Act
    let checker = CompatibilityChecker::new();
    let _checker = CompatibilityChecker::new();

    // Assert - 应该成功创建
}

#[test]
fn test_compatibility_same_types() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::Int(32), &MonoType::Int(32));

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(result.unwrap(), "same types should be compatible");
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_compatibility_different_types() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::Int(32), &MonoType::String);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(!result.unwrap(), "different types should not be compatible");
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_compatibility_with_complex_types() {
    // Arrange
    let checker = CompatibilityChecker::new();
    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
        is_async: false,
    };

    // Act
    let result = checker.check_compatibility(&fn_type, &fn_type);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(result.unwrap(), "same complex types should be compatible");
}
