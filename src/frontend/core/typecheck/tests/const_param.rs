//! 编译期值参数判定测试 — RFC-011 §4.1 两步判定精筛实现
//!
//! RFC-011: docs/src/design/rfc/accepted/011-generic-type-system.md §4.1
//!
//! 测试点：
//! - 用途命中的候选 → const_binders（index 连续，基于 Type 参数数起编）
//! - 落空候选 → fallen 携带原始签名 Param
//! - 混合场景：命中与落空正确分流
//! - annotation 形态 → ConstKind 映射

use std::collections::HashSet;

use crate::frontend::core::parser::ast::{GenericParam, GenericParamKind, Param, Type};
use crate::frontend::core::typecheck::const_param::{ast_type_to_const_kind, resolve_const_candidates};
use crate::frontend::core::types::const_data::ConstKind;
use crate::util::span::Span;

/// 构造 const 候选泛型参数
fn const_param(
    name: &str,
    ty: Type,
) -> GenericParam {
    GenericParam {
        name: name.to_string(),
        kind: GenericParamKind::Const {
            const_type: Box::new(ty),
        },
        constraints: Vec::new(),
    }
}

/// 构造签名值参数
fn sig_param(
    name: &str,
    ty: Option<Type>,
) -> Param {
    Param {
        name: name.to_string(),
        ty,
        is_mut: false,
        span: Span::default(),
    }
}

/// 规范：用途命中的候选进 const_binders，index 从 n_type_params 起连续编号
///
/// 预期行为：
/// - 两个命中候选的 index 为 n、n+1（无空洞）
/// - index 连续性是 process_body_expr_item 按下标取元素的硬前提
#[test]
fn test_resolve_const_candidates_hit_builds_contiguous_index() {
    // Arrange
    let params = vec![
        const_param(
            "N",
            Type::Name {
                name: "Int".to_string(),
                span: Span::default(),
            },
        ),
        const_param("M", Type::Bool),
    ];
    let sig = vec![sig_param("N", None), sig_param("M", None)];
    let used: HashSet<String> = ["N".to_string(), "M".to_string()].into();

    // Act
    let resolved = resolve_const_candidates(&params, &sig, &used, 1);

    // Assert
    assert_eq!(
        resolved.const_binders.len(),
        2,
        "两个命中候选都应进 binders"
    );
    assert_eq!(
        resolved.const_binders[0].index, 1,
        "首个 binder index 应为 type_param 数"
    );
    assert_eq!(
        resolved.const_binders[1].index, 2,
        "binder index 应连续无空洞"
    );
    assert!(resolved.fallen.is_empty(), "无落空候选");
}

/// 规范：落空候选不进 binders，fallen 携带原始签名参数供并入参数链或报 E1094
///
/// 预期行为：
/// - 未被类型位置引用的候选出现在 fallen 中
/// - fallen 元素即 signature_params 中的原始 Param（含类型标注）
#[test]
fn test_resolve_const_candidates_fallen_carries_original_param() {
    // Arrange
    let int_ty = Type::Name {
        name: "Int".to_string(),
        span: Span::default(),
    };
    let params = vec![const_param("t", int_ty.clone())];
    let sig = vec![sig_param("t", Some(int_ty.clone()))];
    let used: HashSet<String> = HashSet::new();

    // Act
    let resolved = resolve_const_candidates(&params, &sig, &used, 0);

    // Assert
    assert!(resolved.const_binders.is_empty(), "落空候选不得进 binders");
    assert_eq!(resolved.fallen.len(), 1, "落空候选应返回原始参数");
    assert_eq!(resolved.fallen[0].name, "t", "fallen 应携带原参数名");
    assert!(
        resolved.fallen[0].ty.is_some(),
        "fallen 应保留原始类型标注（E1094 文案与值参数链都需要）"
    );
}

/// 规范：混合场景下命中与落空按用途分析精确分流，互不串扰
///
/// 预期行为：
/// - N 在类型位置被引用 → binder；t 仅 body 值位置引用（#295/#297 场景）→ fallen
#[test]
fn test_resolve_const_candidates_mixed_hit_and_fallen_split() {
    // Arrange
    let int_ty = Type::Name {
        name: "Int".to_string(),
        span: Span::default(),
    };
    let params = vec![
        const_param("N", int_ty.clone()),
        const_param("t", int_ty.clone()),
    ];
    let sig = vec![sig_param("N", None), sig_param("t", None)];
    let used: HashSet<String> = ["N".to_string()].into();

    // Act
    let resolved = resolve_const_candidates(&params, &sig, &used, 0);

    // Assert
    assert_eq!(resolved.const_binders.len(), 1, "仅 N 命中");
    assert_eq!(resolved.const_binders[0].name, "N", "命中者应为 N");
    assert_eq!(resolved.fallen.len(), 1, "仅 t 落空");
    assert_eq!(resolved.fallen[0].name, "t", "落空者应为 t");
}

/// 规范：annotation 形态映射 ConstKind（Int/Float/Bool/未知回退 Int）
///
/// 预期行为：
/// - Name("Float") → Float；Bool → Bool；无法识别的形态回退 Int(None)
#[test]
fn test_ast_type_to_const_kind_maps_annotation_forms() {
    // Arrange / Act / Assert
    let float_ty = Type::Name {
        name: "Float".to_string(),
        span: Span::default(),
    };
    assert!(
        matches!(ast_type_to_const_kind(&float_ty), ConstKind::Float(_)),
        "Name(Float) 应映射为 ConstKind::Float"
    );
    assert!(
        matches!(ast_type_to_const_kind(&Type::Bool), ConstKind::Bool),
        "Bool 应映射为 ConstKind::Bool"
    );
    let unknown_ty = Type::Name {
        name: "Bizarre".to_string(),
        span: Span::default(),
    };
    assert!(
        matches!(ast_type_to_const_kind(&unknown_ty), ConstKind::Int(None)),
        "无法识别形态应回退 ConstKind::Int(None)"
    );
}
