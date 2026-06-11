//! Layer 3：精化谓词证明（RFC-027 Section 3）
//!
//! 对 Refined 类型的约束表达式进行求值。
//! 阶段 1 只做直接求值——不做假设栈蕴含推理（阶段 2+SMT）。

use std::collections::HashMap;

use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::types::const_data::ConstValue;
use crate::frontend::core::types::eval::evaluator::Evaluator;
use super::super::proof::verdict::{DisproofModel, ProofResult, UnprovenReason};
use super::super::proof::context::ProofContext;

/// 检查精化谓词是否成立
///
/// 从 Refined 类型提取 constraint，代入 bindings，调用 Evaluator::eval_expr 求值。
///
/// # 参数
/// - `ctx`: 证明上下文（携带 env 和 budget）
/// - `refined`: 精化类型（MonoType::Refined { base, constraint }）
/// - `bindings`: 变量名 → 具体值的映射（如 { "b": Int(2) }）
///
/// # 返回
/// - `Proved`: 约束求值为 true
/// - `Disproved`: 约束求值为 false，携带反例
/// - `Unproven`: 求值失败或超预算
pub fn check_predicate(
    ctx: &ProofContext<'_>,
    refined: &MonoType,
    bindings: &HashMap<String, ConstValue>,
) -> ProofResult {
    // 提取约束表达式
    let constraint = match refined {
        MonoType::Refined { constraint, .. } => constraint,
        _ => return ProofResult::Proved, // 不是精化类型，无事可证
    };

    // 求值约束
    let mut evaluator = Evaluator::new(ctx.env, &ctx.budget);
    match evaluator.eval_expr(constraint, bindings) {
        Ok(ConstValue::Bool(true)) => ProofResult::Proved,
        Ok(ConstValue::Bool(false)) => ProofResult::Disproved(DisproofModel {
            assignments: bindings
                .iter()
                .map(|(k, v)| (k.clone(), format!("{:?}", v)))
                .collect(),
        }),
        Ok(_) => ProofResult::Unproven {
            reason: UnprovenReason::BeyondKernel("约束表达式未求值为 Bool".into()),
            budget: ctx.budget.report(),
        },
        Err(e) => ProofResult::Unproven {
            reason: UnprovenReason::BeyondKernel(format!("谓词求值失败: {:?}", e)),
            budget: ctx.budget.report(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_predicate_true() {
        use crate::frontend::core::types::const_data::{BinOp, ConstExpr};
        use crate::frontend::core::typecheck::TypeEnvironment;

        let refined = MonoType::Refined {
            base: Box::new(MonoType::Int(64)),
            constraint: ConstExpr::BinOp {
                op: BinOp::Gt,
                left: Box::new(ConstExpr::NamedVar("b".into())),
                right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
            },
        };
        let mut bindings = HashMap::new();
        bindings.insert("b".into(), ConstValue::Int(5));

        let env = TypeEnvironment::new();
        let ctx = ProofContext::new(&env);
        let result = check_predicate(&ctx, &refined, &bindings);
        assert!(result.is_proved());
    }

    #[test]
    fn test_check_predicate_false() {
        use crate::frontend::core::types::const_data::{BinOp, ConstExpr};
        use crate::frontend::core::typecheck::TypeEnvironment;

        let refined = MonoType::Refined {
            base: Box::new(MonoType::Int(64)),
            constraint: ConstExpr::BinOp {
                op: BinOp::Gt,
                left: Box::new(ConstExpr::NamedVar("b".into())),
                right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
            },
        };
        let mut bindings = HashMap::new();
        bindings.insert("b".into(), ConstValue::Int(0));

        let env = TypeEnvironment::new();
        let ctx = ProofContext::new(&env);
        let result = check_predicate(&ctx, &refined, &bindings);
        assert!(!result.is_proved());
        match result {
            ProofResult::Disproved(model) => {
                assert!(model.assignments.iter().any(|(k, _)| k == "b"));
            }
            _ => panic!("Expected Disproved"),
        }
    }

    #[test]
    fn test_check_predicate_non_refined_passes() {
        use crate::frontend::core::typecheck::TypeEnvironment;

        let non_refined = MonoType::Int(64);
        let bindings = HashMap::new();
        let env = TypeEnvironment::new();
        let ctx = ProofContext::new(&env);
        let result = check_predicate(&ctx, &non_refined, &bindings);
        assert!(result.is_proved()); // 非 Refined 类型直接通过
    }
}
