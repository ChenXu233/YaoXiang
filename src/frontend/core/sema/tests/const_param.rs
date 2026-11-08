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
// ─────────────────────────────────────────────────────────────
// 6.8.10：命名空间问答**四处同源**——用途精筛与唯一分类器对同一候选必须同判
//
// 判据来源：SPEC type-system §7.1 第 2 条「解析到**已声明类型**」= `env.types` 的键
// （本地类型定义 + `use` 导入的类型导出）。四个调用点（Registry / 签名 / 构造器 /
// 函数体面）注入同一集合：checker 三面经 `TypeChecker::is_declared_type_name`、
// 函数体面经 `StatementChecker::type_names`。
// ─────────────────────────────────────────────────────────────

/// 构造 `Vec(T)` 形态的类型
fn vec_of(arg: Type) -> Type {
    Type::Generic {
        name: "Vec".to_string(),
        name_span: Span::default(),
        args: vec![arg],
    }
}

/// 构造 `Bool` 标注
fn bool_type() -> Type {
    Type::Name {
        name: "Bool".to_string(),
        span: Span::default(),
    }
}

/// 两面对**同一份声明**逐候选比对：`used_as_const_in_annotation`（四调用点共用）
/// 与 `classify_compile_time_value_params`（`runtime_value_params_cls` 的内核）同判。
fn assert_faces_agree(
    ann: &Type,
    params: &[Param],
    declared_types: &[&str],
    label: &str,
) {
    let cls = crate::ast::ParamClassifier {
        is_const_param_type: &|n| crate::frontend::core::sema::probe::is_const_param_type_name(n),
        is_type_name: &|n| declared_types.contains(&n),
        is_predicate: &|_| false,
    };
    // 粗筛候选：有名 + 标注是 const 参数类型名（与分类器粗筛同一判据）
    let candidates: HashSet<String> = params
        .iter()
        .filter(|p| p.is_named())
        .filter_map(|p| match p.ty.as_ref() {
            Some(Type::Name { name, .. })
                if crate::frontend::core::sema::probe::is_const_param_type_name(name) =>
            {
                Some(p.name.clone())
            }
            _ => None,
        })
        .collect();

    let via_classifier = crate::ast::classify_compile_time_value_params(params, &[ann], &cls);
    let via_annotation =
        used_as_const_in_annotation(ann, &candidates, cls.is_type_name, cls.is_predicate);

    assert_eq!(
        via_annotation, via_classifier,
        "{label}：用途精筛与唯一分类器必须同判（四处同一集合）；候选: {candidates:?}"
    );
}

/// 格 1：真 const 参数（`(N: Int) -> (n: N)`）两面同判命中。
#[test]
fn test_const_param_faces_agree_on_real_const_param() {
    // Arrange — 内层 `n: N` 的标注是 Name("N")，不是 const 参数类型 ⇒ 只有 N 是候选
    let ann = Type::Fn {
        params: vec![sig_param("N", Some(int_type()))],
        return_type: Box::new(Type::Fn {
            params: vec![sig_param("n", Some(name_type("N")))],
            return_type: Box::new(int_type()),
        }),
    };
    let params = vec![
        sig_param("N", Some(int_type())),
        sig_param("n", Some(name_type("N"))),
    ];

    // Act & Assert
    assert_faces_agree(&ann, &params, &[], "(N: Int) -> (n: N)");
}

/// 格 2：候选名是内置名（`Bool`）——本轮口径下不解析到已声明类型 ⇒ 两面同判命中。
#[test]
fn test_const_param_faces_agree_on_builtin_name_candidate() {
    // Arrange — `(Bool: Int) -> Vec(Bool)`
    let ann = Type::Fn {
        params: vec![sig_param("Bool", Some(int_type()))],
        return_type: Box::new(vec_of(name_type("Bool"))),
    };
    let params = vec![sig_param("Bool", Some(int_type()))];

    // Act & Assert
    assert_faces_agree(&ann, &params, &[], "(Bool: Int) -> Vec(Bool)");
}

/// 格 3：候选名是内置名（`Int`）且标注不同类型（`Bool`）——两面同判命中。
#[test]
fn test_const_param_faces_agree_on_builtin_name_with_other_annotation() {
    // Arrange — `(Int: Bool) -> Vec(Int)`
    let ann = Type::Fn {
        params: vec![sig_param("Int", Some(bool_type()))],
        return_type: Box::new(vec_of(name_type("Int"))),
    };
    let params = vec![sig_param("Int", Some(bool_type()))];

    // Act & Assert
    assert_faces_agree(&ann, &params, &[], "(Int: Bool) -> Vec(Int)");
}

