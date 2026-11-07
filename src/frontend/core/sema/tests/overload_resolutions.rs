//! RFC-039 D20 / WBS 4.5.2：`overload_resolutions` 键 = 调用点 `PlanId`
//! —— 决议形态覆盖与消费必达
//!
//! 规范来源:
//! - docs/src/rfc/accepted/039-compiler-architecture.md D20（Span 键控 → `PlanId`：
//!   `ReleasePlan` / `overload_resolutions` 的键改为随 AST 节点走的身份）
//! - RFC-011a §3（方法级重载：调用点按 实参兼容 + 期望返回 决议候选，IR 定义侧按
//!   声明序混编 `Temp.to` / `Temp.to#1`）
//!
//! # 键的同一性
//!
//! 产出（`inference/expressions.rs` 的 `infer_call_expr`）与消费（`ir_gen` 的两处
//! 方法调用命名）都读**同一个 `Expr::Call` 节点**上的 `plan_id`——不再靠「Call 的
//! span 两处各算一遍」的位置巧合。未分配身份（`PlanId::UNPLANNED`，解析器之外的
//! 合成调用）**不可被消费**（`get` 返回 `None`），故任何「合成调用进了决议表」都会
//! 以未消费条目在义务账本结算处响亮（W1090），不会静默落回未混编名。
//!
//! # 决议形态覆盖清单（来源：本文件逐条钉板）
//!
//! | 形态 | 产出条目 | 消费 | 钉板用例 |
//! | --- | --- | --- | --- |
//! | 期望返回过滤（匿名绑定双候选） | 2 | ✓ 两个混编名都进 IR | `expected_return` |
//! | 位置实参兼容过滤 | 1 | ✓ 决议出的混编名进 IR | `positional_arg` |
//! | 类型体内绑定（`pick = fn[0]`，有绑定消费点） | 1 | ✓ 决议名替换绑定名 | `type_body_binding` |
//! | `?`/Try 操作数中的调用 | 1 | ✓ | `try_operand` |
//! | 命名实参（纯命名） | 1 | ✓ 决议 + 消费（#452） | `named_args` |
//! | 命名实参（位置 + 命名混合） | 1 | ✓ 决议 + 消费（#452） | `mixed_args` |
//! | 歧义（无期望上下文） | 0 | —（E1107 报错） | `ambiguous` |
//! | 变体构造 `Color.red()` | 0 | —（走变体构造路径，另在相位） | `variant_ctor` |
//! | 未分配身份（合成调用） | 不可消费 | —（`get` 拒绝，结算响亮） | `unplanned_key` |
//!
//! 命名实参两条形态的语义边界（#452 修订）：命名实参按**形参名**绑定到候选形参位后
//! 与位置实参一起参与匹配 ⇒ 两条形态都产出条目且都被消费。改动前的实测红态：
//! `r: Int = t.pick(1, a = 2)` 决议候选被丢弃、落回默认绑定（静默给错答案）；
//! 4.5.2 把消费门落成响亮（W1090）；#452 把决议与消费两端一起补齐，
//! 冲突/缺参/未知名/重复命名一律编译期报错（不得静默择一）。

use crate::frontend::core::sema::types::OverloadResolutions;
use crate::util::plan_id::PlanId;

/// 未分配身份（合成调用）**不可被消费**：`get` 拒绝、条目留在差集里——
/// 「消费侧遇未分配必须响亮」的单元级席位（响亮由义务账本结算给出）。
#[test]
fn test_overload_resolution_unplanned_key_is_never_consumed() {
    // Arrange：解析器之外的合成调用节点（UNPLANNED）上登记一条决议
    let mut table = OverloadResolutions::default();
    table.resolve(PlanId::UNPLANNED, "Temp.synth".to_string());

    // Act
    let consumed = table.get(PlanId::UNPLANNED);
    let unconsumed = table.unconsumed();

    // Assert
    assert!(
        consumed.is_none(),
        "未分配身份不得被消费（否则合成调用会静默取到别人的决议）"
    );
    assert_eq!(
        unconsumed,
        vec![PlanId::UNPLANNED],
        "未分配身份的条目必须留在差集里（结算据此响亮）"
    );
}
