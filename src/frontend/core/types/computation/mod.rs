//! 类型级计算（RFC-011）
//!
//! 条件类型、Const泛型、类型级运算、类型族、模式匹配

pub mod conditional_types;
pub mod const_generics;
pub mod dependent_types;
pub mod evaluation;
pub mod operations;
pub mod type_families;
pub mod type_match;

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
    Normalized(T),
    Pending(T),
    Error(TypeLevelError),
}

impl<T> TypeLevelResult<T> {
    pub fn ok(self) -> Option<T> {
        match self {
            Self::Normalized(t) | Self::Pending(t) => Some(t),
            Self::Error(_) => None,
        }
    }

    pub fn result(self) -> Result<T, TypeLevelError> {
        match self {
            Self::Normalized(t) | Self::Pending(t) => Ok(t),
            Self::Error(e) => Err(e),
        }
    }

    pub fn is_normalized(&self) -> bool {
        matches!(self, Self::Normalized(_))
    }
}
