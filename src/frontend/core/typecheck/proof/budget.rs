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
    time_ms_used: Cell<u64>, // 阶段 1 保留，阶段 2 (SMT) 启用
    time_ms_limit: u64,
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BudgetTracker {
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

    /// 记录 Z3 求解消耗的时间
    #[allow(dead_code)] // 精确计时在 Phase 3/4 实现
    pub fn record_time_ms(
        &self,
        ms: u64,
    ) {
        self.time_ms_used.set(self.time_ms_used.get() + ms);
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
