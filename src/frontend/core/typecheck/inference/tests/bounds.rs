//! 类型边界测试 — 基于语言规范 §3.9 & RFC-011 §2
//!
//! §3.9: 类型约束
//! RFC-011 §2: 类型约束系统

use crate::frontend::core::typecheck::inference::bounds::BoundsChecker;
use crate::frontend::core::types::base::{MonoType, StructType};
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use std::collections::HashMap;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_bounds_checker_creation() {
    // Arrange & Act
    let checker = BoundsChecker::new();
    let _checker = BoundsChecker::new();

    // Assert - 应该成功创建
}

#[test]
fn test_check_trait_bounds_satisfied() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let ty = MonoType::Int(32);
    let bounds = vec!["Clone".to_string()];

    // Act
    let result = checker.check_trait_bounds(&ty, &bounds);

    // Assert - Int 应该满足 Clone
    assert!(result.is_ok(), "Int should satisfy Clone bound");
}

#[test]
fn test_check_trait_bounds_empty() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let ty = MonoType::Int(32);
    let bounds: Vec<String> = vec![];

    // Act
    let result = checker.check_trait_bounds(&ty, &bounds);

    // Assert - 空边界应该通过
    assert!(result.is_ok(), "empty bounds should pass");
}

#[test]
fn test_check_const_bounds() {
    // Arrange
    let checker = BoundsChecker::new();
    let ty = MonoType::Int(32);
    let bounds: Vec<MonoType> = vec![];

    // Act
    let result = checker.check_const_bounds(&ty, &bounds);

    // Assert
    assert!(result.is_ok(), "const bounds should pass");
}

#[test]
fn test_check_lifetime_bounds() {
    // Arrange
    let checker = BoundsChecker::new();
    let ty = MonoType::Int(32);
    let bounds: Vec<String> = vec![];

    // Act
    let result = checker.check_lifetime_bounds(&ty, &bounds);

    // Assert
    assert!(result.is_ok(), "lifetime bounds should pass");
}

#[test]
fn test_check_generic_bounds() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let ty = MonoType::Int(32);
    let trait_bounds = vec!["Clone".to_string()];
    let const_bounds: Vec<MonoType> = vec![];

    // Act
    let result = checker.check_generic_bounds(&ty, &trait_bounds, &const_bounds);

    // Assert
    assert!(result.is_ok(), "generic bounds should pass");
}

#[test]
fn test_check_constraint_empty_constraint() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let ty = MonoType::Int(32);
    let constraint = MonoType::Struct(StructType {
        name: "Empty".to_string(),
        fields: vec![],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    });

    // Act
    let result = checker.check_constraint(&ty, &constraint, None);

    // Assert - 空约束应该通过
    assert!(result.is_ok(), "empty constraint should pass");
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_check_trait_bounds_not_satisfied() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let ty = MonoType::Void;
    let bounds = vec!["Clone".to_string()];

    // Act
    let result = checker.check_trait_bounds(&ty, &bounds);

    // Assert - Void 不应该满足 Clone
    assert!(result.is_err(), "Void should not satisfy Clone bound");
}

#[test]
fn test_check_constraint_missing_method() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let ty = MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), MonoType::Float(64))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let constraint = MonoType::Struct(StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    });

    // Act
    let result = checker.check_constraint(&ty, &constraint, None);

    // Assert - 应该报告缺少方法
    assert!(result.is_err(), "should report missing method");
}

#[test]
fn test_check_constraint_signature_mismatch() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let ty = MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::Int(32)],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let constraint = MonoType::Struct(StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    });

    // Act
    let result = checker.check_constraint(&ty, &constraint, None);

    // Assert - 应该报告签名不匹配
    assert!(result.is_err(), "should report signature mismatch");
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_check_constraint_with_method_binding() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let mut env = TypeEnvironment::new();

    // 添加方法绑定
    env.method_bindings.insert(
        "Point.draw".to_string(),
        MonoType::Fn {
            params: vec![
                MonoType::TypeRef("Point".to_string()),
                MonoType::TypeRef("Surface".to_string()),
            ],
            return_type: Box::new(MonoType::Void),
            is_async: false,
        },
    );

    let ty = MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), MonoType::Float(64))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let constraint = MonoType::Struct(StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    });

    // Act
    let result = checker.check_constraint(&ty, &constraint, Some(&env));

    // Assert - 应该通过（通过方法绑定找到）
    assert!(result.is_ok(), "should pass with method binding");
}
