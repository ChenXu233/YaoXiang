//! Layer 3：精化谓词证明 — 基于 RFC-027 §4
//!
//! 对 Refined 类型的约束表达式进行求值。
//! 四级分派：
//!   1. Evaluator 直接求值（Phase 1，微秒级）——所有变量有具体值
//!   2. 假设栈蕴含（Phase 2A + 3.2）——精确匹配 + SMT 蕴含推理
//!   3. Z3 SMT 求解（Phase 2B，毫秒级）——符号变量 + 蕴含推理
//!   4. 证明函数调用（Phase 2.5）——识别 ConstExpr::Call 让 Pipeline 编译期执行

use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{LazyLock, Mutex};

use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
use crate::frontend::core::types::eval::evaluator::Evaluator;
use crate::frontend::core::types::mono::MonoType;
use super::super::proof::context::ProofContext;
#[cfg(not(target_arch = "wasm32"))]
use super::super::proof::smt::ast::SMTResult;
#[cfg(not(target_arch = "wasm32"))]
use super::super::proof::smt::translate;
#[cfg(not(target_arch = "wasm32"))]
use super::super::proof::smt::z3_backend::Z3Backend;
use super::super::proof::verdict::{
    BudgetReport, DisproofKind, DisproofModel, ProofFunctionCall, ProofResult, UnprovenReason,
};

/// Z3 实例——整个编译过程只初始化一次
#[cfg(not(target_arch = "wasm32"))]
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
    // 2a：精确匹配——零开销快速路径
    if ctx.assumptions.contains(constraint) {
        return ProofResult::Proved;
    }
    // 2b：SMT 蕴含——假设非空但不精确匹配
    #[cfg(not(target_arch = "wasm32"))]
    if !ctx.assumptions.is_empty() {
        if let Some(result) = try_implication(ctx, constraint, bindings) {
            return result;
        }
    }

    // === 第 3 级：SMT 求解 ===
    // SMT 翻译不支持 Call/If/Range 形式——跳过，直接进入第 4 级
    #[cfg(not(target_arch = "wasm32"))]
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
            kind: DisproofKind::PredicateViolation,
            assignments: bindings
                .iter()
                .map(|(k, v)| (k.clone(), format!("{:?}", v)))
                .collect(),
            constraint: format!("{}", constraint),
            span: None,
            predicate_span: None,
        })),
        Ok(_) => Some(ProofResult::Unproven {
            reason: UnprovenReason::BeyondKernel("约束表达式未求值为 Bool".into()),
            proof_calls: vec![],
            budget: ctx.budget.report(),
        }),
        Err(_) => None,
    }
}

/// 第 2b 级：SMT 假设蕴含
///
/// 检查当前假设栈是否蕴含目标约束。复用 `translate_constraint`
/// 将假设作为背景断言、目标取反送 Z3。unsat 表示假设蕴含目标。
/// sat/unknown 时不宣称 Disproved——返回 None 让后续级别继续。
#[cfg(not(target_arch = "wasm32"))]
fn try_implication(
    ctx: &ProofContext<'_>,
    constraint: &ConstExpr,
    bindings: &HashMap<String, ConstValue>,
) -> Option<ProofResult> {
    // 收集约束和假设中所有变量的 SMT 排序
    let mut var_sorts = translate::infer_var_sorts(constraint, bindings);
    for assumption in ctx.assumptions.current() {
        for (k, v) in translate::infer_var_sorts(assumption, bindings) {
            var_sorts.entry(k).or_insert(v);
        }
    }

    let commands =
        translate::translate_constraint(constraint, ctx.assumptions.current(), &var_sorts);

    match Z3_INSTANCE
        .lock()
        .unwrap()
        .solve(&commands, ctx.budget.time_ms_limit())
    {
        SMTResult::Unsat => Some(ProofResult::Proved),
        // sat = 假设不蕴含，约束可能独立成立 → 升级
        SMTResult::Sat { .. } => None,
        // unknown = 无法判断 → 升级
        SMTResult::Unknown { .. } => None,
    }
}

/// 第 3 级：SMT 求解
#[cfg(not(target_arch = "wasm32"))]
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
            kind: DisproofKind::PredicateViolation,
            assignments: model.assignments,
            constraint: format!("{}", constraint),
            span: None,
            predicate_span: None,
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
