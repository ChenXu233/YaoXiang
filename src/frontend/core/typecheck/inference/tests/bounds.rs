//! 类型边界测试 — 基于语言规范 §3.9 & RFC-011 §2
//!
//! §3.9: 类型约束
//! §3.5.2: 标准库接口（Clone, Equal, Debug）
//! RFC-011 §2: 类型约束系统
//!
//! 注意：规范 §B.4 明确不实现生命周期和借用检查器，因此不测试生命周期边界。
//! 规范中没有独立的 "const bounds" 概念，因此不测试 const 边界。

use crate::frontend::core::typecheck::inference::bounds::BoundsChecker;
use crate::frontend::core::types::base::{MonoType, StructType};
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::typecheck::traits::solver::TraitSolver;
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

/// List 类型不实现 Clone（泛型容器需显式实现）
#[test]
fn test_check_trait_bounds_not_satisfied() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let ty = MonoType::List(Box::new(MonoType::Int(32)));
    let bounds = vec!["Clone".to_string()];

    // Act
    let result = checker.check_trait_bounds(&ty, &bounds);

    // Assert - List 类型不自动实现 Clone
    assert!(result.is_err(), "List should not satisfy Clone bound");
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

// ===================================================================
// Dup trait bound with auto-derive tests
// ===================================================================

/// 结构体所有字段均为 Dup 类型时，TraitSolver 应通过 Dup 检查
/// （通过 check_dup_trait 递归检查结构体字段）
#[test]
fn test_check_trait_bounds_dup_struct_auto_derive() {
    // Arrange - View { name: String, ref_field: &Int } 所有字段均为 Dup
    let mut solver = TraitSolver::new();

    let struct_type = MonoType::Struct(StructType {
        name: "View".to_string(),
        fields: vec![
            ("name".to_string(), MonoType::String),
            (
                "ref_field".to_string(),
                MonoType::Ref {
                    mutable: false,
                    inner: Box::new(MonoType::Int(64)),
                },
            ),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });

    // Act & Assert - 无 trait_table 时走 check_dup_trait 递归路径
    assert!(
        solver.check_trait(&struct_type, "Dup"),
        "View with all-Dup fields (String + &T) should pass Dup check"
    );
}

/// 结构体包含非 Dup 字段（List）时，TraitSolver 应拒绝 Dup 检查
/// List 类型在 check_dup_trait 中匹配 `_ => false` 分支
#[test]
fn test_check_trait_bounds_dup_struct_auto_derive_fails() {
    // Arrange - Buffer { data: List(Int(64)), len: Int(64) } 包含非 Dup 的 List 字段
    let mut solver = TraitSolver::new();

    let struct_type = MonoType::Struct(StructType {
        name: "Buffer".to_string(),
        fields: vec![
            (
                "data".to_string(),
                MonoType::List(Box::new(MonoType::Int(64))),
            ),
            ("len".to_string(), MonoType::Int(64)),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });

    // Act & Assert - List 在 check_dup_trait 中不被识别为 Dup
    assert!(
        !solver.check_trait(&struct_type, "Dup"),
        "Buffer with List field should NOT pass Dup check"
    );
}

/// 端到端测试：BoundsChecker::check_trait_bounds 对全 Dup 字段结构体应通过
/// 涵盖 check_trait -> check_dup_trait 递归路径
#[test]
fn test_bounds_checker_dup_struct_passes() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let struct_type = MonoType::Struct(StructType {
        name: "View".to_string(),
        fields: vec![
            ("name".to_string(), MonoType::String),
            (
                "ref_field".to_string(),
                MonoType::Ref {
                    mutable: false,
                    inner: Box::new(MonoType::Int(64)),
                },
            ),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });
    let bounds = vec!["Dup".to_string()];

    // Act
    let result = checker.check_trait_bounds(&struct_type, &bounds);

    // Assert - 所有字段均为 Dup 类型（String + &T），应通过
    assert!(
        result.is_ok(),
        "BoundsChecker should accept struct with all Dup fields"
    );
}

/// 端到端测试：BoundsChecker::check_trait_bounds 对含非 Dup 字段的结构体应失败
/// check_dup_trait 返回 false，且无 trait_table 时无法走 auto-derive 路径
#[test]
fn test_bounds_checker_dup_struct_fails() {
    // Arrange
    let mut checker = BoundsChecker::new();
    let struct_type = MonoType::Struct(StructType {
        name: "Buffer".to_string(),
        fields: vec![
            (
                "data".to_string(),
                MonoType::List(Box::new(MonoType::Int(64))),
            ),
            ("len".to_string(), MonoType::Int(64)),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });
    let bounds = vec!["Dup".to_string()];

    // Act
    let result = checker.check_trait_bounds(&struct_type, &bounds);

    // Assert - List 字段导致 Dup 检查失败
    assert!(
        result.is_err(),
        "BoundsChecker should reject Buffer with List field for Dup bound"
    );
}

/// 嵌套结构体 Dup 检查：外层结构体字段为另一个全 Dup 结构体时应通过
#[test]
fn test_bounds_checker_dup_nested_struct_passes() {
    // Arrange - Connection { from: View, to: View } 嵌套 Dup 结构体
    let mut checker = BoundsChecker::new();

    let view_type = MonoType::Struct(StructType {
        name: "View".to_string(),
        fields: vec![
            ("name".to_string(), MonoType::String),
            (
                "ref_field".to_string(),
                MonoType::Ref {
                    mutable: false,
                    inner: Box::new(MonoType::Int(64)),
                },
            ),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });

    let conn_type = MonoType::Struct(StructType {
        name: "Connection".to_string(),
        fields: vec![
            ("from".to_string(), view_type.clone()),
            ("to".to_string(), view_type),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });
    let bounds = vec!["Dup".to_string()];

    // Act
    let result = checker.check_trait_bounds(&conn_type, &bounds);

    // Assert - 嵌套 Dup 结构体应通过递归检查
    assert!(
        result.is_ok(),
        "BoundsChecker should accept nested Dup structs"
    );
}
