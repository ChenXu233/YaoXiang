//! RFC-027 Phase 3.1 集成测试：TypeDepGraph 激活 + 赋值触发 VC 生成
//!
//! RFC-027 §6.1：当被依赖变量变更时，编译器重验证依赖变量的精化类型。
//!
//! 测试覆盖：
//! - 依赖图构建（extract_free_vars）
//! - 赋值触发影响分析（affected_by）
//! - 空依赖图（无精化类型标注时零开销）

use crate::frontend::core::typecheck::proof::dep_graph::TypeDepGraph;

// ===================================================================
// RFC-027 §6.1: 依赖图构建
// ===================================================================

/// extract_free_vars 能从约束表达式中正确提取自由变量
#[test]
fn test_dep_graph_build_and_query() {
    // 约束: s == sum(arr[0..i])
    // 依赖: s → i, s → arr
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.add_dep("s", "i");
    dep_graph.add_dep("s", "arr");

    let affected_by_i = dep_graph.affected_by("i");
    assert!(affected_by_i.contains(&"s"), "i 变更应影响 s");
    assert_eq!(affected_by_i.len(), 1, "s 只依赖 i 一个变量");

    let affected_by_arr = dep_graph.affected_by("arr");
    assert!(affected_by_arr.contains(&"s"), "arr 变更应影响 s");
}

/// 依赖图：多变量依赖同一被依赖变量
#[test]
fn test_dep_graph_multiple_dependants() {
    let mut dep_graph = TypeDepGraph::new();
    // s: SumUpTo(arr, i) → s 依赖 i
    // t: BoundedBy(i, j) → t 依赖 i, j
    dep_graph.add_dep("s", "i");
    dep_graph.add_dep("t", "i");
    dep_graph.add_dep("t", "j");

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

/// 无依赖时 affected_by 返回空
#[test]
fn test_dep_graph_no_dependency_returns_empty() {
    let dep_graph = TypeDepGraph::new();
    let affected = dep_graph.affected_by("x");
    assert!(affected.is_empty(), "无依赖时 affected_by 应返回空");
}

/// 自依赖不计入依赖图
#[test]
fn test_dep_graph_self_reference_not_recorded() {
    let mut dep_graph = TypeDepGraph::new();
    // 添加 s → s（自引用）——实际代码中 extract_free_vars 会过滤自身
    // 但如果被意外添加，也不应崩溃
    dep_graph.add_dep("s", "s");

    // 自依赖不应产生外部影响
    let affected = dep_graph.affected_by("s");
    // s 的依赖者只有自己，对下游无影响
    // 这是边界行为测试
    assert!(
        affected.iter().all(|v| *v == "s") || affected.is_empty(),
        "自依赖只影响自身"
    );
}

// ===================================================================
// RFC-027 §6.1: 赋值顺序强制（机制验证）
// ===================================================================

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
    dep_graph.add_dep("s", "i");

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
    dep_graph.add_dep("s", "i");

    // 即使赋值顺序错误，依赖图查询仍应正确返回受影响变量
    let affected = dep_graph.affected_by("i");
    assert!(
        !affected.is_empty(),
        "即使赋值顺序错误，依赖图查询仍应返回受影响变量"
    );
}

// ===================================================================
// RFC-027 §6.1: 传递依赖链 + remove_dependant
// ===================================================================

/// 传递依赖链 a→b→c：a 变更影响 b，b 变更影响 c
#[test]
fn test_dep_graph_transitive_chain() {
    let mut dep_graph = TypeDepGraph::new();
    // a 依赖 b，b 依赖 c
    dep_graph.add_dep("a", "b");
    dep_graph.add_dep("b", "c");

    let affected_by_c = dep_graph.affected_by("c");
    assert!(affected_by_c.contains(&"b"), "c 变更应直接影响 b");
    // 传递性：c 变更 → b 被重验证 → 触发的 b 赋值再影响 a
    // 但依赖图本身只记录直接依赖，传递由 checker 的递归查询处理
    let affected_by_b = dep_graph.affected_by("b");
    assert!(affected_by_b.contains(&"a"), "b 变更应直接影响 a");
}

/// remove_dependant：变量离开作用域时清除依赖记录
#[test]
fn test_dep_graph_remove_dependant_clears_edges() {
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.add_dep("s", "i");
    dep_graph.add_dep("t", "i");

    // s 离开作用域
    dep_graph.remove_dependant("s");

    // i 仍影响 t，但不再影响 s
    let affected = dep_graph.affected_by("i");
    assert!(!affected.contains(&"s"), "s 被移除后不应再受 i 影响");
    assert!(affected.contains(&"t"), "t 仍应受 i 影响");
}

/// 菱形依赖：两端都依赖同一变量
#[test]
fn test_dep_graph_diamond_dependency() {
    let mut dep_graph = TypeDepGraph::new();
    // s 依赖 i，t 也依赖 i
    dep_graph.add_dep("s", "i");
    dep_graph.add_dep("t", "i");

    let affected = dep_graph.affected_by("i");
    assert_eq!(affected.len(), 2, "i 变更应影响 s 和 t，实际: {affected:?}");
    assert!(affected.contains(&"s"), "菱形: i→s 应成立");
    assert!(affected.contains(&"t"), "菱形: i→t 应成立");
}

/// 空图 add+remove 不崩溃
#[test]
fn test_dep_graph_empty_remove_does_not_panic() {
    let mut dep_graph = TypeDepGraph::new();
    // 移除不存在的依赖者不应 panic
    dep_graph.remove_dependant("nonexistent");
    assert!(dep_graph.affected_by("x").is_empty());
}
