//! 进度事件（LSP 支持核心）

use super::{Event, EventMetadata, EventType};
use crate::util::span::{Span, Position};

/// 进度工作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkType {
    /// 语法分析
    Syntax,
    /// 类型检查
    TypeCheck,
    /// 代码补全
    Completion,
    /// 代码跳转
    GotoDefinition,
    /// 查找引用
    FindReferences,
    /// 重构
    Refactor,
    /// 诊断
    Diagnostics,
    /// 编译
    Compile,
}

impl std::fmt::Display for WorkType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            WorkType::Syntax => write!(f, "Syntax Analysis"),
            WorkType::TypeCheck => write!(f, "Type Checking"),
            WorkType::Completion => write!(f, "Code Completion"),
            WorkType::GotoDefinition => write!(f, "Go to Definition"),
            WorkType::FindReferences => write!(f, "Find References"),
            WorkType::Refactor => write!(f, "Refactoring"),
            WorkType::Diagnostics => write!(f, "Diagnostics"),
            WorkType::Compile => write!(f, "Compiling"),
        }
    }
}

/// 进度开始事件
#[derive(Debug, Clone)]
pub struct ProgressStart {
    work_type: WorkType,
    title: String,
    message: Option<String>,
    metadata: EventMetadata,
}

impl ProgressStart {
    pub fn new(
        work_type: WorkType,
        title: impl Into<String>,
    ) -> Self {
        Self {
            work_type,
            title: title.into(),
            message: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn with_message(
        mut self,
        message: impl Into<String>,
    ) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn work_type(&self) -> WorkType {
        self.work_type
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

impl Event for ProgressStart {
    fn event_type(&self) -> EventType {
        EventType::Progress
    }

    fn name(&self) -> &'static str {
        "ProgressStart"
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

/// 进度更新事件
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    work_type: WorkType,
    percentage: Option<f64>,
    message: Option<String>,
    current_position: Option<Position>,
    metadata: EventMetadata,
}

impl ProgressUpdate {
    pub fn new(work_type: WorkType) -> Self {
        Self {
            work_type,
            percentage: None,
            message: None,
            current_position: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn with_percentage(
        mut self,
        percentage: f64,
    ) -> Self {
        self.percentage = Some(percentage.clamp(0.0, 100.0));
        self
    }

    pub fn with_message(
        mut self,
        message: impl Into<String>,
    ) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_position(
        mut self,
        position: Position,
    ) -> Self {
        self.current_position = Some(position);
        self
    }

    pub fn work_type(&self) -> WorkType {
        self.work_type
    }

    pub fn percentage(&self) -> Option<f64> {
        self.percentage
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn current_position(&self) -> Option<Position> {
        self.current_position
    }
}

impl Event for ProgressUpdate {
    fn event_type(&self) -> EventType {
        EventType::Progress
    }

    fn name(&self) -> &'static str {
        "ProgressUpdate"
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

/// 进度结束事件
#[derive(Debug, Clone)]
pub struct ProgressEnd {
    work_type: WorkType,
    success: bool,
    message: Option<String>,
    metadata: EventMetadata,
}

impl ProgressEnd {
    pub fn new(
        work_type: WorkType,
        success: bool,
    ) -> Self {
        Self {
            work_type,
            success,
            message: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn with_message(
        mut self,
        message: impl Into<String>,
    ) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn work_type(&self) -> WorkType {
        self.work_type
    }

    pub fn success(&self) -> bool {
        self.success
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

impl Event for ProgressEnd {
    fn event_type(&self) -> EventType {
        EventType::Progress
    }

    fn name(&self) -> &'static str {
        "ProgressEnd"
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

/// 语法分析进度事件（细粒度）
#[derive(Debug, Clone)]
pub struct SyntaxAnalysisProgress {
    current_line: usize,
    total_lines: usize,
    current_span: Option<Span>,
    metadata: EventMetadata,
}

impl SyntaxAnalysisProgress {
    pub fn new(
        current_line: usize,
        total_lines: usize,
    ) -> Self {
        Self {
            current_line,
            total_lines,
            current_span: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn with_span(
        mut self,
        span: Span,
    ) -> Self {
        self.current_span = Some(span);
        self
    }

    pub fn current_line(&self) -> usize {
        self.current_line
    }

    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    pub fn percentage(&self) -> f64 {
        if self.total_lines == 0 {
            0.0
        } else {
            (self.current_line as f64 / self.total_lines as f64) * 100.0
        }
    }
}

impl Event for SyntaxAnalysisProgress {
    fn event_type(&self) -> EventType {
        EventType::Progress
    }

    fn name(&self) -> &'static str {
        "SyntaxAnalysisProgress"
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
        self.current_span
    }
}

/// 类型检查进度事件（细粒度）
#[derive(Debug, Clone)]
pub struct TypeCheckProgress {
    current_item: String,
    total_items: usize,
    current_index: usize,
    inferred_count: usize,
    error_count: usize,
    warning_count: usize,
    current_span: Option<Span>,
    metadata: EventMetadata,
}

impl TypeCheckProgress {
    pub fn new(
        current_item: impl Into<String>,
        total_items: usize,
        current_index: usize,
    ) -> Self {
        Self {
            current_item: current_item.into(),
            total_items,
            current_index,
            inferred_count: 0,
            error_count: 0,
            warning_count: 0,
            current_span: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn with_span(
        mut self,
        span: Span,
    ) -> Self {
        self.current_span = Some(span);
        self
    }

    pub fn with_stats(
        mut self,
        inferred_count: usize,
        error_count: usize,
        warning_count: usize,
    ) -> Self {
        self.inferred_count = inferred_count;
        self.error_count = error_count;
        self.warning_count = warning_count;
        self
    }

    pub fn current_item(&self) -> &str {
        &self.current_item
    }

    pub fn total_items(&self) -> usize {
        self.total_items
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn percentage(&self) -> f64 {
        if self.total_items == 0 {
            0.0
        } else {
            (self.current_index as f64 / self.total_items as f64) * 100.0
        }
    }

    pub fn inferred_count(&self) -> usize {
        self.inferred_count
    }

    pub fn error_count(&self) -> usize {
        self.error_count
    }

    pub fn warning_count(&self) -> usize {
        self.warning_count
    }
}

impl Event for TypeCheckProgress {
    fn event_type(&self) -> EventType {
        EventType::Progress
    }

    fn name(&self) -> &'static str {
        "TypeCheckProgress"
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
        self.current_span
    }
}

/// 取消事件
#[derive(Debug, Clone)]
pub struct Cancel {
    work_type: Option<WorkType>,
    reason: Option<String>,
    metadata: EventMetadata,
}

impl Default for Cancel {
    fn default() -> Self {
        Self::new()
    }
}

impl Cancel {
    pub fn new() -> Self {
        Self {
            work_type: None,
            reason: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn for_work_type(
        mut self,
        work_type: WorkType,
    ) -> Self {
        self.work_type = Some(work_type);
        self
    }

    pub fn with_reason(
        mut self,
        reason: impl Into<String>,
    ) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn work_type(&self) -> Option<WorkType> {
        self.work_type
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

impl Event for Cancel {
    fn event_type(&self) -> EventType {
        EventType::Progress
    }

    fn name(&self) -> &'static str {
        "Cancel"
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
