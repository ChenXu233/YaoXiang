//! 量词事实与两视图（RFC-039 D62 / issue #533 阶段 0）
//!
//! 判据锚：`docs/src/reference/language-spec/type-system.md` §7.1（编译期值参数的
//! 用途判定是有唯一分类器的一条事实）、`docs/src/rfc/accepted/011-generic-type-system.md`
//! §4.1（调用点实参序列与实现头绑定序列是**两条**序列）。
//!
//! 病史：6.9.1 收口实录 §七 第 6 条——`refined_relation_arg` 的 `f`/`g` 把谓词应用的
//! 实参位登记成编译期值参数（`generic_params: ["N"]` / `["n"]`），判据分叉。
//!
//! 覆盖：
//! - 调用点实参视图保留 `Zero` 位；实现头视图再排除 `Zero`（逐名逐位）
//! - 混合 / 柯里声明的两视图与层视图
//! - 谓词应用的实参位**不是** `Zero`（正反两向）
//! - 已解析的运行期量词（`One` / `Omega`）不被解析源覆盖

use crate::ast::{self, Param, Quantity, Type};
use crate::util::span::Span;

fn name_ty(name: &str) -> Type {
    Type::Name {
        name: name.to_string(),
        span: Span::dummy(),
    }
}

fn param(
    name: &str,
    ty: Type,
) -> Param {
    Param {
        name: name.to_string(),
        ty: Some(ty),
        is_mut: false,
        quantity: None,
        span: Span::dummy(),
    }
}

fn generic(
    head: &str,
    args: Vec<Type>,
) -> Type {
    Type::Generic {
        name: head.to_string(),
        name_span: Span::dummy(),
        args,
    }
}

