//! 编译期值参数判定测试 — RFC-011 §4.1 两步判定精筛实现
//!
//! RFC-011: docs/src/rfc/accepted/011-generic-type-system.md §4.1
//!
//! 测试点：
//! - 用途命中的候选 → const_binders（index 连续，基于 Type 参数数起编）
//! - 落空候选 → fallen 携带原始签名 Param
//! - 混合场景：命中与落空正确分流
//! - annotation 形态 → ConstKind 映射
//! - 用途精筛的扫描面（`used_as_const_in_annotation`）：
//!   类型位置 = 声明中的全部 `Type` 子树（SPEC type-system §7.1）

use std::collections::HashSet;

use crate::ast::{GenericParam, GenericParamKind, Param, Type};
use crate::frontend::core::sema::const_param::{
    ast_type_to_const_kind, resolve_const_candidates, used_as_const_in_annotation,
};
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

// ─────────────────────────────────────────────────────────────
// 用途精筛的**扫描面**（`used_as_const_in_annotation`）
//
// 规范：type-system.md §7.1——类型位置 = 声明中的全部 `Type` 子树，判定时遍历
// **全部** 子树、不按位置分情况。三处调用点（Registry 面 / 签名面 / 函数体面）
// 共用本函数，以下四条钉住扫描面本身。
// ─────────────────────────────────────────────────────────────

/// 构造 `Int` 标注
fn int_type() -> Type {
    Type::Name {
        name: "Int".to_string(),
        span: Span::default(),
    }
}

/// 构造裸名类型标注（如 `N`）
fn name_type(name: &str) -> Type {
    Type::Name {
        name: name.to_string(),
        span: Span::default(),
    }
}

/// 规范：**本组形参自己的标注**同样属于类型位置（§7.1「遍历全部子树」）
///
/// 预期行为：
/// - `f: (N: Int, M: N) -> Int` 里 `M: N` 的 `N` 计入 const 用途
/// - 只扫返回位（或「返回位 + 返回位内层 Fn 的形参」）的旧口径漏掉这一处
///   ⇒ `const_binders` 为空 ⇒ `M` 的标注停在裸名（实测 E1012）
#[test]
fn test_used_as_const_in_annotation_scans_own_group_param_annotation() {
    // Arrange：`f: (N: Int, M: N) -> Int`
    let ann = Type::Fn {
        params: vec![
            sig_param("N", Some(int_type())),
            sig_param("M", Some(name_type("N"))),
        ],
        return_type: Box::new(int_type()),
    };
    let candidates: HashSet<String> = ["N".to_string()].into();

    // Act — 命名空间问答注入「否」：本格钉的是「本组形参自己的标注也属于类型位置」
    let used = used_as_const_in_annotation(&ann, &candidates, &|_| false, &|_| false);

    // Assert
    assert!(
        used.contains("N"),
        "本组形参标注 `M: N` 的 N 是类型引用，必须计入 const 用途；实际: {used:?}"
    );
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
        const_param(
            "M",
            Type::Name {
                name: "Bool".to_string(),
                span: Span::default(),
            },
        ),
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
        matches!(
            ast_type_to_const_kind(&Type::Name {
                name: "Bool".to_string(),
                span: Span::default(),
            }),
            ConstKind::Bool
        ),
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

/// 规范：SPEC §7.1「被引用按命名空间解析判定，不做名字的字面比对」——
/// `f: (Int: Int) -> Int` 里两个 `Int` 分属值/类型命名空间，类型位上的 `Int`
/// 解析到**类型** ⇒ 与形参无关，候选 `Int` 必须落空。
#[test]
fn test_used_as_const_in_annotation_resolves_type_name_first() {
    // Arrange — `f: (Int: Int) -> Int`
    let ann = Type::Fn {
        params: vec![sig_param("Int", Some(int_type()))],
        return_type: Box::new(int_type()),
    };
    let candidates: HashSet<String> = ["Int".to_string()].into();

    // Act — `Int` 在类型命名空间里可解析（SPEC §7.1 第 1、2 条）
    let used = used_as_const_in_annotation(&ann, &candidates, &|n| n == "Int", &|_| false);

    // Assert
    assert!(
        used.is_empty(),
        "类型位上的 Int 解析到类型命名空间 ⇒ 与形参无关，候选 Int 必须落空；实际: {used:?}"
    );
}
