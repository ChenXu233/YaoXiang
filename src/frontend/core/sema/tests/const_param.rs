//! 编译期值参数判定测试 — RFC-011 §4.1 两步判定的落地
//!
//! RFC-011: docs/src/rfc/accepted/011-generic-type-system.md §4.1
//! D62: issue #533（量词事实与 q=0 槽位）
//!
//! 测试点：
//! - 用途命中的候选 → const_binders（index 连续，基于 Type 参数数起编）
//! - 落空候选 → 由量词事实派生（`fallen_const_candidates`），不再有独立通道
//! - 量词解析源（`ast::Type::param_quantities`）与唯一分类器同判：扫描面、命名空间
//! - annotation 形态 → ConstKind 映射

use std::collections::HashSet;

use crate::ast::{GenericParam, GenericParamKind, Param, Quantity, Type};
use crate::frontend::core::sema::const_param::{
    ast_type_to_const_kind, fallen_const_candidates, resolve_const_candidates,
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

/// 构造签名值参数（量词默认未解析——测试按需显式写入事实）
fn sig_param(
    name: &str,
    ty: Option<Type>,
) -> Param {
    Param {
        name: name.to_string(),
        ty,
        is_mut: false,
        quantity: None,
        span: Span::default(),
    }
}

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

/// 已解析的量词事实（`Some(Quantity::Zero)`）——测试里显式构造，不猜默认值。
fn zero_param(mut p: Param) -> Param {
    p.quantity = Some(Quantity::Zero);
    p
}

// ─────────────────────────────────────────────────────────────
// const 绑定：量词事实 → Compile-time binder（RFC-011 §4.1）
// ─────────────────────────────────────────────────────────────

/// 规范：用途命中的候选进 const_binders，index 从 n_type_params 起连续编号
///
/// 预期行为：
/// - 两个命中候选的 index 为 n、n+1（无空洞）
/// - index 连续性是 process_body_expr_item 按下标取元素的硬前提
#[test]
fn test_resolve_const_candidates_hit_builds_contiguous_index() {
    // Arrange — N / M 都带 q=0 事实
    let params = vec![const_param("N", int_type()), const_param("M", bool_type())];
    let resolved_params = vec![
        zero_param(sig_param("N", None)),
        zero_param(sig_param("M", None)),
    ];

    // Act
    let resolved = resolve_const_candidates(&params, &resolved_params, 1);

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
}

/// 规范：**没有 q=0 事实**的候选不进 binder——「未解析」不得被读成「命中」。
#[test]
fn test_resolve_const_candidates_requires_zero_fact() {
    // Arrange — 声明形参只有 `quantity: None`（未解析）
    let params = vec![const_param("N", int_type())];
    let resolved_params = vec![sig_param("N", None)];

    // Act
    let resolved = resolve_const_candidates(&params, &resolved_params, 0);

    // Assert
    assert!(
        resolved.const_binders.is_empty(),
        "`quantity: None` 是「尚未解析」，不得被当作 q=0 命中；实际: {:?}",
        resolved.const_binders.len()
    );
}

/// 规范：**落空候选由绑定事实派生**（构造器级 E1094 的输入，RFC-011 §4.1 / #297 F）。
///
/// 预期行为：
/// - `quantity: None` ⇒ 落空（返回声明形参，供诊断指位）
/// - `quantity: Some(Zero)` ⇒ 不落空
#[test]
fn test_fallen_const_candidates_derives_from_quantity_facts() {
    // Arrange
    let params = vec![const_param("N", int_type()), const_param("M", bool_type())];
    let resolved_params = vec![sig_param("N", None), zero_param(sig_param("M", None))];

    // Act
    let fallen = fallen_const_candidates(&params, &resolved_params);

    // Assert
    assert_eq!(
        fallen.len(),
        1,
        "只有一个候选没有成为 q=0 绑定，实际: {fallen:?}"
    );
    assert_eq!(fallen[0].name, "N", "落空者必须是未成为 q=0 绑定的那个候选");
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
        matches!(ast_type_to_const_kind(&bool_type()), ConstKind::Bool),
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

// ─────────────────────────────────────────────────────────────
// 量词解析源（`ast::Type::param_quantities`）的扫描面与命名空间
//
// 规范：SPEC type-system §7.1——类型位置 = 声明中的全部 `Type` 子树；候选名解析到
// **已声明类型** ⇒ 与形参无关。D62 起判定只有一份：唯一分类器由唯一解析源调用，
// 语义面（`const_param`）只消费 `Param::quantity` 这一条事实。
// ─────────────────────────────────────────────────────────────

/// 解析一份声明的量词事实（测试用固定注入：内置名可作 const 参数类型）
fn quantities_of(ann: &Type) -> Vec<Param> {
    let is_const_param_type =
        |n: &str| crate::frontend::core::sema::probe::is_const_param_type_name(n);
    let no = |_: &str| false;
    let cls = crate::ast::ParamClassifier {
        is_const_param_type: &is_const_param_type,
        is_type_name: &no,
        is_predicate: &no,
    };
    ann.param_quantities(&cls)
}

/// 规范：**本组形参自己的标注**同样属于类型位置（§7.1「遍历全部子树」）
///
/// 预期行为：`f: (N: Int, M: N) -> Int` 里 `M: N` 的 `N` 计入用途 ⇒ N 带 q=0 事实。
/// 只扫返回位的旧口径漏掉这一处 ⇒ `const_binders` 为空 ⇒ `M` 的标注停在裸名（实测 E1012）。
#[test]
fn test_param_quantities_scan_own_group_param_annotation() {
    // Arrange — `f: (N: Int, M: N) -> Int`
    let ann = Type::Fn {
        params: vec![
            sig_param("N", Some(int_type())),
            sig_param("M", Some(name_type("N"))),
        ],
        return_type: Box::new(int_type()),
    };

    // Act
    let facts = quantities_of(&ann);

    // Assert
    let n = facts.iter().find(|p| p.name == "N");
    assert_eq!(
        n.and_then(|p| p.quantity),
        Some(Quantity::Zero),
        "本组形参标注 `M: N` 的 N 是类型引用，必须带 q=0 事实；实际: {facts:?}"
    );
}

/// 规范：SPEC §7.1「被引用按命名空间解析判定，不做名字的字面比对」——
/// `f: (Int: Int) -> Int` 里两个 `Int` 分属值/类型命名空间，类型位上的 `Int`
/// 解析到**类型** ⇒ 与形参无关，候选 `Int` 必须落空。
#[test]
fn test_param_quantities_resolve_declared_type_first() {
    // Arrange — `f: (Int: Int) -> Int`
    let ann = Type::Fn {
        params: vec![sig_param("Int", Some(int_type()))],
        return_type: Box::new(int_type()),
    };
    let is_const_param_type =
        |n: &str| crate::frontend::core::sema::probe::is_const_param_type_name(n);
    let is_declared = |n: &str| n == "Int";
    let no = |_: &str| false;
    let cls = crate::ast::ParamClassifier {
        is_const_param_type: &is_const_param_type,
        is_type_name: &is_declared,
        is_predicate: &no,
    };

    // Act
    let facts = ann.param_quantities(&cls);

    // Assert
    assert!(
        facts.iter().all(|p| p.quantity.is_none()),
        "类型位上的 Int 解析到类型命名空间 ⇒ 与形参无关，候选 Int 必须落空；实际: {facts:?}"
    );
}

// ─────────────────────────────────────────────────────────────
// 6.8.10：命名空间问答**多处同源**——解析源与唯一分类器对同一候选必须同判
//
// 判据来源：SPEC type-system §7.1 第 2 条「解析到**已声明类型**」= `env.types` 的键
// （本地类型定义 + `use` 导入的类型导出）。各面注入同一集合：checker 侧经
// `TypeChecker::is_declared_type_name`、函数体面经 `StatementChecker::type_names`。
// D62 起这条同源是**结构性的**：解析源就是分类器的唯一调用者。
// ─────────────────────────────────────────────────────────────

/// 两面对**同一份声明**逐候选比对：解析源（`param_quantities`）与唯一分类器同判。
fn assert_faces_agree(
    ann: &Type,
    params: &[Param],
    declared_types: &[&str],
    label: &str,
) {
    let is_const_param_type =
        |n: &str| crate::frontend::core::sema::probe::is_const_param_type_name(n);
    let is_type_name = |n: &str| declared_types.contains(&n);
    let no = |_: &str| false;
    let cls = crate::ast::ParamClassifier {
        is_const_param_type: &is_const_param_type,
        is_type_name: &is_type_name,
        is_predicate: &no,
    };

    let via_classifier = crate::ast::classify_compile_time_value_params(params, &[ann], &cls);
    let via_facts: HashSet<String> = ann
        .param_quantities(&cls)
        .into_iter()
        .filter(|p| p.quantity == Some(Quantity::Zero))
        .map(|p| p.name)
        .collect();

    assert_eq!(
        via_facts, via_classifier,
        "{label}：解析源与唯一分类器必须同判（同一集合）；声明形参: {params:?}"
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
