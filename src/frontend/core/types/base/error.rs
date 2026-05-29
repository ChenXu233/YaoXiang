//! 类型错误定义
//!
//! 实现类型系统中的错误类型：
//! - TypeMismatch: 类型不匹配错误
//! - TypeConstraintError: 约束求解错误
//! - ConstEvalError: Const求值错误

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

/// Const求值错误
#[derive(Debug, Clone)]
pub enum ConstEvalError {
    /// 除零错误
    DivisionByZero { span: Span },
    /// 整数溢出
    Overflow {
        value: String,
        ty: String,
        span: Span,
    },
    /// 未定义的Const变量
    UndefinedVariable { name: String, span: Span },
    /// 递归深度超限
    RecursionTooDeep {
        depth: usize,
        max_depth: usize,
        span: Span,
    },
    /// 类型不匹配
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },
    /// 参数数量不匹配
    ArgCountMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },
    /// 非Const函数调用
    NonConstFunctionCall { func: String, span: Span },
    /// 无法求值
    CannotEvaluate { reason: String, span: Span },
}

impl std::fmt::Display for ConstEvalError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ConstEvalError::DivisionByZero { .. } => {
                write!(f, "division by zero")
            }
            ConstEvalError::Overflow { value, ty, .. } => {
                write!(f, "integer overflow: value {} overflows type {}", value, ty)
            }
            ConstEvalError::UndefinedVariable { name, .. } => {
                write!(f, "undefined constant variable: {}", name)
            }
            ConstEvalError::RecursionTooDeep {
                depth, max_depth, ..
            } => {
                write!(
                    f,
                    "constant evaluation exceeded recursion limit: {} > {}",
                    depth, max_depth
                )
            }
            ConstEvalError::TypeMismatch {
                expected, found, ..
            } => {
                write!(f, "type mismatch: expected {}, found {}", expected, found)
            }
            ConstEvalError::ArgCountMismatch {
                expected, found, ..
            } => {
                write!(
                    f,
                    "argument count mismatch: expected {}, found {}",
                    expected, found
                )
            }
            ConstEvalError::NonConstFunctionCall { func, .. } => {
                write!(
                    f,
                    "cannot call non-const function '{}' in constant expression",
                    func
                )
            }
            ConstEvalError::CannotEvaluate { reason, .. } => {
                write!(f, "cannot evaluate constant expression: {}", reason)
            }
        }
    }
}
