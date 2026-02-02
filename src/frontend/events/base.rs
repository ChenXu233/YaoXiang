//! 基础事件类型

use crate::util::span::Span;
use super::{Event, EventMetadata, EventType};

/// 编译阶段枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationPhase {
    /// 词法分析
    Lexing,
    /// 语法分析
    Parsing,
    /// 类型检查
    TypeChecking,
    /// IR 生成
    IRGeneration,
    /// 完整编译
    Full,
}

impl std::fmt::Display for CompilationPhase {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            CompilationPhase::Lexing => write!(f, "lexing"),
            CompilationPhase::Parsing => write!(f, "parsing"),
            CompilationPhase::TypeChecking => write!(f, "type checking"),
            CompilationPhase::IRGeneration => write!(f, "IR generation"),
            CompilationPhase::Full => write!(f, "full compilation"),
        }
    }
}

/// 阶段开始事件
#[derive(Debug, Clone)]
pub struct PhaseStart {
    phase: CompilationPhase,
    metadata: EventMetadata,
}

impl PhaseStart {
    pub fn new(phase: CompilationPhase) -> Self {
        Self {
            phase,
            metadata: EventMetadata::default(),
        }
    }

    pub fn phase(&self) -> CompilationPhase {
        self.phase
    }
}

impl Event for PhaseStart {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "PhaseStart"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// 阶段完成事件
#[derive(Debug, Clone)]
pub struct PhaseComplete {
    phase: CompilationPhase,
    duration_ms: u64,
    metadata: EventMetadata,
}

impl PhaseComplete {
    pub fn new(
        phase: CompilationPhase,
        duration_ms: u64,
    ) -> Self {
        Self {
            phase,
            duration_ms,
            metadata: EventMetadata::default(),
        }
    }

    pub fn phase(&self) -> CompilationPhase {
        self.phase
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }
}

impl Event for PhaseComplete {
    fn event_type(&self) -> EventType {
        EventType::Phase
    }

    fn name(&self) -> &'static str {
        "PhaseComplete"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// 错误级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLevel {
    /// 错误
    Error,
    /// 警告
    Warning,
    /// 信息
    Info,
    /// 提示
    Hint,
}

/// 错误发生事件
#[derive(Debug, Clone)]
pub struct ErrorOccurred {
    message: String,
    error_code: String,
    level: ErrorLevel,
    span: Option<Span>,
    metadata: EventMetadata,
}

impl ErrorOccurred {
    pub fn new(
        message: impl Into<String>,
        error_code: impl Into<String>,
        level: ErrorLevel,
    ) -> Self {
        Self {
            message: message.into(),
            error_code: error_code.into(),
            level,
            span: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn with_span(
        mut self,
        span: Span,
    ) -> Self {
        self.span = Some(span);
        self
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn error_code(&self) -> &str {
        &self.error_code
    }

    pub fn level(&self) -> ErrorLevel {
        self.level
    }
}

impl Event for ErrorOccurred {
    fn event_type(&self) -> EventType {
        EventType::Base
    }

    fn name(&self) -> &'static str {
        "ErrorOccurred"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }

    fn span(&self) -> Option<Span> {
        self.span
    }
}

/// 警告发生事件
#[derive(Debug, Clone)]
pub struct WarningOccurred {
    message: String,
    warning_code: String,
    span: Option<Span>,
    metadata: EventMetadata,
}

impl WarningOccurred {
    pub fn new(
        message: impl Into<String>,
        warning_code: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            warning_code: warning_code.into(),
            span: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn with_span(
        mut self,
        span: Span,
    ) -> Self {
        self.span = Some(span);
        self
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn warning_code(&self) -> &str {
        &self.warning_code
    }
}

impl Event for WarningOccurred {
    fn event_type(&self) -> EventType {
        EventType::Base
    }

    fn name(&self) -> &'static str {
        "WarningOccurred"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }

    fn span(&self) -> Option<Span> {
        self.span
    }
}

/// 信息性事件（用于日志记录）
#[derive(Debug, Clone)]
pub struct InfoEvent {
    message: String,
    source: &'static str,
    metadata: EventMetadata,
}

impl InfoEvent {
    pub fn new(
        message: impl Into<String>,
        source: &'static str,
    ) -> Self {
        Self {
            message: message.into(),
            source,
            metadata: EventMetadata::default(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn source(&self) -> &'static str {
        self.source
    }
}

impl Event for InfoEvent {
    fn event_type(&self) -> EventType {
        EventType::Base
    }

    fn name(&self) -> &'static str {
        "InfoEvent"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}

/// 取消请求事件
#[derive(Debug, Clone)]
pub struct CancellationRequested {
    reason: Option<String>,
    metadata: EventMetadata,
}

impl Default for CancellationRequested {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationRequested {
    pub fn new() -> Self {
        Self {
            reason: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn with_reason(
        mut self,
        reason: impl Into<String>,
    ) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

impl Event for CancellationRequested {
    fn event_type(&self) -> EventType {
        EventType::Base
    }

    fn name(&self) -> &'static str {
        "CancellationRequested"
    }

    fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    ) {
        self.metadata = metadata;
    }
}
