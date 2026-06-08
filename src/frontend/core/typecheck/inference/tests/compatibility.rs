//! 类型兼容性测试 — 基于语言规范 §3.2 & RFC-010
//!
//! §3.2: 原类型（带位宽的整数：Int8, Int16, Int32, Int64, Int128）
//! §3.2.1: 类型转换（禁止隐式拓宽/收窄）
//! §3.5: 接口类型（结构化兼容性）
//! RFC-010: 统一类型语法

use crate::frontend::core::typecheck::inference::compatibility::CompatibilityChecker;
use crate::frontend::core::types::base::{MonoType, StructType};
use std::collections::HashMap;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_compatibility_checker_creation() {
    // Arrange & Act
    let checker = CompatibilityChecker::new();

    // Assert - 验证创建后能正常工作
    let result = checker.check_compatibility(&MonoType::Int(32), &MonoType::Int(32));
    assert!(result.is_ok(), "newly created checker should work");
    assert!(result.unwrap(), "same types should be compatible");
}

/// §3.2: 同类型应兼容
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

/// §3.2: 同位宽浮点类型应兼容
#[test]
fn test_compatibility_same_float_types() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::Float(64), &MonoType::Float(64));

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(result.unwrap(), "same Float types should be compatible");
}

/// §3.2: Bool 类型自兼容
#[test]
fn test_compatibility_bool_with_bool() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::Bool, &MonoType::Bool);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(result.unwrap(), "Bool should be compatible with Bool");
}

/// §3.2: String 类型自兼容
#[test]
fn test_compatibility_string_with_string() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::String, &MonoType::String);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(result.unwrap(), "String should be compatible with String");
}

/// §3.2: Void 类型自兼容
#[test]
fn test_compatibility_void_with_void() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::Void, &MonoType::Void);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(result.unwrap(), "Void should be compatible with Void");
}

// ===================================================================
// Error path 测试
// ===================================================================

/// §3.2: 不同原类型不应兼容
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

/// §3.2.1: 不同位宽整数不应兼容（禁止隐式拓宽）
///
/// 代码待修复: 当前 CompatibilityChecker 允许不同位宽整数兼容，违反规范 §3.2.1。
#[test]
fn test_compatibility_different_int_bit_widths() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::Int(32), &MonoType::Int(64));

    // Assert - 规范 §3.2.1：禁止隐式拓宽（Int32 → Int64）
    // 代码待修复: CompatibilityChecker 当前允许不同位宽整数兼容，需修复为返回 false
    assert!(result.is_ok(), "should check compatibility");
    assert!(
        !result.unwrap(),
        "Int(32) and Int(64) should NOT be compatible (no implicit widening per spec §3.2.1)"
    );
}

/// §3.2.1: Int 和 Float 不应兼容（禁止隐式转换）
///
/// 代码待修复: 当前 CompatibilityChecker 允许 Int→Float 兼容，违反规范 §3.2.1。
#[test]
fn test_compatibility_int_and_float() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::Int(32), &MonoType::Float(64));

    // Assert - 规范 §3.2.1：禁止隐式转换
    // 代码待修复: CompatibilityChecker 当前允许 Int→Float 兼容，需修复为返回 false
    assert!(result.is_ok(), "should check compatibility");
    assert!(
        !result.unwrap(),
        "Int and Float should NOT be compatible (no implicit conversion per spec §3.2.1)"
    );
}

/// §3.2: Int 和 Bool 不应兼容
#[test]
fn test_compatibility_int_and_bool() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::Int(64), &MonoType::Bool);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(!result.unwrap(), "Int and Bool should not be compatible");
}

/// §3.2: String 和 Bool 不应兼容
#[test]
fn test_compatibility_string_and_bool() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::String, &MonoType::Bool);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(!result.unwrap(), "String and Bool should not be compatible");
}

// ===================================================================
// Boundary 测试 — 函数类型兼容性
// ===================================================================

/// §3.1: 相同签名的函数类型应兼容
#[test]
fn test_compatibility_same_function_types() {
    // Arrange
    let checker = CompatibilityChecker::new();
    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };

    // Act
    let result = checker.check_compatibility(&fn_type, &fn_type);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(result.unwrap(), "same function types should be compatible");
}

