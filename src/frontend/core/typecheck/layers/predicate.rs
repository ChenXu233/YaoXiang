//! Layer 3：精化谓词证明 — 基于 RFC-027 §4
//!
//! 对 Refined 类型的约束表达式进行求值。
//! 四级分派：
//!   1. Evaluator 直接求值（Phase 1，微秒级）——所有变量有具体值
//!   2. 假设栈蕴含（Phase 2A，零开销）——约束正好是已知条件
//!   3. Z3 SMT 求解（Phase 2B，毫秒级）——符号变量 + 蕴含推理
//!   4. 证明函数调用（Phase 2.5）——识别 ConstExpr::Call 让 Pipeline 编译期执行

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
use crate::frontend::core::types::eval::evaluator::Evaluator;
use crate::frontend::core::types::mono::MonoType;
use super::super::proof::context::ProofContext;
use super::super::proof::smt::ast::SMTResult;
use super::super::proof::smt::translate;
use super::super::proof::smt::z3_backend::Z3Backend;
use super::super::proof::verdict::{BudgetReport, DisproofModel, ProofFunctionCall, ProofResult, UnprovenReason};

/// Z3 实例——整个编译过程只初始化一次
static Z3_INSTANCE: LazyLock<Mutex<Z3Backend>> = LazyLock::new(|| {
    Mutex::new(Z3Backend::new().expect("Z3 solver initialization failed — is libz3 installed?"))
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
    // SMT 翻译不支持 Call/If/Range 形式——跳过，直接进入第 4 级
    if !matches!(
        constraint,
        ConstExpr::Call { .. } | ConstExpr::If { .. } | ConstExpr::Range { .. }
    ) {
        let smt_result = try_smt_solve(ctx, constraint, bindings);
        if smt_result.is_proved() || matches!(smt_result, ProofResult::Disproved(_)) {
            return smt_result;
        }
    }

    // === 第 4 级：识别证明函数调用 ===
    try_proof_fn_call(constraint, bindings, ctx.budget.report())
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
            proof_calls: vec![],
            budget: ctx.budget.report(),
        }),
        Err(_) => None,
    }
}

/// 第 3 级：SMT 求解
fn try_smt_solve(
    ctx: &ProofContext<'_>,
    constraint: &ConstExpr,
    bindings: &HashMap<String, ConstValue>,
) -> ProofResult {
    let var_sorts = translate::infer_var_sorts(constraint, bindings);
    let commands =
        translate::translate_constraint(constraint, ctx.assumptions.current(), &var_sorts);

    match Z3_INSTANCE
        .lock()
        .unwrap()
        .solve(&commands, ctx.budget.time_ms_limit())
    {
        SMTResult::Unsat => ProofResult::Proved,
        SMTResult::Sat { model } => ProofResult::Disproved(DisproofModel {
            assignments: model.assignments,
        }),
        SMTResult::Unknown { reason } => ProofResult::Unproven {
            reason: UnprovenReason::BeyondKernel(reason),
            proof_calls: vec![],
            budget: ctx.budget.report(),
        },
    }
}

/// 第 4 级：识别证明函数调用（Phase 2.5）
///
/// 当约束是函数调用形式（如 Sorted(arr)）且前三级无法证明时，
/// 构造 ProofFunctionCall 让 Pipeline 编译期执行。
fn try_proof_fn_call(
    constraint: &ConstExpr,
    bindings: &HashMap<String, ConstValue>,
    budget_report: BudgetReport,
) -> ProofResult {
    if let ConstExpr::Call { func, args } = constraint {
        // 将 ConstExpr args 转为 ConstValue
        let mut const_args: Vec<ConstValue> = Vec::with_capacity(args.len());
        for a in args {
            match a {
                ConstExpr::Lit(v) => const_args.push(v.clone()),
                ConstExpr::NamedVar(name) => {
                    if let Some(val) = bindings.get(name) {
                        const_args.push(val.clone());
                    } else {
                        return ProofResult::Unproven {
                            reason: UnprovenReason::BeyondKernel(
                                "证明函数实参包含未绑定变量".into(),
                            ),
                            proof_calls: vec![],
                            budget: budget_report,
                        };
                    }
                }
                _ => {
                    return ProofResult::Unproven {
                        reason: UnprovenReason::BeyondKernel(
                            "证明函数实参必须是字面量或已绑定变量".into(),
                        ),
                        proof_calls: vec![],
                        budget: budget_report,
                    };
                }
            }
        }

        return ProofResult::Unproven {
            reason: UnprovenReason::ProofFunctionRequired,
            proof_calls: vec![ProofFunctionCall {
                func_name: func.clone(),
                args: const_args,
            }],
            budget: budget_report,
        };
    }

    ProofResult::Unproven {
        reason: UnprovenReason::BeyondKernel("无法自动证明且无证明函数".into()),
        proof_calls: vec![],
        budget: budget_report,
    }
}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
    use crate::frontend::core::typecheck::layers::predicate::check_predicate;
    use crate::frontend::core::typecheck::proof::context::ProofContext;
    use crate::frontend::core::typecheck::proof::verdict::ProofResult;
    use crate::frontend::core::typecheck::TypeEnvironment;
    use crate::frontend::core::types::mono::MonoType;
    use std::collections::HashMap;

    // =========== Phase 1: Evaluator 直接求值 ===========

    #[test]
    fn test_direct_eval_with_bound_variable_proved() {
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
        assert!(result.is_proved(), "b=5 时 b>0 应直接求值为 Proved");
    }

    #[test]
    fn test_direct_eval_with_bound_variable_disproved() {
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
        assert!(!result.is_proved(), "b=0 时 b>0 应直接求值为 Disproved");
        match result {
            ProofResult::Disproved(model) => {
                assert!(
                    model.assignments.iter().any(|(k, _)| k == "b"),
                    "反例模型应包含变量 b"
                );
            }
            other => panic!("期望 Disproved，实际: {other:?}"),
        }
    }

    #[test]
    fn test_non_refined_type_passes_immediately() {
        let env = TypeEnvironment::new();
        let ctx = ProofContext::new(&env);
        let result = check_predicate(&ctx, &MonoType::Int(64), &HashMap::new());
        assert!(result.is_proved(), "非 Refined 类型应直接返回 Proved");
    }

    // =========== Phase 2A: 假设栈蕴含 ===========

    #[test]
    fn test_assumption_stack_direct_match_proves_immediately() {
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
        assert!(result.is_proved(), "约束正好在假设栈中应直接返回 Proved");
    }

    #[test]
    fn test_direct_eval_with_concrete_literals() {
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
        assert!(result.is_proved(), "5>0 纯字面量应直接求值为 Proved");
    }
}
