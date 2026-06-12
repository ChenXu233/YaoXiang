//! RFC-027 Phase 3 端到端测试：完整证明管道
//!
//! RFC-027 §4：四级分派证明管道端到端验证。
//! RFC-027 §6.1：TypeDepGraph 激活 + VC 生成端到端验证。
//!
//! 测试覆盖：
//! - 完整四级分派路径（Level 1→2a→2b→3→4）
//! - 假设蕴含链路（if-guard → divide 调用验证）
//! - 依赖图构建 + 赋值触发 VC（完整 checker 流程）

use std::collections::HashMap;

use crate::frontend::core::typecheck::layers::predicate::check_predicate;
use crate::frontend::core::typecheck::proof::context::ProofContext;
use crate::frontend::core::typecheck::proof::dep_graph::TypeDepGraph;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::typecheck::TypeEnvironment;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;

// ===================================================================
// RFC-027 §4: 完整四级分派 E2E
// ===================================================================

/// E2E: Level 1 → Proved（绑定变量有具体值，直接求值）
///
/// 模拟 `let x: Positive(x) = 5` — 编译期验证 Positive(5) = { 5 > 0 } → True
#[test]
fn test_e2e_level1_direct_eval_proved() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("x".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(5));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act — 应走 Level 1 直接求值
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    assert!(
        result.is_proved(),
        "E2E Level 1: x=5 时 x>0 应直接求值为 Proved"
    );
}

/// E2E: Level 1 → Disproved（绑定变量使约束为假）
///
/// 模拟 `let x: Positive(x) = 0` — 编译期验证 Positive(0) = { 0 > 0 } → False
#[test]
fn test_e2e_level1_direct_eval_disproved() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("x".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(0));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    assert!(
        matches!(result, ProofResult::Disproved(_)),
        "E2E Level 1: x=0 时 x>0 应求值为 Disproved，实际: {result:?}"
    );
}

/// E2E: Level 2a → Proved（约束正好在假设栈中，零开销精确匹配）
///
/// 模拟 `if y > 0 { divide(x, y) }` — 编译器从 if-guard 压入 y>0，约束匹配
#[test]
fn test_e2e_level2a_exact_match_from_if_guard() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    // 模拟 if y > 0 分支内的假设栈
    let guard_assumption = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.push(guard_assumption);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 应走 Level 2a 精确匹配，零 Z3 开销
    assert!(
        result.is_proved(),
        "E2E Level 2a: if y>0 分支内约束 y>0 应精确匹配 Proved"
    );
}

/// E2E: Level 2b → Proved（强假设蕴含弱约束，SMT 验证）
///
/// 模拟 `if y >= 5 { divide(x, y) }` — 编译器知道 y>=5，需 SMT 判蕴含 y>0
#[test]
fn test_e2e_level2b_implication_from_if_guard() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    // 模拟 if y >= 5 分支内
    let guard = ConstExpr::BinOp {
        op: BinOp::Ge,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
    };

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.push(guard);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 应走 Level 2b SMT 蕴含
    assert!(
        result.is_proved(),
        "E2E Level 2b: if y>=5 分支内约束 y>0 应由 SMT 蕴含 Proved，实际: {result:?}"
    );
}

/// E2E: Level 3 → Disproved（无假设，约束独立不成立，Z3 找到反例）
///
/// 模拟 `divide(x, y)` 无 if-guard — 编译器无法证明 y>0，Z3 找反例
#[test]
fn test_e2e_level3_no_assumptions_finds_counterexample() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };

    // 无假设栈 — 模拟无 if-guard 的直接调用
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 应走 Level 3 SMT，找到反例 y=0
    assert!(
        matches!(result, ProofResult::Disproved(_)),
        "E2E Level 3: 无假设时 y>0 应由 Z3 找到反例 Disproved，实际: {result:?}"
    );
}

/// E2E: 全链路穿透 — 假设不蕴含时 Level 2b→3 降级
///
/// 模拟 `if z > 0 { divide(x, y) }` — z>0 不蕴含 y>0，Level 2b 返回 None → Level 3
#[test]
fn test_e2e_level2b_fallback_to_level3() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    // 无关假设
    let irrelevant_guard = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("z".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.push(irrelevant_guard);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — Level 2b 返回 None → Level 3 找反例
    assert!(
        matches!(result, ProofResult::Disproved(_)),
        "E2E 全链路: 无关假设 z>0 不蕴含 y>0 → 2b→3 降级 → Disproved，实际: {result:?}"
    );
}

// ===================================================================
// RFC-027 §6.1: TypeDepGraph 激活 E2E
// ===================================================================

