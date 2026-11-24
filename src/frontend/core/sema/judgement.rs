//! 判定点协议（RFC-027 §12.1 / RFC-039 D62 的 I2 + I7，6.9.3d）
//!
//! 一次判定 = **先统一，失败则回滚，再判等**：
//!
//! 1. 开日志标记 → `unify`；
//! 2. 成功：提交（绑定保留），判等不参与；
//! 3. 失败：**无条件回滚**本次尝试建立的绑定（RFC-027 §12.1「必须先回滚统一尝试
//!    产生的部分绑定」）；
//! 4. 回滚后取两侧的**展开形态**：任一侧含类型变量 ⇒ 不是判等的义务（判等不绑定
//!    变量，两侧 ground 才谈「同一」）⇒ 原样返回 `unify` 的诊断；
//! 5. 两侧 ground：用**一次判定一份**的预算跑 `defeq`：
//!    - `Proved` ⇒ 放行（两侧 ground 意味着本次 unify 本就不可能建立绑定，
//!      放行因此不会留下「半途放行」）；
//!    - `Disproved` ⇒ 保留 `unify` 的原文诊断（真不等；expected/found 结构比
//!      判等的 Debug 形态可读）；
//!    - `Unproven` ⇒ 换成 **E4023**，并把命题主体覆写为 `l ≡ r`、位置覆写为
//!      判定点（`defeq` 内部没有位置，不得由它伪造）。
//!
//! 为什么入口在这一层而不是 `TypeConstraintSolver`：求解器不持有类型环境/预算/
//! 依赖类型环境，而且它本身被 `TypeEnvironment` 拥有（自引用，类型上取不到那些
//! 输入）。判等只在**这一处**发生，其余 `unify` 调用点保持结构比较语义。

use std::collections::HashMap;

use crate::frontend::core::sema::layers::equivalence::{defeq, DefeqContext};
use crate::frontend::core::sema::proof::budget::BudgetTracker;
use crate::frontend::core::sema::proof::verdict::{ProofFailure, UnprovenSubject};
use crate::frontend::core::types::eval::dependent_types::DependentTypeEnv;
use crate::frontend::core::types::mono::{MonoType, PolyType};
use crate::frontend::core::types::solver::TypeConstraintSolver;
use crate::util::diagnostic::Diagnostic;
use crate::util::span::Span;

/// 判定失败的两臂。
///
/// 为什么要区分而不是只回一个 `Diagnostic`：调用点普遍会**自撰**更贴切的报文
/// （指向注解、带上解析后的形态）。若只回一个 `Diagnostic`，那些站点会把
/// 「算不出来」改写成「类型不匹配」——I5 的「Unproven ≠ False」就在用户面失效
/// （6.9.3d 实现首轮实测：E4023 被 19 个站点的 E1002 重写掉，用户看不到）。
/// 类型上的两臂逼每个站点显式做出选择。
pub enum JudgementFailure {
    /// 真不等：沿用 `unify` 的原文诊断（RFC-027 §12.1 的「统一失败」语义）
    Mismatch(Diagnostic),
    /// 算不出来：E4023（带命题与位置），**不得**被站点改写成「不等」
    Unproven(Diagnostic),
}

impl JudgementFailure {
    /// 站点自撰「类型不匹配」报文的场合：`Mismatch` 用站点报文（零文本变化），
    /// `Unproven` 原样上抛。
    pub fn or_site(
        self,
        site: Diagnostic,
    ) -> Diagnostic {
        match self {
            Self::Unproven(unproven) => unproven,
            Self::Mismatch(_) => site,
        }
    }

    /// 无站点专用报文的场合：直接取携带的诊断。
    pub fn into_diagnostic(self) -> Diagnostic {
        match self {
            Self::Mismatch(diagnostic) | Self::Unproven(diagnostic) => diagnostic,
        }
    }
}

/// **一次判定**（判定点的唯一入口）。
///
/// `types` 是 δ 展开表（`TypeEnvironment.types` 或其同一张注入表）；`dep_env` 为
/// `None` 表示该调用面没有依赖类型环境（直构的推断器），此时判定退化为「只
/// unify」——不猜，也不在没有归约环境时宣称判等结论。
#[allow(clippy::result_large_err, clippy::too_many_arguments)]
pub fn decide(
    solver: &mut TypeConstraintSolver,
    lhs: &MonoType,
    rhs: &MonoType,
    span: Option<Span>,
    types: &HashMap<String, PolyType>,
    dep_env: Option<&DependentTypeEnv>,
) -> Result<(), JudgementFailure> {
    // 一次判定一份预算（RFC-027 §8；`BudgetTracker::for_judgment` 是唯一工厂）。
    let budget = BudgetTracker::for_judgment();
    decide_with_budget(solver, lhs, rhs, span, types, dep_env, &budget)
}

