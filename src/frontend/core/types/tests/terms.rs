//! 编译期项的义务步与归一化三态（6.9.2-C1 / 6.9.3a / RFC-039 D62）
//!
//! 三条**判据牙齿**，分别钉住一道防线：
//! - 义务步（`TermEnv::admits`）：运行期名进不了 `normalize`；
//! - 归一化的类型化出口（6.9.3a）：`Unproven` 带**符号名**、预算耗尽可注入，
//!   闭项求值失败判 `Disproved`；
//! - 三态互不降格：`Unproven` 永不当 `Disproved`。

use crate::frontend::core::sema::proof::budget::BudgetTracker;
use crate::frontend::core::sema::proof::verdict::{ProofFailure, UnprovenReason};
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::eval::const_eval::{normalize, TermEnv};

/// 一次判定的预算（生产同款工厂：每次归一化新建一份）。
fn judgment_budget() -> BudgetTracker {
    BudgetTracker::for_judgment()
}

/// 义务步：**运行期名**（不在编译期环境里的名字）必须被拒之门外。
///
/// 这是「运行期值 vs 编译期名」的**第一道网**：`f(y)` 里的 `y` 走不到 `normalize`。
/// 它失守时后果并非「静默放行」——`y` 进入 `normalize` 后不是符号名，求值器未绑定
/// 该名即 Err ⇒ **`Disproved`** ⇒ 仍被拒绝（与义务步同码 E1113）。两道网同码，故
/// t29 M2（`admits` 恒真）在端到端行为矩阵里**不可辨**、只能在这一层被抓住；本测试
/// 守的就是「不进入 normalize」这条第一道网。
#[test]
fn test_term_env_rejects_runtime_name() {
    // Arrange
    let runtime_name = ConstExpr::NamedVar("y".to_string());
    let env = TermEnv::default();

    // Act
    let admitted = env.admits(&runtime_name);

    // Assert
    assert!(
        !admitted,
        "运行期名 y 不在编译期环境里，义务步必须拒绝（不得进入 normalize）"
    );
}

/// 归一化：**符号名**走 `Unproven { reason: Symbolic(名字) }`，名字必须带出去。
///
/// 6.9.3a 之前项层失败是无载荷的三值枚举（符号名无处落脚）——诊断只能说「无法确定」，
/// 说不出**哪个**名字；现在名字随失败值一起走完全程。
#[test]
fn test_normalize_symbolic_failure_carries_name() {
    // Arrange
    let symbolic_env = TermEnv::new(["k".to_string()], []);
    let term = ConstExpr::NamedVar("k".to_string());

    // Act
    let outcome = normalize(&term, &symbolic_env, &judgment_budget());

    // Assert
    match outcome {
        Err(ProofFailure::Unproven {
            reason: UnprovenReason::Symbolic(name),
            ..
        }) => assert_eq!(name, "k", "Unproven 必须携带符号名，不得丢成无名无因"),
        other => panic!("符号名应判 Unproven{{Symbolic(k)}}，实际: {other:?}"),
    }
}

/// 归一化：预算耗尽走 `Unproven { reason: BudgetExceeded }`（可注入触发）。
///
/// 预算语义是**每次判定**一份（`for_judgment`），不是每编译单元配额；测试用
/// `with_steps` 注入小预算即可让 2 节点以上的项在中途耗尽。
#[test]
fn test_normalize_budget_exhaustion_is_unproven() {
    // Arrange — `1 + 1` 有 3 个节点（BinOp + 两个 Lit），上限只给 1 步
    let term = ConstExpr::BinOp {
        op: BinOp::Add,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
    };
    let budget = BudgetTracker::with_steps(1);

    // Act
    let outcome = normalize(&term, &TermEnv::default(), &budget);

    // Assert
    match outcome {
        Err(ProofFailure::Unproven {
            reason: UnprovenReason::BudgetExceeded,
            budget,
            ..
        }) => assert_eq!(
            budget.steps_limit, 1,
            "预算报告必须带出本次判定的上限（证明用的是注入的那份预算）"
        ),
        other => panic!("预算耗尽应判 Unproven{{BudgetExceeded}}，实际: {other:?}"),
    }
}

/// 归一化：闭项求值失败判 `Disproved`（与 `Unproven`、`Ok` 三态分明）。
#[test]
fn test_normalize_closed_failure_is_disproved() {
    // Arrange
    let div_by_zero = ConstExpr::BinOp {
        op: BinOp::Div,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let closed_ok = ConstExpr::Lit(ConstValue::Int(42));

    // Act
    let failed = normalize(&div_by_zero, &TermEnv::default(), &judgment_budget());
    let succeeded = normalize(&closed_ok, &TermEnv::default(), &judgment_budget());

    // Assert
    assert!(
        matches!(failed, Err(ProofFailure::Disproved(_))),
        "1 / 0 是闭项且求值失败 ⇒ Disproved（C1 ⑥ 拒绝面）；实际: {failed:?}"
    );
    assert!(
        !matches!(failed, Err(ProofFailure::Unproven { .. })),
        "Disproved 与 Unproven 语义不同：前者拒绝，后者放行不特化"
    );
    assert_eq!(
        succeeded,
        Ok(ConstValue::Int(42)),
        "闭项求值成功 ⇒ Ok(value)（特化面）"
    );
}