/// E2E: TypeDepGraph 构建 + 赋值触发影响分析
///
/// 模拟 `mut s: SumUpTo(arr, i)` 标注依赖 `i`，`i += 1` 触发 `s` 重验证
#[test]
fn test_e2e_dep_graph_assignment_triggers_affected_by() {
    // Arrange — 构建依赖图
    let mut dep_graph = TypeDepGraph::new();
    // s 的类型标注 SumUpTo(arr, i) 引用了 i 和 arr
    dep_graph.add_dep("s", "i");
    dep_graph.add_dep("s", "arr");

    // Act — i 被赋值，查询受影响变量
    let affected = dep_graph.affected_by("i");

    // Assert
    assert!(affected.contains(&"s"), "E2E: i 变更应触发 s 的重验证");
    assert_eq!(affected.len(), 1, "E2E: i 变更应只影响 s");
}

/// E2E: 无依赖变量赋值不触发 VC
#[test]
fn test_e2e_no_dependency_no_vc_triggered() {
    // Arrange
    let dep_graph = TypeDepGraph::new();

    // Act — x 被赋值，但无人依赖 x
    let affected = dep_graph.affected_by("x");

    // Assert
    assert!(affected.is_empty(), "E2E: 无依赖时赋值不触发任何 VC");
}

/// E2E: 组合依赖 — 一个变量依赖多个被依赖变量
#[test]
fn test_e2e_combined_dependency_multiple_triggers() {
    // Arrange — t: BoundedBy(i, j) 同时依赖 i 和 j
    let mut dep_graph = TypeDepGraph::new();
    dep_graph.add_dep("t", "i");
    dep_graph.add_dep("t", "j");

    // Act & Assert
    let affected_by_i = dep_graph.affected_by("i");
    assert!(affected_by_i.contains(&"t"), "E2E: i 变更应触发 t 重验证");

    let affected_by_j = dep_graph.affected_by("j");
    assert!(affected_by_j.contains(&"t"), "E2E: j 变更也应触发 t 重验证");

    // j 变更不影响其他变量
    assert_eq!(affected_by_j.len(), 1, "E2E: j 变更应只影响 t");
}

// ===================================================================
// RFC-027 §4.2: Level 4 证明函数调用 + 错误路径验证
// ===================================================================

/// E2E: Disproved 反例包含具体变量赋值
#[test]
fn test_e2e_disproved_counterexample_contains_variable_values() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 反例模型应包含变量的具体值
    match result {
        ProofResult::Disproved(model) => {
            assert!(
                !model.assignments.is_empty(),
                "反例模型应包含至少一个变量赋值"
            );
            assert!(
                model.assignments.iter().any(|(k, _)| k == "y"),
                "反例模型应包含变量 y，实际: {:?}",
                model.assignments.iter().map(|(k, _)| k).collect::<Vec<_>>()
            );
        }
        other => panic!("期望 Disproved 获取反例详情，实际: {other:?}"),
    }
}

/// E2E: Unproven 原因可被读取
#[test]
fn test_e2e_unproven_reason_readable() {
    // Arrange — 使用 Call 形式让 Level 3 跳过，Level 4 可能返回 Unproven
    let call_constraint = ConstExpr::Call {
        func: "NonExistentPredicate".into(),
        args: vec![ConstExpr::NamedVar("x".into())],
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: call_constraint,
    };
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 应为 Unproven（Level 4 无法解析非存在证明函数）
    assert!(
        !result.is_proved(),
        "不存在的证明函数应返回 Unproven，实际: {result:?}"
    );
    assert!(
        matches!(result, ProofResult::Unproven { .. }),
        "期望 Unproven，实际: {result:?}"
    );
}

/// E2E: 假设蕴含 + Disproved 交互 —— 有假设但不是精确约束时正确降级
#[test]
fn test_e2e_assumption_not_exact_still_triggers_implication_check() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    // 假设 y > 5，约束 y > 10 —— contains() 不匹配，走 implication
    // y=6 满足 y>5 但不满足 y>10 → 不蕴含
    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.push(ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
    });

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — y > 5 不蕴含 y > 10（如 y=6），Level 2b sat → None → Level 3
    assert!(
        matches!(result, ProofResult::Disproved(_)),
        "y>5 不蕴含 y>10，应退到 Level 3 找反例，实际: {result:?}"
    );
}

/// E2E: 等价约束通过精确匹配 —— y > 0 在栈中，约束也是 y > 0
#[test]
fn test_e2e_exact_equivalent_constraint_matched() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.push(ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    });

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — contains() 精确匹配，零 SMT 开销
    assert!(
        result.is_proved(),
        "精确匹配应走 Level 2a 直接 Proved，实际: {result:?}"
    );
}
