//! 求解预算追踪器
//!
//! RFC-027 §8：硬限制编译期求值的步数和时间。
//! 不给用户旋钮——内部写死。使用 Cell 实现内部可变性。

use std::cell::Cell;

use super::verdict::BudgetReport;

/// 预算追踪器（内部可变性——spend() 只需要 &self）
#[derive(Debug)]
pub struct BudgetTracker {
    steps_used: Cell<u32>,
    steps_limit: u32,
    #[expect(dead_code)]
    time_ms_used: Cell<u64>, // 阶段 1 保留，阶段 2 (SMT) 启用
    time_ms_limit: u64,
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BudgetTracker {
    /// **一次判定**的预算（6.9.3a 的唯一工厂）。
    ///
    /// 语义仍是 RFC-027 §8 的既有口径「每次判定一份」：**不是**每编译单元配额。
    /// 每次证明 / 每次项归一化前新建一份，互不共享、互不累计。
    pub fn for_judgment() -> Self {
        Self::new()
    }

    /// 指定步数上限（**测试**注入小预算用；生产一律走 [`Self::for_judgment`]）。
    pub fn with_steps(steps_limit: u32) -> Self {
        Self {
            steps_used: Cell::new(0),
            steps_limit,
            time_ms_used: Cell::new(0),
            time_ms_limit: 100,
        }
    }

    pub fn new() -> Self {
        Self {
            steps_used: Cell::new(0),
            steps_limit: 10_000,
            time_ms_used: Cell::new(0),
            time_ms_limit: 100,
        }
    }

    /// 消耗一步。超限返回 false → 调用方返回 Unproven
    pub fn spend(&self) -> bool {
        let used = self.steps_used.get();
        if used >= self.steps_limit {
            return false;
        }
        self.steps_used.set(used + 1);
        true
    }

    /// 生成预算报告
    pub fn report(&self) -> BudgetReport {
        BudgetReport {
            steps_used: self.steps_used.get(),
            steps_limit: self.steps_limit,
        }
    }

    /// Z3 超时毫秒数。SMT 后端在调用 Z3 前读取此值。
    pub fn time_ms_limit(&self) -> u64 {
        self.time_ms_limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_spend_within_limit() {
        let budget = BudgetTracker::new();
        assert!(budget.spend());
        assert_eq!(budget.report().steps_used, 1);
    }

    #[test]
    fn test_budget_exhausted() {
        let budget = BudgetTracker::new();
        for _ in 0..10_000 {
            assert!(budget.spend());
        }
        assert!(!budget.spend());
    }

    #[test]
    fn test_time_ms_limit_default() {
        let budget = BudgetTracker::new();
        assert_eq!(budget.time_ms_limit(), 100);
    }
}
