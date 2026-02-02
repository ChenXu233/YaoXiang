//! 错误收集和报告
//!
//! 定义类型检查过程中的所有错误类型

use crate::frontend::core::type_system::MonoType;
use crate::frontend::shared::error::{Diagnostic, ErrorCollector, SpannedError};
use crate::util::span::Span;
use crate::util::i18n::{t_cur, MSG};
use thiserror::Error;

/// 实现 SpannedError 接口，使 TypeError 兼容错误收集器
impl SpannedError for TypeError {
    fn span(&self) -> Span {
        TypeError::span(self)
    }
}

/// 类型错误
///
/// 包含所有可能的类型检查错误
#[derive(Debug, Error, Clone)]
pub enum TypeError {
    /// 类型不匹配错误
    #[error("Type mismatch: expected {expected:?}, found {found:?}")]
    TypeMismatch {
        expected: Box<MonoType>,
        found: Box<MonoType>,
        span: Span,
    },

    /// 未知变量错误
    #[error("Unknown variable: {name}")]
    UnknownVariable { name: String, span: Span },

    /// 未知类型错误
    #[error("Unknown type: {name}")]
    UnknownType { name: String, span: Span },

    /// 参数数量不匹配错误
    #[error("Arity mismatch: expected {expected} arguments, found {found}")]
    ArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },

    /// 递归类型定义错误
    #[error("Recursive type definition: {name}")]
    RecursiveType { name: String, span: Span },

    /// 不支持的操作错误
    #[error("Unsupported operation: {op}")]
    UnsupportedOp { op: String, span: Span },

    /// 泛型约束违反错误
    #[error("Generic constraint violated: {constraint}")]
    GenericConstraint { constraint: String, span: Span },

    /// 无限类型错误
    #[error("Infinite type: {var} = {ty}")]
    InfiniteType {
        var: String,
        ty: Box<MonoType>,
        span: Span,
    },

    /// 未实例化的类型变量错误
    #[error("Unbound type variable: {var}")]
    UnboundTypeVar { var: String, span: Span },

    /// 未知标签错误（break/continue）
    #[error("Unknown label: {name}")]
    UnknownLabel { name: String, span: Span },

    /// 未知字段错误
    #[error("Unknown field: {field_name} in {struct_name}")]
    UnknownField {
        struct_name: String,
        field_name: String,
        span: Span,
    },

    /// 下标越界错误
    #[error("Index out of bounds: {index} (size: {size})")]
    IndexOutOfBounds {
        index: i128,
        size: usize,
        span: Span,
    },

    /// 函数调用错误
    #[error("Call error: {message}")]
    CallError { message: String, span: Span },

    /// 赋值错误
    #[error("Assignment error: {message}")]
    AssignmentError { message: String, span: Span },

    /// 类型推断错误
    #[error("Inference error: {message}")]
    InferenceError { message: String, span: Span },

    /// 无法推断参数类型错误
    #[error("Cannot infer type for parameter '{name}': parameter has no type annotation and is not used in a way that allows inference")]
    CannotInferParamType { name: String, span: Span },

    /// 模式匹配穷举性错误
    #[error("Non-exhaustive patterns: missing {} pattern(s)", .missing.len())]
    NonExhaustivePatterns { missing: Vec<String>, span: Span },

    /// 导入错误
    #[error("Import error: {message}")]
    ImportError { message: String, span: Span },

    /// 方法签名无效错误
    #[error("Invalid method signature: {method_name}")]
    InvalidMethodSignature { method_name: String, span: Span },

    /// 方法需要 self 参数错误
    #[error("Method '{method_name}' needs a self parameter")]
    MethodNeedsSelf { method_name: String, span: Span },

    /// self 类型不匹配错误
    #[error("Invalid self type for {method_name}: expected {expected:?}, found {found:?}")]
    InvalidSelfType {
        method_name: String,
        expected: Box<MonoType>,
        found: Box<MonoType>,
        span: Span,
    },
}

impl TypeError {
    /// 获取错误的位置
    pub fn span(&self) -> Span {
        match self {
            TypeError::TypeMismatch { span, .. } => *span,
            TypeError::UnknownVariable { span, .. } => *span,
            TypeError::UnknownType { span, .. } => *span,
            TypeError::ArityMismatch { span, .. } => *span,
            TypeError::RecursiveType { span, .. } => *span,
            TypeError::UnsupportedOp { span, .. } => *span,
            TypeError::GenericConstraint { span, .. } => *span,
            TypeError::InfiniteType { span, .. } => *span,
            TypeError::UnboundTypeVar { span, .. } => *span,
            TypeError::UnknownLabel { span, .. } => *span,
            TypeError::UnknownField { span, .. } => *span,
            TypeError::IndexOutOfBounds { span, .. } => *span,
            TypeError::CallError { span, .. } => *span,
            TypeError::AssignmentError { span, .. } => *span,
            TypeError::InferenceError { span, .. } => *span,
            TypeError::CannotInferParamType { span, .. } => *span,
            TypeError::NonExhaustivePatterns { span, .. } => *span,
            TypeError::ImportError { span, .. } => *span,
            TypeError::InvalidMethodSignature { span, .. } => *span,
            TypeError::MethodNeedsSelf { span, .. } => *span,
            TypeError::InvalidSelfType { span, .. } => *span,
        }
    }

