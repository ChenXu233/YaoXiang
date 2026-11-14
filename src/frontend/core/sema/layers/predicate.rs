//! Layer 3：精化谓词证明 — 基于 RFC-027 §4
//!
//! 对 Refined 类型的约束表达式进行求值。
//! 四级分派：
//!   1. Evaluator 直接求值（Phase 1，微秒级）——所有变量有具体值
//!   2. 假设栈蕴含（Phase 2A + 3.2）——精确匹配 + SMT 蕴含推理
//!   3. Z3 SMT 求解（Phase 2B，毫秒级）——符号变量 + 蕴含推理
//!   4. 证明函数调用（Phase 2.5）——识别 ConstExpr::Call 让 Pipeline 编译期执行

use std::collections::HashMap;

use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
use crate::frontend::core::types::eval::const_eval::ConstGenericEval;
use crate::frontend::core::types::eval::evaluator::Evaluator;
use crate::frontend::core::types::mono::MonoType;
use super::super::proof::context::ProofContext;
use super::super::proof::smt::ast::SMTResult;
use super::super::proof::smt::backend::{with_shared_solver, SolverConsumer};
use super::super::proof::smt::translate;
use super::super::proof::verdict::{
    BudgetReport, DisproofKind, DisproofModel, ProofFailure, ProofFunctionCall, ProofResult,
    ProofResultExt, UnprovenReason, UnprovenSubject,
};

// 求解器获取面（RFC-039 D58）：本模块**不再**自持 `static SOLVER` 私有单例。
//
// 私有单例是 D58 点名的三形态之一：缓存是实例内的（z3_backend.rs），私有单例
// 使命中率计数在消费点之间分裂；且与 checker.rs / ownership.rs 的获取面并存。
// 两个使用点（`try_implication` / `try_smt_solve`）改走进程级共享口
// `backend::with_shared_solver`，后端不可用（Z3 未安装）时**不 panic**，
// 按 `SMTResult::Unknown` 保守降级——`Solver` 契约（backend.rs）：未知一律
// 保守方向，由上层决定降级形态；此处上层即 checker 的 Unproven→诊断路径
// （E2031 族硬错误），诊断仍在、进程不崩（WBS 3.3.1，02 §改动清单）。

/// 求解器不可用时的保守降级结果。
///
/// 抽出为独立函数以便单测——进程级单例无法安全模拟 Z3 缺失（并发测试共享
/// 本静态量，置 None 会污染其他用例）。
pub(super) fn solver_unavailable_result() -> SMTResult {
    SMTResult::Unknown {
        reason: "SMT 求解器不可用（Z3 未安装或初始化失败）".to_string(),
    }
}

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
    check_predicate_with_mode(ctx, refined, bindings, PredicateMode::Full)
}

/// 判定档位（RFC-027 §4 三值判定的来源限制）。
///
/// `Full` = 现有四级（direct eval → 精确 Γ → SMT 蕴含/求解 → 证明函数）。
/// `NoSmt` = 只走 direct eval / 精确 Γ / 证明函数：SMT 只作加速提示、**不作
/// 证明来源**。依赖记录 Σ 的构造点义务用这一档（09a 契约 §四 的 D3 待裁形态；
/// 裁决前不得把 SMT 的 Unsat 当成构造义务的证明）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PredicateMode {
    /// 全部四级（既有调用点口径）
    Full,
    /// 不用 SMT 作证明来源（构造点义务口径）
    NoSmt,
}

