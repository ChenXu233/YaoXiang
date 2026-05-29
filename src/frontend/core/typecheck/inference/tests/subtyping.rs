//! 子类型测试 — 基于语言规范 §3.2 & RFC-010
//!
//! §3.2: 原类型
//! §3.2.1: 类型转换（禁止隐式拓宽，必须显式转换）
//! RFC-010: 统一类型语法
//!
//! 规范 v1.9.0 明确：禁止隐式拓宽（Int32 → Int64），
//! Int → Float 必须使用 Float(x) 显式转换。
//! 因此 Int 不是 Float 的子类型。

use crate::frontend::core::typecheck::inference::subtyping::SubtypeChecker;
use crate::frontend::core::types::base::MonoType;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_subtype_checker_creation() {
    // Arrange & Act
    let checker = SubtypeChecker::new();

    // Assert - 验证创建后能正常工作
    let result = checker.is_subtype(&MonoType::Int(32), &MonoType::Int(32));
    assert!(result, "newly created SubtypeChecker should work correctly");
}

/// §3.2: 同类型应为子类型关系（自反性）
#[test]
fn test_subtype_int_is_subtype_of_int() {
    // Arrange
    let checker = SubtypeChecker::new();

    // Act
    let result = checker.is_subtype(&MonoType::Int(32), &MonoType::Int(32));

    // Assert
    assert!(result, "Int(32) should be subtype of Int(32) (reflexivity)");
}

/// §3.2.1: Int 不是 Float 的子类型
///
/// 规范 v1.9.0 明确：禁止隐式拓宽，Int → Float 必须使用 Float(x) 显式转换。
/// 如果代码当前允许隐式转换，这是代码 bug，测试按规范断言。
///
/// 代码待修复: 当前 SubtypeChecker 允许 Int→Float 隐式转换，违反规范 §3.2.1。
#[test]
fn test_subtype_int_is_not_subtype_of_float() {
    // Arrange
    let checker = SubtypeChecker::new();

    // Act
    let result = checker.is_subtype(&MonoType::Int(32), &MonoType::Float(64));

    // Assert - 规范 §3.2.1：禁止隐式拓宽，Int 不是 Float 的子类型
    // 代码待修复: SubtypeChecker.is_subtype 当前允许 Int→Float，需修复为返回 false
    assert!(
        !result,
        "Int should NOT be subtype of Float per spec §3.2.1 (no implicit widening)"
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

/// §3.2.1: Float 不是 Int 的子类型（禁止隐式收窄）
#[test]
fn test_subtype_float_is_not_subtype_of_int() {
    // Arrange
    let checker = SubtypeChecker::new();

    // Act
    let result = checker.is_subtype(&MonoType::Float(64), &MonoType::Int(32));

    // Assert
    assert!(
        !result,
        "Float should not be subtype of Int (no implicit narrowing)"
    );
}

// ===================================================================
// Boundary 测试
// ===================================================================

/// §3.1: 函数类型应满足自反性子类型关系
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
    assert!(
        result,
        "function type should be subtype of itself (reflexivity)"
    );
}