    /// 获取国际化的错误消息
    pub fn to_i18n_message(&self) -> String {
        match self {
            TypeError::TypeMismatch {
                expected, found, ..
            } => t_cur(
                MSG::ErrorTypeMismatch,
                Some(&[&expected.type_name(), &found.type_name()]),
            ),
            TypeError::UnknownVariable { name, .. } => {
                t_cur(MSG::ErrorUnknownVariable, Some(&[name]))
            }
            TypeError::UnknownType { name, .. } => t_cur(MSG::ErrorUnknownType, Some(&[name])),
            TypeError::ArityMismatch {
                expected, found, ..
            } => t_cur(
                MSG::ErrorArityMismatch,
                Some(&[&expected.to_string(), &found.to_string()]),
            ),
            TypeError::RecursiveType { name, .. } => t_cur(MSG::ErrorRecursiveType, Some(&[name])),
            TypeError::UnsupportedOp { op, .. } => t_cur(MSG::ErrorUnsupportedOp, Some(&[op])),
            TypeError::GenericConstraint { constraint, .. } => constraint.clone(),
            TypeError::InfiniteType { var, ty, .. } => {
                format!("{} = {}", var, ty.type_name())
            }
            TypeError::UnboundTypeVar { var, .. } => var.clone(),
            TypeError::UnknownLabel { name, .. } => {
                format!("Unknown label: {}", name)
            }
            TypeError::UnknownField {
                struct_name,
                field_name,
                ..
            } => t_cur(MSG::ErrorUnknownField, Some(&[field_name, struct_name])),
            TypeError::IndexOutOfBounds { index, size, .. } => t_cur(
                MSG::ErrorIndexOutOfBounds,
                Some(&[&index.to_string(), &size.to_string()]),
            ),
            TypeError::CallError { message, .. } => message.clone(),
            TypeError::AssignmentError { message, .. } => message.clone(),
            TypeError::InferenceError { message, .. } => {
                t_cur(MSG::ErrorInferenceFailed, Some(&[message]))
            }
            TypeError::CannotInferParamType { name, .. } => {
                t_cur(MSG::ErrorCannotInferParamType, Some(&[name]))
            }
            TypeError::NonExhaustivePatterns { missing, .. } => t_cur(
                MSG::ErrorNonExhaustivePatterns,
                Some(&[&missing.len().to_string()]),
            ),
            TypeError::ImportError { message, .. } => {
                t_cur(MSG::ErrorImportError, Some(&[message]))
            }
            TypeError::InvalidMethodSignature { method_name, .. } => {
                format!("Invalid method signature: {}", method_name)
            }
            TypeError::MethodNeedsSelf { method_name, .. } => {
                format!("Method '{}' needs a self parameter", method_name)
            }
            TypeError::InvalidSelfType {
                method_name,
                expected,
                found,
                ..
            } => {
                format!(
                    "Invalid self type for {}: expected {:?}, found {:?}",
                    method_name, expected, found
                )
            }
        }
    }

    /// 获取智能建议（目前主要针对 UnknownVariable）
    pub fn get_suggestions(
        &self,
        _scope_vars: Option<&[String]>,
    ) -> Option<Vec<String>> {
        match self {
            TypeError::UnknownVariable { .. } => {
                // TODO: 实现作用域变量查找
                // 目前返回空的建议列表
                Some(Vec::new())
            }
            _ => None,
        }
    }

