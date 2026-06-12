//! RFC-027 §4/§8 Phase 2 SMT 集成测试
//!
//! 测试三级分派：
//!   §4.1: Evaluator 直接求值（Phase 1）
//!   §3.2: 假设栈蕴含（Phase 2A）
//!   §8:   Z3 SMT 求解（Phase 2B）

use std::collections::HashMap;

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::typecheck::layers::predicate::check_predicate;
use crate::frontend::core::typecheck::proof::context::ProofContext;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::typecheck::TypeEnvironment;

/// 辅助：构造 GT 约束 (var > n)
fn constraint_gt(
    var: &str,
    n: i128,
) -> ConstExpr {
    ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar(var.into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(n))),
    }
}

/// 辅助：构造 Refined 类型
fn refined_int(constraint: ConstExpr) -> MonoType {
    MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    }
}

// =========== §4.1: Evaluator 直接求值（Phase 1 回归） ===========

#[test]
fn test_direct_eval_proved_for_true_literal_comparison() {
    let refined = refined_int(ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    });

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());
    assert!(result.is_proved(), "纯字面量 5>0 应直接求值为 Proved");
}

#[test]
fn test_direct_eval_disproved_for_false_literal_comparison() {
    let refined = refined_int(ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    });

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());
    assert!(!result.is_proved(), "纯字面量 0>0 应求值为 Disproved");
}

// =========== §3.2: 假设栈蕴含 ===========

#[test]
fn test_assumption_stack_match_proves_without_evaluator_or_smt() {
    let cond = constraint_gt("y", 0);
    let refined = refined_int(cond.clone());

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.push(cond);

    // 约束在假设栈中 → 直接 Proved，不经过 Evaluator 和 Z3
    let result = check_predicate(&ctx, &refined, &HashMap::new());
    assert!(result.is_proved(), "约束 y>0 在假设栈中应直接 Proved");
}

#[test]
fn test_assumption_stack_no_match_falls_through_to_evaluator() {
    let in_stack = constraint_gt("y", 0);
    let constraint = constraint_gt("y", 5);
    let refined = refined_int(constraint);

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.push(in_stack);

    let mut bindings = HashMap::new();
    bindings.insert("y".into(), ConstValue::Int(5));

    // y>5 ≠ y>0，不匹配假设栈 → 回退到 Evaluator
    let result = check_predicate(&ctx, &refined, &bindings);
    assert!(
        !result.is_proved(),
        "y>5 不在假设栈中，Evaluator 求值 5>5 → false → Disproved"
    );
}

// =========== §8: Z3 SMT 求解 ===========

#[test]
fn test_smt_implication_from_assumptions_to_weaker_constraint() {
    // 假设: y > 5
    // 目标: y > 0
    // Z3 应证明蕴含: y > 5 ⇒ y > 0
    let refined = refined_int(constraint_gt("y", 0));

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.push(constraint_gt("y", 5));

    let result = check_predicate(&ctx, &refined, &HashMap::new());
    assert!(result.is_proved(), "Z3 应证明 y>5 ⇒ y>0");
}

#[test]
fn test_smt_symbolic_variable_without_binding_detects_counterexample() {
    // 目标: y > 0，无 bindings
    // Z3: (not (> y 0)) → sat (y=0 是反例)
    let refined = refined_int(constraint_gt("y", 0));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    let result = check_predicate(&ctx, &refined, &HashMap::new());
    assert!(!result.is_proved(), "Z3 应找到 y>0 的反例 (如 y=0)");
    match result {
        ProofResult::Disproved(model) => {
            assert!(
                !model.assignments.is_empty(),
                "反例模型应包含至少一个赋值: {model:?}"
            );
        }
        other => panic!("期望 Disproved，实际: {other:?}"),
    }
}

#[test]
fn test_smt_linear_arithmetic_with_concrete_binding() {
    // 约束: x > 0 && x < 10
    // bindings: x = 5
    let constraint = ConstExpr::BinOp {
        op: BinOp::And,
        left: Box::new(constraint_gt("x", 0)),
        right: Box::new(ConstExpr::BinOp {
            op: BinOp::Lt,
            left: Box::new(ConstExpr::NamedVar("x".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
        }),
    };
    let refined = refined_int(constraint);

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(5));

    let result = check_predicate(&ctx, &refined, &bindings);
    assert!(result.is_proved(), "x=5 应满足 x>0 && x<10");
}
