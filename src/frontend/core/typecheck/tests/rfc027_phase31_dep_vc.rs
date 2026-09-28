//! RFC-027 Phase 3.1 集成测试：TypeDepGraph 注册 / 查询 / 清除
//!
//! RFC-027 §6.1：当被依赖变量变更时，编译器重验证依赖变量的精化类型。
//!
//! #379 微重构后依赖图经 `register_refined` 注册（约束本体 + 自由变量建边），
//! 测试用真实 `Refined` 类型驱动——不再是「自测自己的桩」（#377 的教训：
//! 旧测试直接 `add_dep` 打边，生产路径坏了测试照样绿）。

use crate::frontend::core::typecheck::proof::dep_graph::TypeDepGraph;
use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

/// 构造 `{ name == d1 + d2 + ... }` 形精化类型（自由变量 = deps ∪ {name}）
fn refined_summing(
    name: &str,
    deps: &[&str],
) -> MonoType {
    let mut rhs = ConstExpr::NamedVar(deps[0].to_string());
    for d in &deps[1..] {
        rhs = ConstExpr::BinOp {
            op: BinOp::Add,
            left: Box::new(rhs),
            right: Box::new(ConstExpr::NamedVar(d.to_string())),
        };
    }
    MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::BinOp {
            op: BinOp::Eq,
            left: Box::new(ConstExpr::NamedVar(name.to_string())),
            right: Box::new(rhs),
        },
    }
}

/// 无变量约束（恒真式）的精化类型
fn refined_literal_true() -> MonoType {
    MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::Lit(ConstValue::Bool(true)),
    }
}

// RFC-027 §6.1: 依赖图注册与查询

/// 注册精化绑定后，约束自由变量正确建边、约束本体入图
#[test]
fn test_dep_graph_build_and_query() {
    // s: { s == i + arr } → 依赖: s → i, s → arr（约束 SumUpTo(arr, i) 的内联形态）
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.register_refined("s", &refined_summing("s", &["i", "arr"]));

    let affected_by_i = dep_graph.affected_by("i");
    assert!(affected_by_i.contains(&"s"), "i 变更应影响 s");

    let affected_by_arr = dep_graph.affected_by("arr");
    assert!(affected_by_arr.contains(&"s"), "arr 变更应影响 s");

    // 约束本体入图：重验证（VC 生成）直接读图，不回查 env（#379 根因）
    let constraint = dep_graph.constraint_of("s");
    assert!(constraint.is_some(), "s 的精化约束应入图");
    assert!(matches!(constraint.unwrap(), MonoType::Refined { .. }));
}

/// 多变量依赖同一被依赖变量
#[test]
fn test_dep_graph_multiple_dependants() {
    // s: { s == i }，t: { t == i + j }
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.register_refined("s", &refined_summing("s", &["i"]));
    dep_graph.register_refined("t", &refined_summing("t", &["i", "j"]));

    let affected_by_i = dep_graph.affected_by("i");
    assert_eq!(
        affected_by_i.len(),
        2,
        "i 变更应影响 s 和 t，实际: {affected_by_i:?}"
    );
    assert!(affected_by_i.contains(&"s"), "i 变更应影响 s");
    assert!(affected_by_i.contains(&"t"), "i 变更应影响 t");

    let affected_by_j = dep_graph.affected_by("j");
    assert_eq!(affected_by_j.len(), 1, "j 变更应只影响 t");
    assert!(affected_by_j.contains(&"t"), "j 变更应影响 t");
}

/// 无依赖时 affected_by 返回空（恒真约束不建边）
#[test]
fn test_dep_graph_no_dependency_returns_empty() {
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.register_refined("x", &refined_literal_true());
    let affected = dep_graph.affected_by("x");
    assert!(affected.is_empty(), "无依赖时 affected_by 应返回空");
}

/// 自依赖不计入依赖图（register_refined 跳过 dependant 自身）
#[test]
fn test_dep_graph_self_reference_not_recorded() {
    // s: { s == s + i }——约束引用自身，边只建 i → s，不建 s → s
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.register_refined("s", &refined_summing("s", &["s", "i"]));

    let affected = dep_graph.affected_by("s");
    assert!(affected.is_empty(), "自依赖不应建边，实际: {affected:?}");
    assert!(
        dep_graph.affected_by("i").contains(&"s"),
        "非自身自由变量应正常建边"
    );
}

