//! WBS 4.9.6 / #450 逐候选归因的**读取面**钉板：`OverloadResolutions::iter` 纯读。
//!
//! 规范来源:
//! - RFC-039 D20 / WBS 4.5.2：决议表键 = 调用点 `PlanId`，`get` 是**唯一消费入口**
//!   （命中即记账；结算判据 = `unconsumed()` 必须为空 ⇒ 未消费即 W1090 响亮）。
//! - WBS 4.9.6 验收③（含未被重载决议命中的方法仍报）：死代码分析要读决议表做事后
//!   归因，但**不得**冒充消费——否则 ir_gen 忘记消费时结算不再响亮。
//!
//! 判据：调用 `iter()` 前后 `unconsumed()` 集合逐条不变（`iter` 不触达 `get`、
//! 不动消费台账）。

use std::collections::HashSet;

use crate::frontend::core::sema::types::{MethodResolution, OverloadResolutions};
use crate::util::plan_id::PlanId;

/// `iter()` 纯读：不改变消费台账（`unconsumed()` 前后一致）。
#[test]
fn test_resolution_iter_is_read_only() {
    // Arrange：两条决议条目，任何一条都还没被 `get` 消费
    let table = OverloadResolutions::from_entries([
        (
            PlanId::from_ordinal(1),
            MethodResolution::target_only("Temp.to"),
        ),
        (
            PlanId::from_ordinal(2),
            MethodResolution::target_only("Temp.to#1"),
        ),
    ]);
    let before: HashSet<PlanId> = table.unconsumed().into_iter().collect();

    // Act：纯读枚举全部条目
    let names: Vec<&str> = table
        .iter()
        .map(|(_, resolution)| resolution.target.as_str())
        .collect();

    // Assert：读到了名字，且消费台账逐条不变（未消费集合原样）
    assert_eq!(
        names.len(),
        2,
        "iter 必须能枚举全部决议条目，实际: {names:?}"
    );
    let after: HashSet<PlanId> = table.unconsumed().into_iter().collect();
    assert_eq!(
        before, after,
        "iter 不得计入消费台账（否则会掩盖 W1090 消费必达结算）"
    );
}
