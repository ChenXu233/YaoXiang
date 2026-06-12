//! 证明上下文
//!
//! 在编译期证明管道中传递的共享状态。
//! 搬迁阶段只包含最小字段，RFC-027 实现时加入路径条件栈和依赖图。

use crate::frontend::core::typecheck::environment::TypeEnvironment;

use super::assumptions::AssumptionStack;
use super::budget::BudgetTracker;
use super::dep_graph::TypeDepGraph;

/// 证明上下文 —— 证明管道的共享状态
///
/// 搬迁阶段：仅持有类型环境引用。
/// RFC-027 实现时加入：
/// - 路径条件栈（AssumptionStack）
/// - 变量类型依赖图（TypeDepGraph）
/// - 求解预算（BudgetTracker）
pub struct ProofContext<'a> {
    /// 类型环境
    pub env: &'a TypeEnvironment,
    /// 路径条件栈（RFC-027 §3.2-3.3）
    pub assumptions: AssumptionStack,
    /// 变量类型依赖图（RFC-027 §6.1）
    pub dep_graph: TypeDepGraph,
    /// 求解预算追踪器（RFC-027 §8）
    pub budget: BudgetTracker,
}

impl<'a> ProofContext<'a> {
    /// 创建新的证明上下文
    pub fn new(env: &'a TypeEnvironment) -> Self {
        Self {
            env,
            assumptions: AssumptionStack::new(),
            dep_graph: TypeDepGraph::new(),
            budget: BudgetTracker::new(),
        }
    }

    /// 获取类型环境引用
    pub fn env_ref(&self) -> &TypeEnvironment {
        self.env
    }
}