/// 注入问答：`Int` / `Bool` 可作编译期值参数候选类型；`Eq` 是编译期谓词。
fn classifier<'a>(predicates: &'a [&'a str]) -> impl Fn(&str) -> bool + 'a {
    move |n: &str| predicates.contains(&n)
}

fn views(
    ann: &Type,
    predicates: &[&str],
) -> (Vec<Param>, Vec<Param>, Vec<Param>) {
    let is_const_param_type = |n: &str| matches!(n, "Int" | "Bool");
    let is_predicate = classifier(predicates);
    let no = |_: &str| false;
    let cls = ast::ParamClassifier {
        is_const_param_type: &is_const_param_type,
        is_type_name: &no,
        is_predicate: &is_predicate,
    };
    (
        ann.runtime_value_params(),
        ann.runtime_value_params_cls(&cls),
        ann.impl_head_runtime_params(&no, &cls),
    )
}

/// 量词**事实表**（两视图的共同上游）：逐名取事实施加注入问答。
fn quantities_with(
    ann: &Type,
    predicates: &[&str],
) -> Vec<Param> {
    let is_const_param_type = |n: &str| matches!(n, "Int" | "Bool");
    let is_predicate = classifier(predicates);
    let no = |_: &str| false;
    let cls = ast::ParamClassifier {
        is_const_param_type: &is_const_param_type,
        is_type_name: &no,
        is_predicate: &is_predicate,
    };
    ann.param_quantities(&cls)
}

/// 规范：`Array(Int, N)` 里的 `N` 被当类型用 ⇒ 它是编译期值参数（q=0）。
/// 两条序列的分工（RFC-011 §4.1）：
/// - **调用点实参视图**（`runtime_value_params`）保留它的实参位（`f(42)` 要占位）；
/// - **实现头绑定序列**（`runtime_value_params_cls`）与**实现头所绑那一层**
///   （`impl_head_runtime_params`）都不含它。
#[test]
fn test_quantity_zero_kept_by_call_site_view_only() {
    // Arrange — `f: (N: Int) -> Array(Int, N)`
    let ann = Type::Fn {
        params: vec![param("N", name_ty("Int"))],
        return_type: Box::new(generic("Array", vec![name_ty("Int"), name_ty("N")])),
    };

    // Act
    let (call_site, impl_binding, impl_head) = views(&ann, &[]);

    // Assert
    assert_eq!(
        call_site
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        vec!["N"],
        "调用点实参视图必须保留 q=0 的实参位（`f(42)` 占位）；实际: {call_site:?}"
    );
    assert!(
        impl_binding.is_empty(),
        "实现头绑定序列排除 q=0；实际: {impl_binding:?}"
    );
    assert!(
        impl_head.is_empty(),
        "实现头所绑那一层同样排除 q=0；实际: {impl_head:?}"
    );
}

/// 规范：混合 + 柯里声明的两视图**逐名逐位**（RFC-011 §4.1 的两条序列）。
///
/// 声明：`f: (A: Type, N: Int, x: Int) -> (n: N) -> Int`
/// - 类型位 `A` 不在任何序列里；
/// - `N` 是 q=0（`n: N` 引用它）⇒ 只出现在调用点实参视图；
/// - `x` 是实现头绑定序列的一位，但层视图（最内层 `(n: N) -> Int`）只绑 `n`。
#[test]
fn test_quantities_two_views_name_by_name_on_curried_declaration() {
    // Arrange
    let ann = Type::Fn {
        params: vec![
            param(
                "A",
                Type::MetaType {
                    name_span: Span::dummy(),
                    args: vec![],
                },
            ),
            param("N", name_ty("Int")),
            param("x", name_ty("Int")),
        ],
        return_type: Box::new(Type::Fn {
            params: vec![param("n", name_ty("N"))],
            return_type: Box::new(name_ty("Int")),
        }),
    };

    // Act
    let (call_site, impl_binding, impl_head) = views(&ann, &[]);

    // Assert — 调用点实参视图：声明序，含 q=0，去类型位
    assert_eq!(
        call_site
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        vec!["N", "x", "n"],
        "调用点实参序列 = 非类型形参（保留 Zero），声明序；实际: {call_site:?}"
    );
    // 实现头绑定序列：拍平，排除 Zero
    assert_eq!(
        impl_binding
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        vec!["x", "n"],
        "实现头绑定序列 = 非类型形参再排除 Zero；实际: {impl_binding:?}"
    );
    // 实现头所绑那一层：最内层值组
    assert_eq!(
        impl_head
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        vec!["n"],
        "层视图只含最内层值组；实际: {impl_head:?}"
    );
}

/// 规范：谓词应用的**实参位是值不是类型**（RFC-027a / RFC-027 §2）。
///
/// `f: (N: Int, n: Eq(n, N)) -> Int`：`Eq` 是谓词时 `eq(n, N)` 的两个实参都是**值**
/// ⇒ `N` 不是编译期值参数。负向（把 `Eq` 当普通类型应用）时 `N` 才被当类型用。
#[test]
fn test_quantity_predicate_argument_is_not_zero() {
    // Arrange
    let ann = Type::Fn {
        params: vec![
            param("N", name_ty("Int")),
            param("n", generic("Eq", vec![name_ty("n"), name_ty("N")])),
        ],
        return_type: Box::new(name_ty("Int")),
    };

    // Act — 谓词注入「Eq 是谓词」
    let facts = quantities_with(&ann, &["Eq"]);

    // Assert
    let n = facts.iter().find(|p| p.name == "N");
    assert!(
        n.is_some_and(|p| p.quantity != Some(Quantity::Zero)),
        "谓词应用的实参位是值 ⇒ N 不是 q=0；实际: {facts:?}"
    );
    // 负向：谓词注入为「否」时 N 被当类型用（旧判据面，用于说明判据差的可观察后果）
    let without_predicate = quantities_with(&ann, &[]);
    let n = without_predicate.iter().find(|p| p.name == "N");
    assert!(
        n.is_some_and(|p| p.quantity == Some(Quantity::Zero)),
        "不认谓词时 N 会被当类型用——这正是 6.9.1 观察项的成因；实际: {without_predicate:?}"
    );
}

/// 规范：解析源**只写** `Zero`——已解析的运行期量词（`One` / `Omega`，取值源是
/// RFC-009 的 Move/Dup 分类）不得被覆盖，也不得被两个视图丢弃。
#[test]
fn test_resolve_preserves_runtime_quantities() {
    // Arrange — 一个已带 `Omega` 的运行期值参数（`s: String` 是 Dup 行）+ 一个未解析的
    let mut s = param("s", name_ty("String"));
    s.quantity = Some(Quantity::Omega);
    let ann = Type::Fn {
        params: vec![s, param("n", name_ty("Int"))],
        return_type: Box::new(name_ty("Int")),
    };
    let is_const_param_type = |n: &str| matches!(n, "Int" | "Bool");
    let no = |_: &str| false;
    let cls = ast::ParamClassifier {
        is_const_param_type: &is_const_param_type,
        is_type_name: &no,
        is_predicate: &no,
    };

    // Act
    let facts = ann.param_quantities(&cls);

    // Assert — 已解析的事实原样保留，未解析的保持 None（不默认任何一种 q）
    assert_eq!(
        facts
            .iter()
            .find(|p| p.name == "s")
            .and_then(|p| p.quantity),
        Some(Quantity::Omega),
        "已解析的运行期量词不得被解析源覆盖；实际: {facts:?}"
    );
    assert!(
        facts.iter().all(|p| p.quantity != Some(Quantity::Zero)),
        "两个形参都没在类型位置被引用 ⇒ 不得出现 q=0 事实；实际: {facts:?}"
    );
}
