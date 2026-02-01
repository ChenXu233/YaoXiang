//! 统一诊断信息
//!
//! 为前端模块提供统一的错误报告机制

use crate::util::span::Span;

/// 诊断级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
    Help,
}

/// 诊断信息
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    /// 创建错误诊断
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            span: None,
        }
    }

    /// 创建警告诊断
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            span: None,
        }
    }
}

/// 诊断构建器
pub struct DiagnosticBuilder {
    level: DiagnosticLevel,
    message: String,
    span: Option<Span>,
}

impl DiagnosticBuilder {
    pub fn new(
        level: DiagnosticLevel,
        message: impl Into<String>,
    ) -> Self {
        Self {
            level,
            message: message.into(),
            span: None,
        }
    }

    pub fn with_span(
        mut self,
        span: Span,
    ) -> Self {
        self.span = Some(span);
        self
    }

    pub fn build(self) -> Diagnostic {
        Diagnostic {
            level: self.level,
            message: self.message,
            span: self.span,
        }
    }
}