// RFC-027 §6.1: 赋值顺序强制（机制验证）

/// 正确的赋值顺序：依赖图查询正确触发
///
/// mut s: SumUpTo(arr, i) = 0
/// mut i: UpTo(arr.len) = 0
/// while i < arr.len {
///     s += arr[i]  // 先更新 s
///     i += 1       // 再更新 i → 触发 s 重验证
/// }
#[test]
fn test_correct_assignment_order_triggers_dep_check() {
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.register_refined("s", &refined_summing("s", &["arr", "i"]));

    // i 被赋值 → 查询依赖 i 的变量
    let affected = dep_graph.affected_by("i");
    assert!(
        affected.contains(&"s"),
        "i 变更应触发 s 的重验证，实际: {affected:?}"
    );
}

/// 错误的赋值顺序也会触发依赖检查（但不保证通过）
///
/// i += 1       // 先更新 i → s 仍为旧值
/// s += arr[i]  // 编译器应在前步检测到不变量违反
#[test]
fn test_wrong_assignment_order_still_triggers_dep_check() {
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.register_refined("s", &refined_summing("s", &["i"]));

    // 即使赋值顺序错误，依赖图查询仍应正确返回受影响变量
    let affected = dep_graph.affected_by("i");
    assert!(
        !affected.is_empty(),
        "即使赋值顺序错误，依赖图查询仍应返回受影响变量"
    );
}

// RFC-027 §6.1: 传递依赖链 + remove_dependant

/// 传递依赖链 a→b→c：a 变更影响 b，b 变更影响 c
#[test]
fn test_dep_graph_transitive_chain() {
    // a: { a == b }，b: { b == c }
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.register_refined("a", &refined_summing("a", &["b"]));
    dep_graph.register_refined("b", &refined_summing("b", &["c"]));

    let affected_by_c = dep_graph.affected_by("c");
    assert!(affected_by_c.contains(&"b"), "c 变更应直接影响 b");
    // 传递性：c 变更 → b 被重验证 → 触发的 b 赋值再影响 a
    // 但依赖图本身只记录直接依赖，传递由 checker 的递归查询处理
    let affected_by_b = dep_graph.affected_by("b");
    assert!(affected_by_b.contains(&"a"), "b 变更应直接影响 a");
}

/// remove_dependant：变量离开作用域时清除依赖记录（含约束）
#[test]
fn test_dep_graph_remove_dependant_clears_edges() {
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.register_refined("s", &refined_summing("s", &["i"]));
    dep_graph.register_refined("t", &refined_summing("t", &["i"]));

    // s 离开作用域
    dep_graph.remove_dependant("s");

    // i 仍影响 t，但不再影响 s
    let affected = dep_graph.affected_by("i");
    assert!(!affected.contains(&"s"), "s 被移除后不应再受 i 影响");
    assert!(affected.contains(&"t"), "t 仍应受 i 影响");
    assert!(
        dep_graph.constraint_of("s").is_none(),
        "s 的约束应随移除清除"
    );
}

/// 菱形依赖：两端都依赖同一变量
#[test]
fn test_dep_graph_diamond_dependency() {
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.register_refined("s", &refined_summing("s", &["i"]));
    dep_graph.register_refined("t", &refined_summing("t", &["i"]));

    let affected = dep_graph.affected_by("i");
    assert_eq!(affected.len(), 2, "i 变更应影响 s 和 t，实际: {affected:?}");
    assert!(affected.contains(&"s"), "菱形: i→s 应成立");
    assert!(affected.contains(&"t"), "菱形: i→t 应成立");
}

/// 空图 remove 不崩溃
#[test]
fn test_dep_graph_empty_remove_does_not_panic() {
    let mut dep_graph = TypeDepGraph::new();
    // 移除不存在的依赖者不应 panic
    dep_graph.remove_dependant("nonexistent");
    assert!(dep_graph.affected_by("x").is_empty());
}
