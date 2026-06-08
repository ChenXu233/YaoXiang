//! 类型错误定义
//!
//! 实现类型系统中的错误类型：
//! - TypeConstraintError: 约束求解错误

use crate::util::diagnostic::Diagnostic;
use crate::util::span::Span;

/// 约束求解错误
#[derive(Debug, Clone)]
pub struct TypeConstraintError {
    pub error: Diagnostic,
    pub span: Span,
}

impl std::fmt::Display for TypeConstraintError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{} at {:?}", self.error.message, self.span)
    }
}
