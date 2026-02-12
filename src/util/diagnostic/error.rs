//! 诊断数据结构
//!
//! 提供统一的错误报告机制
//!
//! # 设计原则
//!
//! - `Diagnostic` 是运行时直接使用的结构，`message` 和 `help` 在编译期已渲染完成
//! - 所有构造方法都产生已渲染的最终字符串，AOT 二进制无需查表
//! - **只允许通过 `DiagnosticBuilder` 创建诊断**，禁止直接构造自定义错误
//!   所有错误码必须在注册表中注册
//!
//! # 创建方式
//!
//! ```ignore
//! // 方式 1: 通过 ErrorCodeDefinition 快捷方法
//! ErrorCodeDefinition::unknown_variable("x")
//!     .at(span)
//!     .build(&i18n);
//!
//! // 方式 2: 通过 error! 宏
//! error!(E1001, name = "x");
//!
//! // 方式 3: 通过注册表查找
//! ErrorCodeDefinition::find("E1001").unwrap()
//!     .builder()
//!     .param("name", "x")
//!     .at(span)
//!     .build(&i18n);
//! ```

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

/// 诊断信息（运行时直接使用，message 已渲染完成）
///
/// **不可直接构造**。必须通过 `DiagnosticBuilder::build()` 创建，
/// 确保所有错误码都经过注册表验证。
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// 严重级别
    pub severity: Severity,
    /// 错误码
    pub code: String,
    /// 完整消息（编译期已渲染）
    pub message: String,
    /// 帮助信息（编译期已渲染）
    pub help: String,
    /// 位置信息
    pub span: Option<Span>,
    /// 相关诊断
    pub related: Vec<Diagnostic>,
}

impl Diagnostic {
    /// 创建错误诊断（message 已渲染）
    ///
    /// `pub(crate)`: 仅由 `DiagnosticBuilder::build()` 调用，
    /// 外部代码必须通过注册表 + Builder 路径创建诊断。
    pub(crate) fn error(
        code: String,
        message: String,
        help: String,
        span: Option<Span>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message,
            help,
            span,
            related: Vec::new(),
        }
    }

    /// 创建警告诊断
    ///
    /// `pub(crate)`: 仅由 `DiagnosticBuilder::build()` 调用。
    pub(crate) fn warning(
        code: String,
        message: String,
        help: String,
        span: Option<Span>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message,
            help,
            span,
            related: Vec::new(),
        }
    }

    /// 添加相关诊断
    pub(crate) fn with_related(
        mut self,
        related: Vec<Diagnostic>,
    ) -> Self {
        self.related = related;
        self
    }
}

impl crate::util::span::SpannedError for Diagnostic {
    fn span(&self) -> crate::util::span::Span {
        self.span.unwrap_or_default()
    }
}

impl Default for Diagnostic {
    fn default() -> Self {
        Self {
            severity: Severity::Error,
            code: String::new(),
            message: String::new(),
            help: String::new(),
            span: None,
            related: Vec::new(),
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]: {}", self.severity, self.code, self.message)
    }
}