/// `decide` 的主体（预算由调用方给：生产走 `for_judgment`，测试可注入小预算）。
///
/// `pub(super)` 只为让 `sema` 内测试能以小预算驱动「预算耗尽 ⇒ E4023」；生产调用者只有 `decide` 一处。
#[allow(clippy::result_large_err, clippy::too_many_arguments)]
pub(super) fn decide_with_budget(
    solver: &mut TypeConstraintSolver,
    lhs: &MonoType,
    rhs: &MonoType,
    span: Option<Span>,
    types: &HashMap<String, PolyType>,
    dep_env: Option<&DependentTypeEnv>,
    budget: &BudgetTracker,
) -> Result<(), JudgementFailure> {
    #[cfg(test)]
    metrics::DECIDE_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let mark = solver.journal_mark();
    let unify_error = match solver.unify(lhs, rhs) {
        Ok(()) => {
            solver.journal_commit();
            return Ok(());
        }
        Err(error) => error,
    };
    // 失败即原子失败：先回滚，再判形态（回滚前测 ground 会被半截绑定骗过）。
    solver.journal_rollback(mark);

    // ground 门：判等不绑定变量，含变量的一侧不是判等的义务。
    let lhs_expanded = solver.resolve_type(lhs);
    let rhs_expanded = solver.resolve_type(rhs);
    if solver.has_type_var(&lhs_expanded) || solver.has_type_var(&rhs_expanded) {
        tracing::debug!(
            "判定点：非 ground，判等不尝试（lhs={} rhs={}）",
            lhs_expanded,
            rhs_expanded
        );
        return Err(JudgementFailure::Mismatch(unify_error));
    }
    // 没有归约环境 ⇒ 不宣称判等结论（退回 unify 的原文诊断）。
    let Some(dep_env) = dep_env else {
        return Err(JudgementFailure::Mismatch(unify_error));
    };

    let ctx = DefeqContext {
        types,
        budget,
        dep_env,
    };
    #[cfg(test)]
    metrics::DEFEQ_ATTEMPTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let verdict = defeq(&lhs_expanded, &rhs_expanded, &ctx);
    tracing::debug!(
        "判定点：ground 判等 lhs={} rhs={} 结果={:?}",
        lhs_expanded,
        rhs_expanded,
        verdict
    );
    match verdict {
        Ok(()) => {
            #[cfg(test)]
            {
                metrics::DEFEQ_PROVED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                record_steps(budget);
            }
            Ok(())
        }
        // 真不等：保留 unify 的原文诊断（expected/found 结构比 Debug 形态可读）。
        Err(ProofFailure::Disproved(_)) => {
            #[cfg(test)]
            record_steps(budget);
            Err(JudgementFailure::Mismatch(unify_error))
        }
        // 算不出来：**不得**降格成「不等」，换 E4023 并补真实命题与位置。
        Err(ProofFailure::Unproven {
            reason,
            proof_calls,
            budget: report,
            ..
        }) => {
            #[cfg(test)]
            record_steps(budget);
            let failure = ProofFailure::Unproven {
                reason,
                proof_calls,
                budget: report,
                subject: Some(UnprovenSubject::TypeEquivalence(
                    Box::new(lhs_expanded),
                    Box::new(rhs_expanded),
                )),
                span,
            };
            Err(JudgementFailure::Unproven(failure.into_diagnostic()))
        }
    }
}

/// 测试期预算量测：总步数与单次最大步数（生产构建不含本函数）。
#[cfg(test)]
fn record_steps(budget: &BudgetTracker) {
    let used = u64::from(budget.report().steps_used);
    metrics::STEPS_TOTAL.fetch_add(used, std::sync::atomic::Ordering::Relaxed);
    metrics::STEPS_MAX.fetch_max(used, std::sync::atomic::Ordering::Relaxed);
}

/// 判定点的量测计数器（仅测试构建存在）。
///
/// 用途：契约第 4 条要求「语料全量 decide 次数 / 进入 defeq 次数 / 总步数 /
/// 单次最大步数」是实测数字而不是估计。
#[cfg(test)]
pub(super) mod metrics {
    use std::sync::atomic::{AtomicU64, Ordering};

    pub static DECIDE_CALLS: AtomicU64 = AtomicU64::new(0);
    pub static DEFEQ_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
    pub static DEFEQ_PROVED: AtomicU64 = AtomicU64::new(0);
    pub static STEPS_TOTAL: AtomicU64 = AtomicU64::new(0);
    pub static STEPS_MAX: AtomicU64 = AtomicU64::new(0);

    /// 清零（量测入口先调用，避免与同进程其它测试的数字混在一起）。
    pub fn reset() {
        DECIDE_CALLS.store(0, Ordering::Relaxed);
        DEFEQ_ATTEMPTS.store(0, Ordering::Relaxed);
        DEFEQ_PROVED.store(0, Ordering::Relaxed);
        STEPS_TOTAL.store(0, Ordering::Relaxed);
        STEPS_MAX.store(0, Ordering::Relaxed);
    }

    /// (decide 次数, 进入 defeq 次数, Proved 次数, 总步数, 单次最大步数)
    pub fn snapshot() -> (u64, u64, u64, u64, u64) {
        (
            DECIDE_CALLS.load(Ordering::Relaxed),
            DEFEQ_ATTEMPTS.load(Ordering::Relaxed),
            DEFEQ_PROVED.load(Ordering::Relaxed),
            STEPS_TOTAL.load(Ordering::Relaxed),
            STEPS_MAX.load(Ordering::Relaxed),
        )
    }
}
