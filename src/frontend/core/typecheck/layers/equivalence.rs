//! Layer 0：类型等式证明
//!
//! 检查两个类型是否等价。调用 types/eval/ 做确定性归约，
//! 将归约结果包装为 ProofResult。

use crate::frontend::core::types::eval::evaluator::TypeEvaluator;
use crate::frontend::core::types::mono::MonoType;
use super::super::proof::verdict::{BudgetReport, DisproofModel, ProofResult, UnprovenReason};
use super::super::proof::context::ProofContext;

/// 检查类型等式
pub fn check_type_equivalence(
    ctx: &ProofContext<'_>,
    lhs: &MonoType,
    rhs: &MonoType,
) -> ProofResult {
    let mut evaluator = TypeEvaluator::new();
    evaluator.set_env(ctx.env_ref());

    match (evaluator.eval(lhs), evaluator.eval(rhs)) {
        (Ok(l), Ok(r)) if l == r => ProofResult::Proved,
        (Ok(l), Ok(r)) => ProofResult::Disproved(DisproofModel {
            assignments: vec![
                ("lhs".into(), format!("{:?}", l)),
                ("rhs".into(), format!("{:?}", r)),
            ],
        }),
        (Err(e), _) | (_, Err(e)) => ProofResult::Unproven {
            reason: UnprovenReason::BeyondKernel(format!("Type eval error: {:?}", e)),
            budget: BudgetReport {
                steps_used: 0,
                steps_limit: 0,
            },
        },
    }
}
