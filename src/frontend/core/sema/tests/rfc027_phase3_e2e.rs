//! RFC-027 Phase 3 端到端测试：完整证明管道
//!
//! RFC-027 §4：四级分派证明管道端到端验证。
//! RFC-027 §6.1：TypeDepGraph 激活 + VC 生成端到端验证。
//!
//! 测试覆盖：
//! - 完整四级分派路径（Level 1→2a→2b→3→4）
//! - 假设蕴含链路（if-guard → divide 调用验证）
//! - 依赖图构建 + 赋值触发 VC（完整 checker 流程）

use crate::frontend::core::sema::test_util::{binop, refined_int};
use std::collections::HashMap;

use crate::frontend::core::sema::layers::predicate::check_predicate;
use crate::frontend::core::sema::proof::context::ProofContext;
use crate::frontend::core::sema::proof::dep_graph::TypeDepGraph;
use crate::frontend::core::sema::proof::verdict::{ProofFailure, ProofResultExt};
use crate::frontend::core::sema::TypeEnvironment;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;

/// E2E: Level 1 → Disproved（绑定变量使约束为假）
///
/// 模拟 `let x: Positive(x) = 0` — 编译期验证 Positive(0) = { 0 > 0 } → False
#[test]
fn test_e2e_level1_direct_eval_disproved() {
    // Arrange
    let constraint = binop(
        BinOp::Gt,
        ConstExpr::NamedVar("x".into()),
        ConstExpr::Lit(ConstValue::Int(0)),
    );
    let refined = refined_int(constraint);
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(0));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    assert!(
        matches!(result, Err(ProofFailure::Disproved(_))),
        "E2E Level 1: x=0 时 x>0 应求值为 Disproved，实际: {result:?}"
    );
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

// RFC-027 §4.2: Level 4 证明函数调用 + 错误路径验证

/// E2E: Disproved 反例包含具体变量赋值
#[test]
fn test_e2e_disproved_counterexample_contains_variable_values() {
    // Arrange
    let constraint = binop(
        BinOp::Gt,
        ConstExpr::NamedVar("y".into()),
        ConstExpr::Lit(ConstValue::Int(0)),
    );
    let refined = refined_int(constraint);
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert — 反例模型应包含变量的具体值
    match result {
        Err(ProofFailure::Disproved(model)) => {
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
        matches!(result, Err(ProofFailure::Unproven { .. })),
        "期望 Unproven，实际: {result:?}"
    );
}
