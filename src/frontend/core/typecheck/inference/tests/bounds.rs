//! 类型边界测试 — 基于语言规范 §3.9 & RFC-011 §2
//!
//! §3.9: 类型约束
//! §3.5.2: 标准库接口（Clone, Equal, Debug, Send, Sync）
//! RFC-011 §2: 类型约束系统
//!
//! 注意：规范 §B.4 明确不实现生命周期和借用检查器，因此不测试生命周期边界。
//! 规范中没有独立的 "const bounds" 概念，因此不测试 const 边界。

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
    let mut checker = BoundsChecker::new();

    // Assert - 验证创建后内部状态有效
    // 新建的 BoundsChecker 应该能处理基本的边界检查
    let ty = MonoType::Int(32);
    let bounds: Vec<String> = vec![];
    let result = checker.check_trait_bounds(&ty, &bounds);
    assert!(
        result.is_ok(),
        "newly created BoundsChecker should handle empty bounds"
    );
}

/// §3.5.2: 原类型自动实现标准库接口，Int 应满足 Clone 约束
#[test]
fn test_check_trait_bounds_satisfied() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let ty = MonoType::Int(32);
    let bounds = vec!["Clone".to_string()];

    // Act
    let result = checker.check_trait_bounds(&ty, &bounds);

    // Assert - 规范 §3.5.2：原类型自动实现 Clone
    assert!(
        result.is_ok(),
        "Int should satisfy Clone bound per spec §3.5.2"
    );
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

/// §3.5.2: 泛型参数边界检查 — Int 满足 Clone + 空 const 边界
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
    assert!(
        result.is_ok(),
        "Int should satisfy generic bounds with Clone trait"
    );
}

/// §3.5: 空约束（空接口）应被任何类型满足
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

    // Assert - 空约束应该通过（空接口）
    assert!(
        result.is_ok(),
        "empty constraint should be satisfied by any type"
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

/// §3.5.2: Void 不实现 Clone
#[test]
fn test_check_trait_bounds_not_satisfied() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let ty = MonoType::Void;
    let bounds = vec!["Clone".to_string()];

    // Act
    let result = checker.check_trait_bounds(&ty, &bounds);

    // Assert - 规范 §3.5.2：Void 不实现 Clone
    assert!(
        result.is_err(),
        "Void should not satisfy Clone bound per spec §3.5.2"
    );
}

/// §3.5: 接口约束 — 类型缺少约束要求的方法应报错
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

/// §3.5: 接口约束 — 方法签名不兼容应报错
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

/// §3.5 + §10: 通过方法绑定满足接口约束
#[test]
fn test_check_constraint_with_method_binding() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let mut env = TypeEnvironment::new();

    // 添加方法绑定：Point.draw
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
    assert!(
        result.is_ok(),
        "should pass constraint check via method binding"
    );
}
