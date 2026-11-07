//! RFC-027 §4/§8 Phase 2 SMT 集成测试
//!
//! 测试三级分派：
//!   §4.1: Evaluator 直接求值（Phase 1）
//!   §3.2: 假设栈蕴含（Phase 2A）
//!   §8:   Z3 SMT 求解（Phase 2B）

use crate::frontend::core::sema::test_util::{binop, refined_int};
use std::collections::HashMap;

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::sema::layers::predicate::check_predicate;
use crate::frontend::core::sema::proof::context::ProofContext;
use crate::frontend::core::sema::proof::verdict::ProofResult;
use crate::frontend::core::sema::TypeEnvironment;

/// 辅助：构造 GT 约束 (var > n)
fn constraint_gt(
    var: &str,
    n: i128,
) -> ConstExpr {
    binop(
        BinOp::Gt,
        ConstExpr::NamedVar(var.into()),
        ConstExpr::Lit(ConstValue::Int(n)),
    )
}

/// 辅助：构造 Refined 类型
// =========== §4.1: Evaluator 直接求值（Phase 1 回归） ===========

#[test]
fn test_direct_eval_proved_for_true_literal_comparison() {
    let refined = refined_int(binop(
        BinOp::Gt,
        ConstExpr::Lit(ConstValue::Int(5)),
        ConstExpr::Lit(ConstValue::Int(0)),
    ));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());
    assert!(result.is_proved(), "纯字面量 5>0 应直接求值为 Proved");
}

// =========== §3.2: 假设栈蕴含 ===========

// =========== §8: Z3 SMT 求解 ===========

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
