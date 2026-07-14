//! 类型边界测试 — 基于语言规范 §3.9 & RFC-011 §2
//!
//! §3.9: 类型约束
//! §3.5.2: 标准库接口（Clone, Equal, Debug）
//! RFC-011 §2: 类型约束系统
//!
//! RFC-011 §4: 编译期泛型 — const 泛型参数类型验证
//! RFC-027 §4: 编译期证明管道 — const 参数通过证明管道验证

use crate::frontend::core::typecheck::inference::bounds::BoundsChecker;
use crate::frontend::core::types::{MonoType, StructType, TraitTable};
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::types::const_data::ConstVarDef;
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use std::collections::HashMap;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_bounds_checker_creation() {
    // Arrange & Act
    let checker = BoundsChecker::new();

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
    let checker = BoundsChecker::new();
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
    let checker = BoundsChecker::new();
    let ty = MonoType::Int(32);
    let bounds: Vec<String> = vec![];

    // Act
    let result = checker.check_trait_bounds(&ty, &bounds);

    // Assert - 空边界应该通过
    assert!(result.is_ok(), "empty bounds should pass");
}

/// §3.5.2: 泛型参数边界检查 — Int 满足 Clone + 空 const 边界
#[test]
fn test_check_const_bounds_empty() {
    // Arrange
    let checker = BoundsChecker::new();
    let const_binders: Vec<ConstVarDef> = vec![];
    let const_args: Vec<MonoType> = vec![];

    // Act
    let result = checker.check_const_bounds(&const_binders, &const_args);

    // Assert
    assert!(
        matches!(result, ProofResult::Proved),
        "Empty const args should trivially pass const bounds"
    );
}

