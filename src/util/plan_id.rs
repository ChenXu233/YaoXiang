//! 释放计划身份（RFC-039 D20 / WBS 4.5.1）
//!
//! # 为什么需要它
//!
//! `ReleasePlan`（NLL 精确释放计划）原先以 **Span** 为键：所有权检查器按
//! 「最后使用**表达式**的 span」记键，IR 生成按「**语句** span」查表——两个
//! 跨层各自持有的位置量天然可以对不上（实测 173 个语料文件、533 个产出键
//! 永不命中，见 `src/middle/core/tests/release_plan_spans.rs`），Drop 静默丢失。
//!
//! `PlanId` 把这条契约从「span 位置巧合」改成「AST 节点结构身份」：由解析器
//! 在构造语句时**单调分配**（编译单元内唯一），随 `Stmt` 节点走——产出侧
//! （ownership）与消费侧（ir_gen）读的是**同一个节点上的同一个值**，不存在
//! 两份位置量对不上的可能。
//!
//! # 契约
//!
//! - **分配**：`ParserState::alloc_plan_id` 是唯一分配点，解析期按序递增；
//! - **载体**：`crate::frontend::core::parser::ast::Stmt::plan_id`；
//! - **结算**：`ReleasePlan` 产出 ⊆ 消费（IR 生成逐语句查表）——差集非空即
//!   响亮报出（义务账本 `W1090`），不得静默（D41）。
//!
//! 解析器之外的合成语句（ir_gen 内建语句、直驱测试夹具）用
//! [`PlanId::UNPLANNED`]：它们不属任何解析单元，因此也不构成释放计划义务。

use std::fmt;

/// 编译单元内唯一的语句身份——释放计划（`ReleasePlan`）的键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlanId(u32);

impl PlanId {
    /// 未纳入释放计划的身份：解析器之外的合成语句（IR 内建语句、测试夹具）。
    ///
    /// 取 `u32::MAX`：解析器的单调计数器从 0 起，永远到不了它。
    pub const UNPLANNED: Self = Self(u32::MAX);

    /// 由解析期的分配序号构造（唯一调用点是 `ParserState::alloc_plan_id`）。
    pub fn from_ordinal(ordinal: u32) -> Self {
        Self(ordinal)
    }

    /// 编译单元内的分配序号（诊断与跨文件比对用；非稳定 ABI）。
    pub fn ordinal(self) -> u32 {
        self.0
    }

    /// 是否是解析器分配的计划身份（`false` = 合成语句，无释放计划义务）。
    pub fn is_planned(self) -> bool {
        self != Self::UNPLANNED
    }
}

impl fmt::Display for PlanId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        if *self == Self::UNPLANNED {
            write!(f, "plan#unplanned")
        } else {
            write!(f, "plan#{}", self.0)
        }
    }
}
