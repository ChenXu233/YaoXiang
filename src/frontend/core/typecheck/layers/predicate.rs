//! Layer 3：精化谓词证明（RFC-027 Section 3）
//!
//! 对 Refined 类型的约束表达式进行求值。
//! 三级分派：
//!   1. Evaluator 直接求值（Phase 1，微秒级）——所有变量有具体值
//!   2. 假设栈蕴含（Phase 2A，零开销）——约束正好是已知条件
//!   3. Z3 SMT 求解（Phase 2B，毫秒级）——符号变量 + 蕴含推理

use std::collections::HashMap;

use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
use crate::frontend::core::types::eval::evaluator::Evaluator;
use crate::frontend::core::types::mono::MonoType;
use super::super::proof::context::ProofContext;
use super::super::proof::verdict::{DisproofModel, ProofResult, UnprovenReason};

#[cfg(feature = "z3")]
use super::super::proof::smt::translate;
#[cfg(feature = "z3")]
use super::super::proof::smt::ast::SMTResult;
#[cfg(feature = "z3")]
use super::super::proof::smt::z3_backend::Z3Backend;
#[cfg(feature = "z3")]
use std::sync::LazyLock;

/// Z3 实例——整个编译过程只初始化一次
#[cfg(feature = "z3")]
static Z3_INSTANCE: LazyLock<Option<Z3Backend>> = LazyLock::new(|| {
    match Z3Backend::new() {
        Ok(z3) => Some(z3),
        Err(e) => {
            eprintln!("[yaoxiang] Z3 init failed: {} — falling back to evaluator-only mode", e);
            None
        }
    }
});

/// 检查精化谓词是否成立
///
/// # 参数
/// - `ctx`: 证明上下文（携带 env、assumptions、budget）
/// - `refined`: 精化类型（MonoType::Refined { base, constraint }）
/// - `bindings`: 变量名 → 具体值的映射（如 { "b": Int(2) }）
///
/// # 返回
/// - `Proved`: 约束成立
/// - `Disproved`: 约束不成立，携带反例
/// - `Unproven`: 无法证明（超出能力或超预算）
pub fn check_predicate(
    ctx: &ProofContext<'_>,
    refined: &MonoType,
    bindings: &HashMap<String, ConstValue>,
) -> ProofResult {
    // 提取约束表达式
    let constraint = match refined {
        MonoType::Refined { constraint, .. } => constraint,
        _ => return ProofResult::Proved,
    };

    // === 第 1 级：Evaluator 直接求值 ===
    if let Some(result) = try_direct_eval(ctx, constraint, bindings) {
        return result;
    }

    // === 第 2 级：假设栈蕴含 ===
    if ctx.assumptions.contains(constraint) {
        return ProofResult::Proved;
    }

    // === 第 3 级：SMT 求解 ===
    try_smt_solve(ctx, constraint, bindings)
}

/// 第 1 级：Evaluator 直接求值
///
/// 所有变量有具体值时能直接算出结果。返回 None 表示求值失败（有未绑定变量），
/// 需要升级到后续级别。
fn try_direct_eval(
    ctx: &ProofContext<'_>,
    constraint: &ConstExpr,
    bindings: &HashMap<String, ConstValue>,
) -> Option<ProofResult> {
    let mut evaluator = Evaluator::new(ctx.env, &ctx.budget);
    match evaluator.eval_expr(constraint, bindings) {
        Ok(ConstValue::Bool(true)) => Some(ProofResult::Proved),
        Ok(ConstValue::Bool(false)) => Some(ProofResult::Disproved(DisproofModel {
            assignments: bindings
                .iter()
                .map(|(k, v)| (k.clone(), format!("{:?}", v)))
                .collect(),
        })),
        Ok(_) => Some(ProofResult::Unproven {
            reason: UnprovenReason::BeyondKernel("约束表达式未求值为 Bool".into()),
            budget: ctx.budget.report(),
        }),
        Err(_) => {
            // 求值失败（如未绑定变量）→ 不是 Proved/Disproved，
            // 返回 None 让上层继续尝试后续级别
            None
        }
    }
}

/// 第 3 级：SMT 求解
#[allow(unused_variables)]
fn try_smt_solve(
    ctx: &ProofContext<'_>,
    constraint: &ConstExpr,
    bindings: &HashMap<String, ConstValue>,
) -> ProofResult {
    #[cfg(feature = "z3")]
    {
        if let Some(z3) = Z3_INSTANCE.as_ref() {
            let var_sorts = translate::infer_var_sorts(constraint, bindings);
            let commands = translate::translate_constraint(
                constraint,
                ctx.assumptions.current(),
                &var_sorts,
            );

            match z3.solve(&commands, ctx.budget.time_ms_limit()) {
                SMTResult::Unsat => ProofResult::Proved,
                SMTResult::Sat { model } => ProofResult::Disproved(DisproofModel {
                    assignments: model.assignments,
                }),
                SMTResult::Unknown { reason } => ProofResult::Unproven {
                    reason: UnprovenReason::BeyondKernel(reason),
                    budget: ctx.budget.report(),
                },
            }
        } else {
            ProofResult::Unproven {
                reason: UnprovenReason::BeyondKernel(
                    "Z3 solver unavailable — install Z3 or enable the 'z3' feature".into(),
                ),
                budget: ctx.budget.report(),
            }
        }
    }

    #[cfg(not(feature = "z3"))]
    {
        ProofResult::Unproven {
            reason: UnprovenReason::BeyondKernel(
                "SMT solver not compiled in — rebuild with --features z3".into(),
            ),
            budget: ctx.budget.report(),
        }
    }
}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr};
    use crate::frontend::core::typecheck::TypeEnvironment;

    // --- Phase 1 tests (keep existing) ---

    #[test]
    fn test_check_predicate_true() {
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
        let non_refined = MonoType::Int(64);
        let bindings = HashMap::new();
        let env = TypeEnvironment::new();
        let ctx = ProofContext::new(&env);
        let result = check_predicate(&ctx, &non_refined, &bindings);
        assert!(result.is_proved());
    }

    // --- Phase 2A tests (assumption stack) ---

    #[test]
    fn test_assumption_stack_contains_constraint() {
        let constraint = ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar("y".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        };

        let refined = MonoType::Refined {
            base: Box::new(MonoType::Int(64)),
            constraint: constraint.clone(),
        };

        let env = TypeEnvironment::new();
        let mut ctx = ProofContext::new(&env);
        ctx.assumptions.push(constraint);

        let result = check_predicate(&ctx, &refined, &HashMap::new());
        assert!(result.is_proved());
    }

    #[test]
    fn test_direct_eval_with_literals() {
        let refined = MonoType::Refined {
            base: Box::new(MonoType::Int(64)),
            constraint: ConstExpr::BinOp {
                op: BinOp::Gt,
                left: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
                right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
            },
        };

        let env = TypeEnvironment::new();
        let ctx = ProofContext::new(&env);
        let result = check_predicate(&ctx, &refined, &HashMap::new());
        assert!(result.is_proved());
    }
}