/// §3.5: 空约束（空接口）应被任何类型满足
#[test]
fn test_check_constraint_empty_constraint() {
    // Arrange
    let checker = BoundsChecker::new();
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
    let checker = BoundsChecker::new();
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
    let checker = BoundsChecker::new();
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
    let checker = BoundsChecker::new();
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
    let checker = BoundsChecker::new();
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

/// 结构体所有字段均为 Dup 类型时，TraitTable 应通过 Dup 检查
#[test]
fn test_check_trait_bounds_dup_struct_auto_derive() {
    // Arrange - View { name: String, ref_field: &Int } 所有字段均为 Dup
    let trait_table = TraitTable::with_std();

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

    // Act & Assert
    assert!(
        trait_table.satisfies("Dup", &struct_type),
        "View with all-Dup fields (String + &T) should pass Dup check"
    );
}

/// 结构体包含非 Dup 字段（List）时，TraitTable 应拒绝 Dup 检查
#[test]
fn test_check_trait_bounds_dup_struct_auto_derive_fails() {
    // Arrange - Buffer { data: List(Int(64)), len: Int(64) } 包含非 Dup 的 List 字段
    let trait_table = TraitTable::with_std();

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

    // Act & Assert
    assert!(
        !trait_table.satisfies("Dup", &struct_type),
        "Buffer with List field should NOT pass Dup check"
    );
}

/// 端到端测试：BoundsChecker::check_trait_bounds 对全 Dup 字段结构体应通过
/// 涵盖 check_trait -> check_dup_trait 递归路径
#[test]
fn test_bounds_checker_dup_struct_passes() {
    // Arrange
    let checker = BoundsChecker::new();
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
    let checker = BoundsChecker::new();
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
    let checker = BoundsChecker::new();

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

// ===================================================================
// validate_const_args 测试
// ===================================================================

#[test]
fn test_validate_const_args_int_matches() {
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::MonoType;
    use crate::frontend::core::typecheck::inference::bounds::validate_const_args;

    let binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    let args = vec![MonoType::Literal {
        name: "5".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(5),
    }];
    let result = validate_const_args(&binders, &args);
    assert!(result.is_ok(), "Int const arg should match Int binder");
}

#[test]
fn test_validate_const_args_type_mismatch() {
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::MonoType;
    use crate::frontend::core::typecheck::inference::bounds::validate_const_args;

    let binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    let args = vec![MonoType::Literal {
        name: "true".to_string(),
        base_type: Box::new(MonoType::Bool),
        value: ConstValue::Bool(true),
    }];
    let result = validate_const_args(&binders, &args);
    assert!(
        result.is_err(),
        "Bool const arg should NOT match Int binder"
    );
}

#[test]
fn test_validate_const_args_not_literal() {
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef};
    use crate::frontend::core::types::MonoType;
    use crate::frontend::core::typecheck::inference::bounds::validate_const_args;

    let binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    let args = vec![MonoType::Int(64)];
    let result = validate_const_args(&binders, &args);
    assert!(result.is_err(), "Non-literal type should fail validation");
}

#[test]
fn test_validate_const_args_empty() {
    use crate::frontend::core::types::ConstVarDef;
    use crate::frontend::core::types::MonoType;
    use crate::frontend::core::typecheck::inference::bounds::validate_const_args;

    let binders: Vec<ConstVarDef> = vec![];
    let args: Vec<MonoType> = vec![];
    let result = validate_const_args(&binders, &args);
    assert!(result.is_ok(), "Empty const args should pass");
}

#[test]
fn test_validate_const_args_bool_matches() {
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::MonoType;
    use crate::frontend::core::typecheck::inference::bounds::validate_const_args;

    let binders = vec![ConstVarDef::new("FLAG".to_string(), ConstKind::Bool, 0)];
    let args = vec![MonoType::Literal {
        name: "true".to_string(),
        base_type: Box::new(MonoType::Bool),
        value: ConstValue::Bool(true),
    }];
    let result = validate_const_args(&binders, &args);
    assert!(result.is_ok(), "Bool const arg should match Bool binder");
}
#[test]
fn test_check_const_bounds_fast_path_proved() {
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::MonoType;

    let checker = BoundsChecker::new();
    let binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    let args = vec![MonoType::Literal {
        name: "5".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(5),
    }];
    let result = checker.check_const_bounds(&binders, &args);
    assert!(
        result.is_proved(),
        "Int const arg should pass fast path and return Proved"
    );
}

#[test]
fn test_check_const_bounds_fast_path_disproved() {
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::MonoType;

    let checker = BoundsChecker::new();
    let binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    let args = vec![MonoType::Literal {
        name: "true".to_string(),
        base_type: Box::new(MonoType::Bool),
        value: ConstValue::Bool(true),
    }];
    let result = checker.check_const_bounds(&binders, &args);
    assert!(
        !result.is_proved(),
        "Bool arg should fail Int binder and return Disproved"
    );
}

#[test]
fn test_check_const_bounds_empty_proved() {
    use crate::frontend::core::types::ConstVarDef;
    use crate::frontend::core::types::MonoType;

    let checker = BoundsChecker::new();
    let binders: Vec<ConstVarDef> = vec![];
    let args: Vec<MonoType> = vec![];
    let result = checker.check_const_bounds(&binders, &args);
    assert!(
        result.is_proved(),
        "Empty const args should pass and return Proved"
    );
}

#[test]
fn test_check_const_bounds_layer2_constraint_satisfied() {
    use crate::frontend::core::typecheck::inference::bounds::BoundsChecker;
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::eval::const_eval::ConstExpr;
    use crate::frontend::core::types::MonoType;

    let checker = BoundsChecker::new();
    let mut binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    // 添加真实约束: Bool(true) — 总是满足
    binders[0].constraints.push(ConstExpr::Bool(true));
    let args = vec![MonoType::Literal {
        name: "5".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(5),
    }];
    let result = checker.check_const_bounds(&binders, &args);
    assert!(
        result.is_proved(),
        "Bool(true) constraint should be satisfied and return Proved"
    );
}

#[test]
fn test_check_const_bounds_layer2_constraint_violated() {
    use crate::frontend::core::typecheck::inference::bounds::BoundsChecker;
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::eval::const_eval::ConstExpr;
    use crate::frontend::core::types::MonoType;

    let checker = BoundsChecker::new();
    let mut binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    // Bool(false) — 约束违反
    binders[0].constraints.push(ConstExpr::Bool(false));
    let args = vec![MonoType::Literal {
        name: "5".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(5),
    }];
    let result = checker.check_const_bounds(&binders, &args);
    assert!(
        !result.is_proved(),
        "Bool(false) constraint should be violated and return Disproved"
    );
}

// ===================================================================
// Const 约束 Layer 2 端到端测试
// 基于 docs/superpowers/specs/2026-07-11-const-expr-constraint-design.md
// ===================================================================

#[test]
fn test_check_const_bounds_layer2_real_constraint_satisfied() {
    use crate::frontend::core::typecheck::inference::bounds::BoundsChecker;
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::eval::const_eval::{ConstExpr, ConstBinOp};
    use crate::frontend::core::types::MonoType;

    // Arrange
    let checker = BoundsChecker::new();
    let mut binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    binders[0].constraints.push(ConstExpr::BinOp {
        op: ConstBinOp::Gt,
        lhs: Box::new(ConstExpr::Var("N".to_string())),
        rhs: Box::new(ConstExpr::Int(0)),
    });
    let args = vec![MonoType::Literal {
        name: "5".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(5),
    }];
    // Act
    let result = checker.check_const_bounds(&binders, &args);
    // Assert
    assert!(
        result.is_proved(),
        "N=5 with constraint N > 0 should be Proved"
    );
}

#[test]
fn test_check_const_bounds_layer2_real_constraint_violated() {
    use crate::frontend::core::typecheck::inference::bounds::BoundsChecker;
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::eval::const_eval::{ConstExpr, ConstBinOp};
    use crate::frontend::core::types::MonoType;

    // Arrange
    let checker = BoundsChecker::new();
    let mut binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    binders[0].constraints.push(ConstExpr::BinOp {
        op: ConstBinOp::Gt,
        lhs: Box::new(ConstExpr::Var("N".to_string())),
        rhs: Box::new(ConstExpr::Int(0)),
    });
    let args = vec![MonoType::Literal {
        name: "0".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(0),
    }];
    // Act
    let result = checker.check_const_bounds(&binders, &args);
    // Assert
    assert!(
        !result.is_proved(),
        "N=0 with constraint N > 0 should be Disproved"
    );
}

#[test]
fn test_check_const_bounds_layer2_chain_expr_negative_violated() {
    use crate::frontend::core::typecheck::inference::bounds::BoundsChecker;
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::eval::const_eval::{ConstExpr, ConstBinOp};
    use crate::frontend::core::types::MonoType;

    // Arrange
    let checker = BoundsChecker::new();
    let mut binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    binders[0].constraints.push(ConstExpr::BinOp {
        op: ConstBinOp::Gt,
        lhs: Box::new(ConstExpr::BinOp {
            op: ConstBinOp::Add,
            lhs: Box::new(ConstExpr::Var("N".to_string())),
            rhs: Box::new(ConstExpr::Int(1)),
        }),
        rhs: Box::new(ConstExpr::Int(0)),
    });
    // N = -1: -1 + 1 > 0 → 0 > 0 → false → Disproved
    let args = vec![MonoType::Literal {
        name: "-1".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(-1),
    }];
    // Act
    let result = checker.check_const_bounds(&binders, &args);
    // Assert
    assert!(
        !result.is_proved(),
        "N=-1 with constraint N + 1 > 0 should be Disproved"
    );
}

#[test]
fn test_check_const_bounds_layer2_chain_expr_positive_satisfied() {
    use crate::frontend::core::typecheck::inference::bounds::BoundsChecker;
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::eval::const_eval::{ConstExpr, ConstBinOp};
    use crate::frontend::core::types::MonoType;

    // Arrange
    let checker = BoundsChecker::new();
    let mut binders = vec![ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0)];
    binders[0].constraints.push(ConstExpr::BinOp {
        op: ConstBinOp::Gt,
        lhs: Box::new(ConstExpr::BinOp {
            op: ConstBinOp::Add,
            lhs: Box::new(ConstExpr::Var("N".to_string())),
            rhs: Box::new(ConstExpr::Int(1)),
        }),
        rhs: Box::new(ConstExpr::Int(0)),
    });
    // N = 0: 0 + 1 > 0 → 1 > 0 → true → Proved
    let args = vec![MonoType::Literal {
        name: "0".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(0),
    }];
    // Act
    let result = checker.check_const_bounds(&binders, &args);
    // Assert
    assert!(
        result.is_proved(),
        "N=0 with constraint N + 1 > 0 should be Proved"
    );
}
