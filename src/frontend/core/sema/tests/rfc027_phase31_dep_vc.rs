//! RFC-027 Phase 3.1 集成测试：TypeDepGraph 注册 / 查询 / 清除
//!
//! RFC-027 §6.1：当被依赖变量变更时，编译器重验证依赖变量的精化类型。
//!
//! #379 微重构后依赖图经 `register_refined` 注册（约束本体 + 自由变量建边），
//! 测试用真实 `Refined` 类型驱动——不再是「自测自己的桩」（#377 的教训：
//! 旧测试直接 `add_dep` 打边，生产路径坏了测试照样绿）。

use crate::frontend::core::sema::proof::dep_graph::TypeDepGraph;
use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr};

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

/// 空图 remove 不崩溃
#[test]
fn test_dep_graph_empty_remove_does_not_panic() {
    let mut dep_graph = TypeDepGraph::new();
    // 移除不存在的依赖者不应 panic
    dep_graph.remove_dependant("nonexistent");
    assert!(dep_graph.affected_by("x").is_empty());
}
