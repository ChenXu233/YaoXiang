//! 类型边界测试 — 基于语言规范 §3.9 & RFC-011 §2
//!
//! §3.9: 类型约束
//! §3.5.2: 标准库接口（Clone, Equal, Debug）
//! RFC-011 §2: 类型约束系统
//!
//! RFC-011 §4: 编译期泛型 — const 泛型参数类型验证
//! RFC-027 §4: 编译期证明管道 — const 参数通过证明管道验证

use crate::frontend::core::sema::inference::bounds::BoundsChecker;
use crate::frontend::core::types::{MonoType, StructType, TraitTable};
use crate::frontend::core::sema::proof::verdict::ProofResult;
use crate::frontend::core::types::const_data::ConstVarDef;
use std::collections::HashMap;

/// 记录类型夹具：`methods`/`interfaces` 为空，两个元数据向量原样传入（沿用各用例既有形态）。
fn record(
    name: &str,
    fields: Vec<(&str, MonoType)>,
    field_mutability: Vec<bool>,
    field_has_default: Vec<bool>,
) -> MonoType {
    MonoType::Struct(StructType {
        name: name.to_string(),
        fields: fields
            .into_iter()
            .map(|(n, t)| (n.to_string(), t))
            .collect(),
        methods: HashMap::new(),
        field_mutability,
        field_has_default,
        interfaces: vec![],
    })
}

/// 函数类型 `(params) -> ret`。
fn mono_fn(
    params: Vec<MonoType>,
    ret: MonoType,
) -> MonoType {
    MonoType::Fn {
        params,
        return_type: Box::new(ret),
    }
}

/// `(Surface) -> ret` 函数类型。
fn surface_fn(ret: MonoType) -> MonoType {
    mono_fn(vec![MonoType::TypeRef("Surface".to_string())], ret)
}

/// 约束侧接口 `Drawable { draw: (Surface) -> Void }`。
fn drawable_constraint() -> MonoType {
    record(
        "Drawable",
        vec![("draw", surface_fn(MonoType::Void))],
        vec![],
        vec![],
    )
}

/// `View { name: String, ref_field: &Int }`（所有字段均 Dup）。
fn view_type() -> MonoType {
    record(
        "View",
        vec![
            ("name", MonoType::make_string()),
            (
                "ref_field",
                MonoType::Ref {
                    mutable: false,
                    inner: Box::new(MonoType::Int(64)),
                },
            ),
        ],
        vec![false, false],
        vec![false, false],
    )
}

/// `Buffer { data: List(Int), len: Int }`（含非 Dup 的 List 字段）。
fn buffer_type() -> MonoType {
    record(
        "Buffer",
        vec![
            ("data", MonoType::make_list(MonoType::Int(64))),
            ("len", MonoType::Int(64)),
        ],
        vec![false, false],
        vec![false, false],
    )
}

// Happy path 测试

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
    let constraint = record("Empty", vec![], vec![], vec![]);

    // Act
    let result = checker.check_constraint(&ty, &constraint, None);

    // Assert - 空约束应该通过（空接口）
    assert!(
        result.is_ok(),
        "empty constraint should be satisfied by any type"
    );
}

// Error path 测试

/// List 类型不实现 Clone（泛型容器需显式实现）
#[test]
fn test_check_trait_bounds_not_satisfied() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange
    let checker = BoundsChecker::new();
    let ty = MonoType::make_list(MonoType::Int(32));
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
    let ty = record(
        "Point",
        vec![("x", MonoType::Float(64))],
        vec![false],
        vec![false],
    );
    let constraint = drawable_constraint();

    // Act
    let result = checker.check_constraint(&ty, &constraint, None);

    // Assert - 应该报告缺少方法
    assert!(result.is_err(), "should report missing method");
}

// Dup trait bound with auto-derive tests

/// 结构体所有字段均为 Dup 类型时，TraitTable 应通过 Dup 检查
#[test]
fn test_check_trait_bounds_dup_struct_auto_derive() {
    // Arrange - View { name: String, ref_field: &Int } 所有字段均为 Dup
    let trait_table = TraitTable::with_std();

    let struct_type = view_type();

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

    let struct_type = buffer_type();

    // Act & Assert
    assert!(
        !trait_table.satisfies("Dup", &struct_type),
        "Buffer with List field should NOT pass Dup check"
    );
}

// validate_const_args 测试

#[test]
fn test_validate_const_args_int_matches() {
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::MonoType;
    use crate::frontend::core::sema::inference::bounds::validate_const_args;

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
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef, ConstValue};
    use crate::frontend::core::types::MonoType;
    use crate::frontend::core::sema::inference::bounds::validate_const_args;

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
