//! 类型代数操作
//!
//! 对类型的纯函数操作。输入类型，输出归约后的类型或错误。
//! 不涉及程序上下文、不涉及证明。

pub mod conditional;
pub mod const_eval;
pub mod dependent_types;
pub mod normalizer;
pub mod operations;
pub mod reducer;
pub mod type_families;

#[cfg(test)]
mod tests;

// TypeLevelResult / TypeLevelError (从原 types/computation/mod.rs 迁移)

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