/// 按档位判定（唯一实现：[check_predicate] 是 `Full` 的薄包装）。
pub fn check_predicate_with_mode(
    ctx: &ProofContext<'_>,
    refined: &MonoType,
    bindings: &HashMap<String, ConstValue>,
    mode: PredicateMode,
) -> ProofResult {
    let constraint = match refined {
        MonoType::Refined { constraint, .. } => constraint,
        _ => return Ok(()),
    };

    // === 第 1 级：Evaluator 直接求值 ===
    if let Some(result) = try_direct_eval(ctx, constraint, bindings) {
        return result;
    }

    // === 第 2 级：假设栈蕴含 ===
    // 2a：精确匹配——零开销快速路径
    if ctx.assumptions.contains(constraint) {
        return Ok(());
    }
    if mode == PredicateMode::Full {
        // 2b：SMT 蕴含——假设非空但不精确匹配
        if !ctx.assumptions.is_empty() {
            if let Some(result) = try_implication(ctx, constraint, bindings) {
                return result;
            }
        }

        // === 第 3 级：SMT 求解 ===
        // SMT 翻译不支持 Call/If/Range 形式——跳过，直接进入第 4 级
        if !matches!(
            constraint,
            ConstExpr::Call { .. } | ConstExpr::If { .. } | ConstExpr::Range { .. }
        ) {
            let smt_result = try_smt_solve(ctx, constraint, bindings);
            if smt_result.is_proved() || matches!(smt_result, Err(ProofFailure::Disproved(_))) {
                return smt_result;
            }
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
    let mut evaluator = Evaluator::new(&ctx.env.types, &ctx.budget, &ctx.dep_env);
    match evaluator.eval_expr(constraint, bindings) {
        Ok(ConstValue::Bool(true)) => Some(Ok(())),
        Ok(ConstValue::Bool(false)) => Some(Err(ProofFailure::Disproved(DisproofModel {
            kind: DisproofKind::PredicateViolation,
            assignments: bindings
                .iter()
                .map(|(k, v)| (k.clone(), format!("{:?}", v)))
                .collect(),
            constraint: format!("{}", constraint),
            span: None,
            predicate_span: None,
        }))),
        Ok(_) => Some(Err(ProofFailure::Unproven {
            reason: UnprovenReason::BeyondKernel("约束表达式未求值为 Bool".into()),
            proof_calls: vec![],
            budget: ctx.budget.report(),

            subject: Some(UnprovenSubject::Term(constraint.clone())),
            span: None,
        })),
        Err(_) => None,
    }
}

/// 第 2b 级：SMT 假设蕴含
///
/// 检查当前假设栈是否蕴含目标约束。复用 `translate_constraint`
/// 将假设作为背景断言、目标取反送 Z3。unsat 表示假设蕴含目标。
/// sat/unknown 时不宣称 Disproved——返回 None 让后续级别继续。
fn try_implication(
    ctx: &ProofContext<'_>,
    constraint: &ConstExpr,
    bindings: &HashMap<String, ConstValue>,
) -> Option<ProofResult> {
    // 收集约束和假设中所有变量的 SMT 排序
    let mut var_sorts = translate::infer_var_sorts(constraint, bindings);
    let assumptions = ctx.assumptions.current();
    for assumption in &assumptions {
        for (k, v) in translate::infer_var_sorts(assumption, bindings) {
            var_sorts.entry(k).or_insert(v);
        }
    }

    let commands = translate::translate_constraint(constraint, &assumptions, &var_sorts);

    // 共享口（RFC-039 D58）；求解器不可用（None）= 无法判断 →
    // 经 `?` 返回升级（与 Unknown 同一降级方向）
    let smt_result = with_shared_solver(SolverConsumer::Predicate, |solver| {
        solver.map(|solver| solver.solve(&commands, ctx.budget.time_ms_limit()))
    })?;
    match smt_result {
        SMTResult::Unsat => Some(Ok(())),
        // sat = 假设不蕴含，约束可能独立成立 → 升级
        SMTResult::Sat { .. } => None,
        // unknown = 无法判断 → 升级
        SMTResult::Unknown { .. } => None,
    }
}

/// 第 3 级：SMT 求解
fn try_smt_solve(
    ctx: &ProofContext<'_>,
    constraint: &ConstExpr,
    bindings: &HashMap<String, ConstValue>,
) -> ProofResult {
    let var_sorts = translate::infer_var_sorts(constraint, bindings);
    let assumptions = ctx.assumptions.current();
    let commands = translate::translate_constraint(constraint, &assumptions, &var_sorts);

    // 共享口（RFC-039 D58）；Z3 缺失不 panic——按 Unknown 保守降级（WBS 3.3.1）
    let smt_result = with_shared_solver(SolverConsumer::Predicate, |solver| match solver {
        Some(solver) => solver.solve(&commands, ctx.budget.time_ms_limit()),
        None => solver_unavailable_result(),
    });
    match smt_result {
        SMTResult::Unsat => Ok(()),
        SMTResult::Sat { model } => Err(ProofFailure::Disproved(DisproofModel {
            kind: DisproofKind::PredicateViolation,
            assignments: model.assignments,
            constraint: format!("{}", constraint),
            span: None,
            predicate_span: None,
        })),
        SMTResult::Unknown { reason } => Err(ProofFailure::Unproven {
            reason: UnprovenReason::BeyondKernel(reason),
            proof_calls: vec![],
            budget: ctx.budget.report(),

            subject: Some(UnprovenSubject::Term(constraint.clone())),
            span: None,
        }),
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
        // 将 ConstExpr args 转为 ConstValue（折叠失败 = 义务不进证明管道，
        // 判定单元落 E2031——见 judge_refined_obligation 的空证法分支）
        let mut const_args: Vec<ConstValue> = Vec::with_capacity(args.len());
        for a in args {
            match fold_proof_call_arg(a, bindings) {
                Some(value) => const_args.push(value),
                None => {
                    return Err(ProofFailure::Unproven {
                        reason: UnprovenReason::BeyondKernel(
                            "证明函数实参必须是字面量或已绑定变量".into(),
                        ),
                        proof_calls: vec![],
                        budget: budget_report,

                        subject: Some(UnprovenSubject::Term(constraint.clone())),
                        span: None,
                    });
                }
            }
        }

        return Err(ProofFailure::Unproven {
            reason: UnprovenReason::ProofFunctionRequired,
            proof_calls: vec![ProofFunctionCall {
                func_name: func.clone(),
                args: const_args,
            }],
            budget: budget_report,

            subject: Some(UnprovenSubject::Term(constraint.clone())),
            span: None,
        });
    }

    Err(ProofFailure::Unproven {
        reason: UnprovenReason::BeyondKernel("无法自动证明且无证明函数".into()),
        proof_calls: vec![],
        budget: budget_report,

        subject: Some(UnprovenSubject::Term(constraint.clone())),
        span: None,
    })
}

/// 折叠单个证明函数实参（RFC-027 §3.1：实参就是函数调用的实参——`f(5)` 与
/// `f(-5)` 同为合法调用形态，issue #556）。
///
/// 字面量直取、名字查绑定；一元运算（取负/取正/逻辑非）折叠内层后交**共享
/// const 求值器** [`ConstGenericEval`]——取负走 i128 `checked_neg`，与 #507
/// 备案的饱和极端形态同源，不在此另写第二套取负。其余形态（二元运算、嵌套
/// 调用、If/Range 等）不可折叠 → `None`，由调用方走保守「不可折叠」分支
/// （义务不进证明管道，判定单元落 E2031）。
fn fold_proof_call_arg(
    arg: &ConstExpr,
    bindings: &HashMap<String, ConstValue>,
) -> Option<ConstValue> {
    match arg {
        ConstExpr::Lit(v) => Some(v.clone()),
        ConstExpr::NamedVar(name) => bindings.get(name).cloned(),
        ConstExpr::UnOp { op, expr } => {
            let inner = fold_proof_call_arg(expr, bindings)?;
            let evaluator = ConstGenericEval::new();
            // 求值失败（溢出/类型不合）时 `ConstGenericEval` 会经 span 强制的
            // builder 构造诊断；折叠只要「成不成」，诊断被 `.ok()` 丢弃——
            // 与 `normalize` 同款占位 span 守卫，避免在 walk 上下文外 panic。
            let _span_guard =
                crate::util::diagnostic::push_current_span(crate::util::span::Span::default());
            evaluator
                .eval(&ConstExpr::UnOp {
                    op: *op,
                    expr: Box::new(ConstExpr::Lit(inner)),
                })
                .ok()
        }
        _ => None,
    }
}