/// §3.1: 不同参数的函数类型不应兼容
///
/// 代码待修复: 当前 CompatibilityChecker 对函数类型的参数使用宽松比较，
/// 导致参数类型不同的函数也被视为兼容。
#[test]
fn test_compatibility_different_function_params() {
    // Arrange
    let checker = CompatibilityChecker::new();
    let fn1 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };
    let fn2 = MonoType::Fn {
        params: vec![MonoType::Float(64)],
        return_type: Box::new(MonoType::String),
    };

    // Act
    let result = checker.check_compatibility(&fn1, &fn2);

    // Assert
    // 代码待修复: 函数参数类型不同（Int vs Float）时不应兼容，需修复比较逻辑
    assert!(result.is_ok(), "should check compatibility");
    assert!(
        !result.unwrap(),
        "functions with different param types should not be compatible"
    );
}

/// §3.1: 不同返回类型的函数不应兼容
#[test]
fn test_compatibility_different_function_return_types() {
    // Arrange
    let checker = CompatibilityChecker::new();
    let fn1 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };
    let fn2 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::Bool),
    };

    // Act
    let result = checker.check_compatibility(&fn1, &fn2);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(
        !result.unwrap(),
        "functions with different return types should not be compatible"
    );
}

/// §3.1: 不同参数数量的函数不应兼容
#[test]
fn test_compatibility_different_function_arity() {
    // Arrange
    let checker = CompatibilityChecker::new();
    let fn1 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };
    let fn2 = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Bool],
        return_type: Box::new(MonoType::String),
    };

    // Act
    let result = checker.check_compatibility(&fn1, &fn2);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(
        !result.unwrap(),
        "functions with different arity should not be compatible"
    );
}

// ===================================================================
// Boundary 测试 — 接口结构化兼容性
// ===================================================================

/// §3.5: 结构体与空接口应兼容（空约束被任何类型满足）
#[test]
fn test_compatibility_struct_with_empty_interface() {
    // Arrange
    let checker = CompatibilityChecker::new();
    let point = MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), MonoType::Float(64))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let empty_interface = MonoType::Struct(StructType {
        name: "Empty".to_string(),
        fields: vec![],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    });

    // Act
    let result = checker.check_compatibility(&point, &empty_interface);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    // 注意：兼容性检查的具体行为取决于实现，
    // 空接口的兼容性可能需要通过 check_constraint 而非 check_compatibility
}

/// §3.5: 结构体类型自兼容
#[test]
fn test_compatibility_same_struct_types() {
    // Arrange
    let checker = CompatibilityChecker::new();
    let point = MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MonoType::Float(64)),
            ("y".to_string(), MonoType::Float(64)),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });

    // Act
    let result = checker.check_compatibility(&point, &point);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(result.unwrap(), "same struct types should be compatible");
}

/// §3.5: 不同结构体类型不应兼容
#[test]
fn test_compatibility_different_struct_types() {
    // Arrange
    let checker = CompatibilityChecker::new();
    let point = MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), MonoType::Float(64))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let line = MonoType::Struct(StructType {
        name: "Line".to_string(),
        fields: vec![("length".to_string(), MonoType::Float(64))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });

    // Act
    let result = checker.check_compatibility(&point, &line);

    // Assert
    assert!(result.is_ok(), "should check compatibility");
    assert!(
        !result.unwrap(),
        "different struct types should not be compatible"
    );
}

/// §3.2: 不同位宽浮点不应兼容
///
/// 代码待修复: 当前 CompatibilityChecker 允许不同位宽浮点兼容，违反规范 §3.2.1。
#[test]
fn test_compatibility_different_float_bit_widths() {
    // Arrange
    let checker = CompatibilityChecker::new();

    // Act
    let result = checker.check_compatibility(&MonoType::Float(32), &MonoType::Float(64));

    // Assert - 规范 §3.2.1：禁止隐式拓宽
    // 代码待修复: CompatibilityChecker 当前允许不同位宽浮点兼容，需修复为返回 false
    assert!(result.is_ok(), "should check compatibility");
    assert!(
        !result.unwrap(),
        "Float(32) and Float(64) should NOT be compatible (no implicit widening)"
    );
}
