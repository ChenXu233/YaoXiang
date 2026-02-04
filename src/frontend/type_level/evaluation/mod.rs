//! RFC-011 类型级计算引擎
//!
//! 提供类型级计算的完整能力：
//! - normalize: 类型范式化
//! - reduce: 类型归约
//! - unify: 类型级统一
//! - compute: 类型计算引擎

pub mod compute;
pub mod normalize;
pub mod reduce;
pub mod unify;

// 重新导出主要类型
pub use normalize::{TypeNormalizer, NormalizationContext};
pub use reduce::{TypeReducer, ReductionResult};
pub use unify::{TypeUnifier, UnificationResult};
pub use compute::TypeComputer;

/// 范式类型标记
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NormalForm {
    /// 已范式化
    Normalized,

    /// 需要进一步归约
    NeedsReduction,

    /// 无法范式化
    Stuck,
}

/// 归约步
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReductionStep {
    /// Beta 归约
    Beta,

    /// Eta 归约
    Eta,

    /// Delta 归约（类型展开）
    Delta,

    /// Iota 归约（模式匹配）
    Iota,

    /// 自定义归约
    Custom(String),
}

/// 归约配置
#[derive(Debug, Clone)]
pub struct ReductionConfig {
    /// 最大归约步数
    pub max_steps: usize,

    /// 是否启用 Delta 归约
    pub enable_delta: bool,

    /// 是否启用 Iota 归约
    pub enable_iota: bool,

    /// 是否启用求值策略
    pub evaluation_strategy: EvaluationStrategy,
}

/// 求值策略
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationStrategy {
    /// 惰性求值（仅在需要时归约）
    Lazy,

    /// 急切求值（立即完全归约）
    Eager,

    /// 按需求值
    Demand,
}

impl Default for ReductionConfig {
    fn default() -> Self {
        Self {
            max_steps: 1000,
            enable_delta: true,
            enable_iota: true,
            evaluation_strategy: EvaluationStrategy::Lazy,
        }
    }
}
