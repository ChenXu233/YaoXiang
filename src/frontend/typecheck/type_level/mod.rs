//! 类型级计算模块 (RFC-011 Phase 5)
//!
//! 实现条件类型、类型族和依赖类型等高级类型系统特性

pub mod conditional_types;
pub mod dependent_types;
pub mod type_computer;
pub mod type_family;

#[cfg(test)]
mod tests;

// Re-exports
pub use type_computer::{TypeLevelComputer, TypeFamily, TypeNormalizer, Nat, TypeLevelValue};
pub use conditional_types::{
    ConditionalType, ConditionalTypeChecker, TypeMatch, TypeMatchArm, EvalResult,
};
pub use type_family::{TypeLevelArithmetic, TypeLevelComparison, TypeLevelArithmeticProcessor};
pub use dependent_types::{DependentType, DependentTypeChecker, DependentTypeBuilder, VectorOps};

// Error types
#[derive(Debug, Clone)]
pub enum TypeLevelError {
    /// 类型计算错误
    ComputationFailed {
        reason: String,
        span: crate::util::span::Span,
    },
    /// 条件类型错误
    ConditionalTypeError {
        reason: String,
        span: crate::util::span::Span,
    },
    /// 类型族错误
    TypeFamilyError {
        reason: String,
        span: crate::util::span::Span,
    },
    /// 依赖类型错误
    DependentTypeError {
        reason: String,
        span: crate::util::span::Span,
    },
}

impl std::fmt::Display for TypeLevelError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            TypeLevelError::ComputationFailed { reason, .. } => {
                write!(f, "Type-level computation failed: {}", reason)
            }
            TypeLevelError::ConditionalTypeError { reason, .. } => {
                write!(f, "Conditional type error: {}", reason)
            }
            TypeLevelError::TypeFamilyError { reason, .. } => {
                write!(f, "Type family error: {}", reason)
            }
            TypeLevelError::DependentTypeError { reason, .. } => {
                write!(f, "Dependent type error: {}", reason)
            }
        }
    }
}

impl std::error::Error for TypeLevelError {}
