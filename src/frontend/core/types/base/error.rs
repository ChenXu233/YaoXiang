//! 类型错误定义
//!
//! 实现类型系统中的错误类型：
//! - TypeMismatch: 类型不匹配错误
//! - TypeConstraintError: 约束求解错误

use super::mono::MonoType;
use crate::util::span::Span;

/// 类型不匹配错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeMismatch {
    pub left: MonoType,
    pub right: MonoType,
    pub span: Span,
}

impl std::fmt::Display for TypeMismatch {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "expected {}, found {}",
            self.left.type_name(),
            self.right.type_name()
        )
    }
}

/// 约束求解错误
#[derive(Debug, Clone)]
pub struct TypeConstraintError {
    pub error: TypeMismatch,
    pub span: Span,
}

impl std::fmt::Display for TypeConstraintError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{} at {:?}", self.error, self.span)
    }
}
