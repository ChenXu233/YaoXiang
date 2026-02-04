//! RFC-011 高级类型层
//!
//! 提供类型级计算能力，支持：
//! - 条件类型 (If, Match)
//! - 类型级运算 (算术、比较、逻辑)
//! - Const泛型支持
//! - 类型范式化和归约
//! - Trait 边界和约束求解

pub mod conditional_types;
pub mod const_generics;
pub mod dependent_types;
pub mod derive;
pub mod evaluation;
pub mod impl_check;
pub mod inheritance;
pub mod operations;
pub mod trait_bounds;
pub mod type_families;
pub mod type_match;

// 重新导出主要类型
pub use conditional_types::{If, MatchType, ConditionalType};
pub use evaluation::{TypeNormalizer, TypeReducer, TypeComputer, NormalForm};
pub use operations::{TypeArithmetic, TypeComparison, TypeLogic};
pub use const_generics::{ConstGenericEval, GenericSize};
pub use trait_bounds::{
    TraitDefinition, TraitBound, TraitTable, TraitImplementation, TraitSolver, TraitSolverError,
};
pub use inheritance::{InheritanceChecker, InheritanceError, TraitInheritanceGraph};
pub use impl_check::{TraitImplChecker, TraitImplError};
pub use derive::{DeriveParser, DeriveGenerator};
pub use type_families::{Bool, Nat, IsTrue, IsFalse, IsZero, IsSucc, bool_family, nat_family};
pub use type_match::{MatchPattern, MatchArm, PatternMatcher, MatchBinding, PatternBuilder};

/// 高级类型错误
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TypeLevelError {
    #[error("Type computation failed: {0}")]
    ComputationFailed(String),

    #[error("Normalization failed: {0}")]
    NormalizationFailed(String),

    #[error("Const evaluation failed: {0}")]
    ConstEvalFailed(String),

    #[error("Type operation not supported: {0}")]
    UnsupportedOperation(String),
}

/// 类型级计算结果
#[derive(Debug, Clone, PartialEq)]
pub enum TypeLevelResult<T> {
    /// 计算成功，结果已范式化
    Normalized(T),

    /// 计算成功，结果需要进一步归约
    Pending(T),

    /// 计算失败
    Error(TypeLevelError),
}

impl<T> TypeLevelResult<T> {
    /// 转换为 Option
    pub fn ok(self) -> Option<T> {
        match self {
            Self::Normalized(t) | Self::Pending(t) => Some(t),
            Self::Error(_) => None,
        }
    }

    /// 转换为 Result
    pub fn result(self) -> Result<T, TypeLevelError> {
        match self {
            Self::Normalized(t) | Self::Pending(t) => Ok(t),
            Self::Error(e) => Err(e),
        }
    }

    /// 检查是否已范式化
    pub fn is_normalized(&self) -> bool {
        matches!(self, Self::Normalized(_))
    }
}

#[cfg(test)]
pub mod tests;
