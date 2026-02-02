//! 诊断数据结构
//!
//! 提供统一的错误报告机制

use crate::util::span::Span;

/// 诊断严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

impl Severity {
    /// 获取严重级别对应的数字值
    pub fn as_u8(&self) -> u8 {
        match self {
            Severity::Error => 4,
            Severity::Warning => 3,
            Severity::Info => 2,
            Severity::Hint => 1,
        }
    }

    /// 检查是否为错误级别
    pub fn is_error(&self) -> bool {
        matches!(self, Severity::Error)
    }
}

impl From<i32> for Severity {
    fn from(val: i32) -> Self {
        match val {
            1 => Severity::Error,
            2 => Severity::Warning,
            3 => Severity::Info,
            4 => Severity::Hint,
            _ => Severity::Info,
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
            Severity::Hint => write!(f, "hint"),
        }
    }
}

/// 诊断信息
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// 严重程度
    pub severity: Severity,
    /// 错误代码
    pub code: String,
    /// 消息
    pub message: String,
    /// 位置
    pub span: Option<Span>,
    /// 相关位置
    pub related: Vec<Diagnostic>,
}

impl Diagnostic {
    /// 创建新的诊断信息
    pub fn new(
        severity: Severity,
        code: String,
        message: String,
        span: Option<Span>,
    ) -> Self {
        Self {
            severity,
            code,
            message,
            span,
            related: Vec::new(),
        }
    }

    /// 创建错误诊断
    pub fn error(
        code: String,
        message: String,
        span: Option<Span>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message,
            span,
            related: Vec::new(),
        }
    }

    /// 创建警告诊断
    pub fn warning(
        code: String,
        message: String,
        span: Option<Span>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message,
            span,
            related: Vec::new(),
        }
    }

    /// 添加相关诊断
    pub fn with_related(
        mut self,
        related: Vec<Diagnostic>,
    ) -> Self {
        self.related = related;
        self
    }
}

/// 诊断构建器
pub struct DiagnosticBuilder {
    severity: Severity,
    code: String,
    message: String,
    span: Option<Span>,
    related: Vec<Diagnostic>,
}

impl DiagnosticBuilder {
    pub fn new(
        severity: Severity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: String::new(),
            message: message.into(),
            span: None,
            related: Vec::new(),
        }
    }

    pub fn with_code(
        mut self,
        code: String,
    ) -> Self {
        self.code = code;
        self
    }

    pub fn with_span(
        mut self,
        span: Span,
    ) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_related(
        mut self,
        related: Vec<Diagnostic>,
    ) -> Self {
        self.related = related;
        self
    }

    pub fn build(self) -> Diagnostic {
        Diagnostic {
            severity: self.severity,
            code: self.code,
            message: self.message,
            span: self.span,
            related: self.related,
        }
    }
}