/// 格 4：候选名是 std 导出类型名（`Error`）——本轮口径下仍未声明 ⇒ 两面同判命中。
#[test]
fn test_const_param_faces_agree_on_std_export_name_candidate() {
    // Arrange — `(Error: Int) -> Vec(Error)`
    let ann = Type::Fn {
        params: vec![sig_param("Error", Some(int_type()))],
        return_type: Box::new(vec_of(name_type("Error"))),
    };
    let params = vec![sig_param("Error", Some(int_type()))];

    // Act & Assert
    assert_faces_agree(&ann, &params, &[], "(Error: Int) -> Vec(Error)");
}

/// 格 5（命名空间解析的正例）：候选名等于**已声明类型**（`Foo`）⇒ 两面同判落空。
#[test]
fn test_const_param_faces_agree_when_candidate_is_declared_type() {
    // Arrange — `(Foo: Int) -> Vec(Foo)`，且 `Foo` 是已声明类型
    let ann = Type::Fn {
        params: vec![sig_param("Foo", Some(int_type()))],
        return_type: Box::new(vec_of(name_type("Foo"))),
    };
    let params = vec![sig_param("Foo", Some(int_type()))];

    // Act & Assert
    assert_faces_agree(
        &ann,
        &params,
        &["Foo"],
        "(Foo: Int) -> Vec(Foo) [Foo 已声明]",
    );
}
// ─────────────────────────────────────────────────────────────
// 6.8.10 行为锚：本轮**不宣称**基线面——病态形态仍按编译期值参数判定（E1112），
// 负控（真 const 参数）仍为编译期值参数。放行病态形态须先给 IR 面补命名空间通道。
// ─────────────────────────────────────────────────────────────

/// 源码 → 类型检查结果（与生产入口同一条语义探针）
fn check_source(source: &str) -> crate::frontend::core::sema::types::TypeCheckResult {
    use crate::frontend::core::lexer::tokenize;
    use crate::frontend::core::parser::parse_with_probe;
    use crate::frontend::core::sema::checker::TypeChecker;
    use crate::frontend::core::sema::probe::TypeLevelNamespace;
    use crate::frontend::core::sema::SemanticProbe;

    let tokens = tokenize(source).expect("词法分析应成功");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(
        !parsed.has_errors,
        "解析应无错误，实际: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("test");
    checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    )
}

/// 五类「形参名恰为类型名」的病态形态：本轮口径下仍是编译期值参数 ⇒ 实现头多写一位报 E1112。
///
/// 这不是回归而是**预期结果**：形参名未解析到**已声明类型**（`env.types`），用途精筛命中
/// ⇒ 0 个运行期值位。放行它们会让 IR 面（仍无命名空间通道）按错位出 IR。
#[test]
fn test_pathological_type_name_params_still_report_e1112() {
    // Arrange — 五种名字来源：内置名 ×2 / std 导出名 / 接口名 / 编译器容器名
    let shapes = [
        "f: (Bool: Int) -> Bool = (x) => true\n",
        "f: (Int: Bool) -> Int = (x) => 1\n",
        "f: (Error: Int, e: Error) -> Int = (x, y) => 1\n",
        "f: (Add: Int, a: Vec(Add)) -> Int = (x, y) => 1\n",
        "f: (Vec: Int, v: Vec(Int)) -> Int = (x, y) => 1\n",
    ];

    // Act & Assert
    for shape in shapes {
        let result = check_source(shape);
        assert!(
            result.diagnostics.iter().any(|d| d.code == "E1112"),
            "病态形态 {shape:?} 本轮仍按编译期值参数判定 ⇒ 实现头多出的一位必须报 E1112。实际: {:#?}",
            result.diagnostics
        );
    }
}

/// 负控：真 const 参数仍是编译期值参数（`(N: Int) -> (n: N) -> Int = (n) => n` 零诊断）。
#[test]
fn test_real_const_param_still_compile_time_value_param() {
    // Arrange — N 只作内层形参 n 的类型；实现头只绑运行期的 n
    let source = "f: (N: Int) -> (n: N) -> Int = (n) => n\n";

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "真 const 参数 N 不占运行期值位 ⇒ 该声明应零诊断。实际: {:#?}",
        result.diagnostics
    );
}
