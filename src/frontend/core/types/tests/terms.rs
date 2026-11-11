//! 编译期项的义务步与归一化三态（6.9.2-C1 / RFC-039 D62）
//!
//! 这两条是**判据牙齿**：义务步（`TermEnv::admits`）与归一化（`normalize`）此前的
//! 覆盖只有「经 `compile_time_term` 的间接路径」，变异「admits 恒真」在行为矩阵里
//! 存活（t29 M2）。这里直接对两人判据本身下断言，使该变异可被抓住。

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::eval::const_eval::{normalize, TermEnv, TermOutcome};

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

    // 对照：编译期符号名（q=0 binder / 单值钉住形参）通过义务步，但归一化给 Unproven
    let symbolic_env = TermEnv::new(["k".to_string()], []);
    let symbolic = ConstExpr::NamedVar("k".to_string());
    assert!(
        symbolic_env.admits(&symbolic),
        "编译期符号名应通过义务步（它只是此刻没有值）"
    );
    assert_eq!(
        normalize(&symbolic, &symbolic_env),
        TermOutcome::Unproven,
        "符号名 ⇒ Unproven（放行但不特化），绝不与 Disproved 混同"
    );
}

/// 归一化：闭项求值失败判 `Disproved`（与 `Unproven`、`Value` 三态分明）。
#[test]
fn test_normalize_closed_failure_is_disproved() {
    // Arrange
    let div_by_zero = ConstExpr::BinOp {
        op: BinOp::Div,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let closed_ok = ConstExpr::Lit(ConstValue::Int(42));
    let env = TermEnv::default();

    // Act
    let failed = normalize(&div_by_zero, &env);
    let succeeded = normalize(&closed_ok, &env);

    // Assert
    assert_eq!(
        failed,
        TermOutcome::Disproved,
        "1 / 0 是闭项且求值失败 ⇒ Disproved（C1 ⑥ 拒绝面）"
    );
    assert_ne!(
        failed,
        TermOutcome::Unproven,
        "Disproved 与 Unproven 语义不同：前者拒绝，后者放行不特化"
    );
    assert_eq!(
        succeeded,
        TermOutcome::Value(ConstValue::Int(42)),
        "闭项求值成功 ⇒ Value（特化面）"
    );
}
