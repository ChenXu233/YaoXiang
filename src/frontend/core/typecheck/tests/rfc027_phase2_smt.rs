//! RFC-027 Phase 2 SMT 集成测试
//!
//! 测试三级分派：
//!   1. Evaluator 直接求值（Phase 1，继续通过）
//!   2. 假设栈蕴含（Phase 2A）
//!   3. Z3 SMT 求解（Phase 2B，需要 --features z3）

use std::collections::HashMap;

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::typecheck::layers::predicate::check_predicate;
use crate::frontend::core::typecheck::proof::context::ProofContext;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::typecheck::TypeEnvironment;

/// 辅助：构造 GT 约束 (var > n)
fn constraint_gt(var: &str, n: i128) -> ConstExpr {
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

// =========== 第 1 级：Evaluator 直接求值 ==========

#[test]
fn phase1_direct_eval_true() {
    let refined = refined_int(ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    });

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());
    assert!(result.is_proved(), "5 > 0 should be proved directly");
}

#[test]
fn phase1_direct_eval_false() {
    let refined = refined_int(ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    });

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());
    assert!(!result.is_proved(), "0 > 0 should be disproved directly");
}

// =========== 第 2 级：假设栈蕴含 ==========

#[test]
fn assumption_stack_direct_match() {
    let cond = constraint_gt("y", 0);
    let refined = refined_int(cond.clone());

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.push(cond);

    let mut bindings = HashMap::new();
    bindings.insert("y".into(), ConstValue::Int(5));

    // 约束在假设栈中 → 直接 Proved
    let result = check_predicate(&ctx, &refined, &bindings);
    assert!(result.is_proved());
}

#[test]
fn assumption_stack_no_match() {
    let in_stack = constraint_gt("y", 0);
    let constraint = constraint_gt("y", 5);
    let refined = refined_int(constraint);

    let env = TypeEnvironment::new();
    let mut ctx = ProofContext::new(&env);
    ctx.assumptions.push(in_stack);

    // y > 5 ≠ y > 0，不匹配假设栈
    // 但 bindings 给 y=5 → Evaluator 求值 5 > 5 → false
    let mut bindings = HashMap::new();
    bindings.insert("y".into(), ConstValue::Int(5));

    let result = check_predicate(&ctx, &refined, &bindings);
    assert!(!result.is_proved());
}

// =========== Z3 不可用时回退 ==========

#[test]
fn no_z3_fallback_to_unproven() {
    let refined = refined_int(constraint_gt("unknown_var", 0));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // 无 Z3 时，含符号变量的约束 → Unproven
    assert!(!result.is_proved());
    match result {
        ProofResult::Unproven { .. } => {}
        _ => panic!("Expected Unproven for symbolic constraint without Z3"),
    }
}