    /// 获取错误代码（用于错误编号）
    pub fn error_code(&self) -> &'static str {
        match self {
            TypeError::TypeMismatch { .. } => "E0001",
            TypeError::UnknownVariable { .. } => "E0002",
            TypeError::UnknownType { .. } => "E0003",
            TypeError::ArityMismatch { .. } => "E0004",
            TypeError::RecursiveType { .. } => "E0005",
            TypeError::UnsupportedOp { .. } => "E0006",
            TypeError::GenericConstraint { .. } => "E0007",
            TypeError::InfiniteType { .. } => "E0008",
            TypeError::UnboundTypeVar { .. } => "E0009",
            TypeError::UnknownLabel { .. } => "E0010",
            TypeError::UnknownField { .. } => "E0011",
            TypeError::IndexOutOfBounds { .. } => "E0012",
            TypeError::CallError { .. } => "E0013",
            TypeError::AssignmentError { .. } => "E0014",
            TypeError::InferenceError { .. } => "E0015",
            TypeError::CannotInferParamType { .. } => "E0016",
            TypeError::NonExhaustivePatterns { .. } => "E0017",
            TypeError::ImportError { .. } => "E0018",
            TypeError::InvalidMethodSignature { .. } => "E0019",
            TypeError::MethodNeedsSelf { .. } => "E0020",
            TypeError::InvalidSelfType { .. } => "E0021",
        }
    }

    /// 创建类型不匹配错误
    pub fn type_mismatch(
        expected: Box<MonoType>,
        found: Box<MonoType>,
        span: Span,
    ) -> Self {
        TypeError::TypeMismatch {
            expected,
            found,
            span,
        }
    }

    /// 创建未知变量错误
    pub fn unknown_variable(
        name: String,
        span: Span,
    ) -> Self {
        TypeError::UnknownVariable { name, span }
    }

    /// 创建未知类型错误
    pub fn unknown_type(
        name: String,
        span: Span,
    ) -> Self {
        TypeError::UnknownType { name, span }
    }

    /// 创建参数数量不匹配错误
    pub fn arity_mismatch(
        expected: usize,
        found: usize,
        span: Span,
    ) -> Self {
        TypeError::ArityMismatch {
            expected,
            found,
            span,
        }
    }

    /// 创建导入错误
    pub fn import_error(
        message: String,
        span: Span,
    ) -> Self {
        TypeError::ImportError { message, span }
    }
}

/// 类型推断结果
pub type TypeResult<T> = Result<T, TypeError>;

/// 类型错误收集器
pub type TypeErrorCollector = ErrorCollector<TypeError>;

/// 为 TypeError 实现 Default（用于 ErrorCollector derive）
impl Default for TypeError {
    fn default() -> Self {
        TypeError::InferenceError {
            message: "Unknown inference error".to_string(),
            span: Span::default(),
        }
    }
}

/// 从错误生成诊断
impl From<TypeError> for Diagnostic {
    fn from(error: TypeError) -> Self {
        let span = Some(error.span());
        match &error {
            TypeError::TypeMismatch { .. } => {
                Diagnostic::error("E0001".to_string(), format!("{}", error), span)
            }
            TypeError::UnknownVariable { .. } => {
                Diagnostic::error("E0002".to_string(), format!("{}", error), span)
            }
            TypeError::UnknownType { .. } => {
                Diagnostic::error("E0003".to_string(), format!("{}", error), span)
            }
            TypeError::ArityMismatch { .. } => {
                Diagnostic::error("E0004".to_string(), format!("{}", error), span)
            }
            TypeError::RecursiveType { .. } => {
                Diagnostic::error("E0005".to_string(), format!("{}", error), span)
            }
            TypeError::UnsupportedOp { .. } => {
                Diagnostic::error("E0006".to_string(), format!("{}", error), span)
            }
            TypeError::GenericConstraint { .. } => {
                Diagnostic::error("E0007".to_string(), format!("{}", error), span)
            }
            TypeError::InfiniteType { .. } => {
                Diagnostic::error("E0008".to_string(), format!("{}", error), span)
            }
            TypeError::UnboundTypeVar { .. } => {
                Diagnostic::error("E0009".to_string(), format!("{}", error), span)
            }
            TypeError::UnknownLabel { .. } => {
                Diagnostic::error("E0010".to_string(), format!("{}", error), span)
            }
            TypeError::UnknownField { .. } => {
                Diagnostic::error("E0011".to_string(), format!("{}", error), span)
            }
            TypeError::IndexOutOfBounds { .. } => {
                Diagnostic::error("E0012".to_string(), format!("{}", error), span)
            }
            TypeError::CallError { .. } => {
                Diagnostic::error("E0013".to_string(), format!("{}", error), span)
            }
            TypeError::AssignmentError { .. } => {
                Diagnostic::error("E0014".to_string(), format!("{}", error), span)
            }
            TypeError::InferenceError { .. } => {
                Diagnostic::error("E0015".to_string(), format!("{}", error), span)
            }
            TypeError::CannotInferParamType { .. } => {
                Diagnostic::error("E0016".to_string(), format!("{}", error), span)
            }
            TypeError::NonExhaustivePatterns { .. } => {
                Diagnostic::error("E0017".to_string(), format!("{}", error), span)
            }
            TypeError::ImportError { .. } => {
                Diagnostic::error("E0018".to_string(), format!("{}", error), span)
            }
            TypeError::InvalidMethodSignature { .. } => {
                Diagnostic::error("E0019".to_string(), format!("{}", error), span)
            }
            TypeError::MethodNeedsSelf { .. } => {
                Diagnostic::error("E0020".to_string(), format!("{}", error), span)
            }
            TypeError::InvalidSelfType { .. } => {
                Diagnostic::error("E0021".to_string(), format!("{}", error), span)
            }
        }
    }
}
