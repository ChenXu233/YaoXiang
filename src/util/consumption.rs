//! 消费台账（RFC-039 D20 / WBS 4.5.1 · 4.5.2）
//!
//! 「产出的键必被消费」是跨阶段契约的通用形状：产出侧（前端）登记键，消费侧
//! （IR 生成）按同一身份取用，driver 的义务账本在阶段表尾部**唯一结算点**结算。
//! 台账与产出表**同体**（`Arc` 共享，克隆不复制）：IR 生成器持有的是
//! `TypeCheckResult` 的克隆，故「生成器写的账」与「结算读的账」是同一个账。
//!
//! 本类型是这条机制的**单一实现**，两个义务共用（4.5.1 `ReleasePlan`、
//! 4.5.2 `OverloadResolutions`）——同一行为不得在两处维护（禁令三 A）。
//!
//! 锁中毒（持锁线程 panic）两侧都朝**响亮**方向退化：记账失败 ⇒ 该条目仍算
//! 未消费 ⇒ 结算报 W1090；读取失败 ⇒ 一律算未消费。不存在「静默算已消费」。

use std::collections::HashSet;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

/// 键空间由使用方决定（当前两处都是 `ConsumptionLedger<PlanId>`）。
#[derive(Debug, Clone)]
pub struct ConsumptionLedger<K> {
    inner: Arc<Mutex<LedgerState<K>>>,
}

/// 台账本体（随产出表克隆共享）。
#[derive(Debug)]
struct LedgerState<K> {
    /// 已消费的产出键
    consumed: HashSet<K>,
    /// 消费侧是否**完整跑完一轮**（IR 生成成功返回）：结算的「消费者真的跑过」
    /// 判据。失败 / 未运行时台账不完整——此时未消费键**不判**（失败本身已按
    /// 首因响亮报出，结算不抢首因、也不造假账）。
    completed: bool,
}

impl<K> Default for LedgerState<K> {
    fn default() -> Self {
        Self {
            consumed: HashSet::new(),
            completed: false,
        }
    }
}

impl<K> Default for ConsumptionLedger<K> {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LedgerState::default())),
        }
    }
}

impl<K: Eq + Hash + Copy> ConsumptionLedger<K> {
    /// 记账：该条目已被消费（幂等）。
    pub fn mark(
        &self,
        key: K,
    ) {
        if let Ok(mut ledger) = self.inner.lock() {
            ledger.consumed.insert(key);
        }
    }

    /// 标记消费侧已完整跑完一轮（IR 生成成功返回；幂等）。
    pub fn mark_completed(&self) {
        if let Ok(mut ledger) = self.inner.lock() {
            ledger.completed = true;
        }
    }

    /// 该条目的消费是否已记账。
    pub fn contains(
        &self,
        key: K,
    ) -> bool {
        self.inner
            .lock()
            .map(|ledger| ledger.consumed.contains(&key))
            .unwrap_or(false)
    }

    /// 消费侧是否完整跑完一轮（IR 生成成功返回）。
    pub fn is_completed(&self) -> bool {
        self.inner
            .lock()
            .map(|ledger| ledger.completed)
            .unwrap_or(false)
    }

    /// 已记账条数（审计面计数）。
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .map(|ledger| ledger.consumed.len())
            .unwrap_or(0)
    }

    /// 是否一条都没记账（审计面用）。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 产出键里**未被消费**的那些（结算判据：必须为空；D41）。
    ///
    /// 升序由调用方决定（键的序语义不等同于「产出序」，本层不做假设）。
    pub fn unconsumed<I: IntoIterator<Item = K>>(
        &self,
        produced: I,
    ) -> Vec<K> {
        produced
            .into_iter()
            .filter(|key| !self.contains(*key))
            .collect()
    }
}
