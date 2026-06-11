//! 证明上下文
//!
//! 在编译期证明管道中传递的共享状态。
//! 搬迁阶段只包含最小字段，RFC-027 实现时加入路径条件栈和依赖图。

use crate::frontend::core::typecheck::environment::TypeEnvironment;

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
}

impl<'a> ProofContext<'a> {
    /// 获取类型环境引用
    pub fn env_ref(&self) -> &TypeEnvironment {
        self.env
    }
}
