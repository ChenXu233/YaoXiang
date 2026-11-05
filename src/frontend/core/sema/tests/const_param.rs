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

use crate::ast::{BinOp, Expr, GenericParam, GenericParamKind, Param, Type};
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

/// 构造 `(params) -> Int` 形态的声明标注
fn fn_ann(params: Vec<Param>) -> Type {
    Type::Fn {
        params,
        return_type: Box::new(int_type()),
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

    // Act
    let used = used_as_const_in_annotation(&ann, &candidates, &|_| false);

    // Assert
    assert!(
        used.contains("N"),
        "本组形参标注 `M: N` 的 N 是类型引用，必须计入 const 用途；实际: {used:?}"
    );
}

/// 规范：谓词应用的实参位是**值**，不是类型引用（RFC-027a / §7.1）
///
/// 预期行为：`(x: IsPositive(N)) -> Int` 的 `N` 不计入 const 用途——谓词实参位是
/// 编译期表达式操作数，计入会让形参名被替换成底层类型、精化约束在消费端之前丢失（E1092）
#[test]
fn test_used_as_const_in_annotation_skips_predicate_application_arg() {
    // Arrange：`(x: IsPositive(N)) -> Int`
    let ann = fn_ann(vec![sig_param(
        "x",
        Some(Type::Generic {
            name: "IsPositive".to_string(),
            name_span: Span::default(),
            args: vec![name_type("N")],
        }),
    )]);
    let candidates: HashSet<String> = ["N".to_string()].into();

    // Act
    let used = used_as_const_in_annotation(&ann, &candidates, &|n| n == "IsPositive");

    // Assert
    assert!(
        used.is_empty(),
        "谓词应用实参位是值，不得计入 const 用途（RFC-027a）；实际: {used:?}"
    );
}

/// 规范：类型构造的实参位是**类型引用**（§7.1 `Array(T, N)` 的 `N`）
///
/// 预期行为：`(x: Array(Int, N)) -> Int` 的 `N` 计入 const 用途（`Array` 不是谓词）
#[test]
fn test_used_as_const_in_annotation_counts_type_ctor_arg() {
    // Arrange：`(x: Array(Int, N)) -> Int`
    let ann = fn_ann(vec![sig_param(
        "x",
        Some(Type::Generic {
            name: "Array".to_string(),
            name_span: Span::default(),
            args: vec![int_type(), name_type("N")],
        }),
    )]);
    let candidates: HashSet<String> = ["N".to_string()].into();

    // Act
    let used = used_as_const_in_annotation(&ann, &candidates, &|_| false);

    // Assert
    assert!(
        used.contains("N"),
        "类型构造实参位的 N 必须计入 const 用途；实际: {used:?}"
    );
}

/// 规范：`Assert(N > 0)` 的 `ConstExpr` 是编译期值引用
///
/// 预期行为：
/// - `Type::ConstExpr` 里的 `N` 计入 const 用途
///
/// 这条同时是本函数**不委托 `ast::name_used_as_type`** 的正确性依据：后者对
/// `ConstExpr` / `MetaType` / `Paren` / `NamedParen` 落 `_ => false`（见该函数的
/// 「已知共同缺口」），缺口未补前把两处窄扫描并进唯一分类器，会把谓词里的
/// 编译期值引用判成落空。
#[test]
fn test_used_as_const_in_annotation_counts_assert_const_expr() {
    // Arrange：`(x: Assert(N > 0)) -> Int`
    let ann = fn_ann(vec![sig_param(
        "x",
        Some(Type::Generic {
            name: "Assert".to_string(),
            name_span: Span::default(),
            args: vec![Type::ConstExpr(Box::new(Expr::BinOp {
                op: BinOp::Gt,
                left: Box::new(Expr::Var("N".to_string(), Span::default())),
                right: Box::new(Expr::Var("0".to_string(), Span::default())),
                span: Span::default(),
            }))],
        }),
    )]);
    let candidates: HashSet<String> = ["N".to_string()].into();

    // Act
    let used = used_as_const_in_annotation(&ann, &candidates, &|_| false);

    // Assert
    assert!(
        used.contains("N"),
        "`Assert(N > 0)` 的 N 是编译期值引用，必须计入 const 用途；实际: {used:?}"
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

/// 规范：落空候选不进 binders，fallen 携带原始签名参数供并入参数链或报 E1094
///
/// 预期行为：
/// - 未被类型位置引用的候选出现在 fallen 中
/// - fallen 元素即声明签名中的原始 Param（含类型标注）
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
