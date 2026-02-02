//! 诊断事件

use super::{Event, EventMetadata, EventType};
use crate::util::span::Span;
use crate::util::diagnostic::Severity;

/// 诊断代码（用于标识特定错误类型）
#[derive(Debug, Clone)]
pub struct DiagnosticCode {
    code: String,
    /// 错误类别（如 "E" 表示错误，"W" 表示警告）
    category: char,
}

impl DiagnosticCode {
    pub fn new(
        category: char,
        code: impl Into<String>,
    ) -> Self {
        Self {
            category,
            code: code.into(),
        }
    }

    pub fn from_string(code: &str) -> Self {
        if code.is_empty() {
            return Self {
                category: 'E',
                code: code.to_string(),
            };
        }
        let category = code.chars().next().unwrap_or('E');
        let code_str = code[1..].to_string();
        Self {
            category,
            code: code_str,
        }
    }

    pub fn as_string(&self) -> String {
        format!("{}{}", self.category, self.code)
    }

    pub fn category(&self) -> char {
        self.category
    }
}

/// 诊断相关代码（Related Diagnostic）
#[derive(Debug, Clone)]
pub struct RelatedDiagnostic {
    span: Span,
    message: String,
    code: Option<DiagnosticCode>,
}

impl RelatedDiagnostic {
    pub fn new(
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Self {
            span,
            message: message.into(),
            code: None,
        }
    }

    pub fn with_code(
        mut self,
        code: DiagnosticCode,
    ) -> Self {
        self.code = Some(code);
        self
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn code(&self) -> Option<&DiagnosticCode> {
        self.code.as_ref()
    }
}

/// 诊断标签（用于代码操作）
#[derive(Debug, Clone)]
pub struct DiagnosticTag {
    tag: DiagnosticTagKind,
    message: Option<String>,
}

impl DiagnosticTag {
    pub fn new(tag: DiagnosticTagKind) -> Self {
        Self { tag, message: None }
    }

    pub fn with_message(
        mut self,
        message: impl Into<String>,
    ) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn tag(&self) -> DiagnosticTagKind {
        self.tag
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

/// 诊断标签类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticTagKind {
    /// 不必要的代码
    Unnecessary,
    /// 已弃用的代码
    Deprecated,
    /// 错误但被抑制的代码
    Suppressed,
}

/// 诊断代码操作
#[derive(Debug, Clone)]
pub struct CodeAction {
    title: String,
    kind: CodeActionKind,
    edit: Option<TextEdit>,
    command: Option<Command>,
    is_preferred: bool,
}

impl CodeAction {
    pub fn new(
        title: impl Into<String>,
        kind: CodeActionKind,
    ) -> Self {
        Self {
            title: title.into(),
            kind,
            edit: None,
            command: None,
            is_preferred: false,
        }
    }

    pub fn with_edit(
        mut self,
        edit: TextEdit,
    ) -> Self {
        self.edit = Some(edit);
        self
    }

    pub fn with_command(
        mut self,
        command: Command,
    ) -> Self {
        self.command = Some(command);
        self
    }

    pub fn set_preferred(
        mut self,
        preferred: bool,
    ) -> Self {
        self.is_preferred = preferred;
        self
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn kind(&self) -> CodeActionKind {
        self.kind
    }

    pub fn edit(&self) -> Option<&TextEdit> {
        self.edit.as_ref()
    }

    pub fn command(&self) -> Option<&Command> {
        self.command.as_ref()
    }

    pub fn is_preferred(&self) -> bool {
        self.is_preferred
    }
}

/// 代码操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeActionKind {
    /// 快速修复
    QuickFix,
    /// 重构
    Refactor,
    /// 重构提取
    RefactorExtract,
    /// 重命名
    RefactorRename,
    /// 组织导入
    OrganizeImports,
    /// 其他
    Other,
}

/// 文本编辑
#[derive(Debug, Clone)]
pub struct TextEdit {
    span: Span,
    new_text: String,
}

impl TextEdit {
    pub fn new(
        span: Span,
        new_text: impl Into<String>,
    ) -> Self {
        Self {
            span,
            new_text: new_text.into(),
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn new_text(&self) -> &str {
        &self.new_text
    }
}

/// 命令
#[derive(Debug, Clone)]
pub struct Command {
    command: String,
    arguments: Vec<serde_json::Value>,
}

impl Command {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            arguments: Vec::new(),
        }
    }

    pub fn with_argument<T: serde::Serialize>(
        mut self,
        arg: T,
    ) -> Self {
        if let Ok(val) = serde_json::to_value(arg) {
            self.arguments.push(val);
        }
        self
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn arguments(&self) -> &[serde_json::Value] {
        &self.arguments
    }
}

/// 诊断事件（发布到 LSP）
#[derive(Debug, Clone)]
pub struct Diagnostic {
    span: Span,
    message: String,
    severity: Severity,
    code: Option<DiagnosticCode>,
    source: String,
    related_information: Vec<RelatedDiagnostic>,
    tags: Vec<DiagnosticTag>,
    actions: Vec<CodeAction>,
    metadata: EventMetadata,
}

impl Diagnostic {
    pub fn new(
        span: Span,
        message: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            span,
            message: message.into(),
            severity,
            code: None,
            source: "yaoxiang".to_string(),
            related_information: Vec::new(),
            tags: Vec::new(),
            actions: Vec::new(),
            metadata: EventMetadata::default(),
        }
    }

    pub fn with_code(
        mut self,
        code: DiagnosticCode,
    ) -> Self {
        self.code = Some(code);
        self
    }

    pub fn with_related(
        mut self,
        related: RelatedDiagnostic,
    ) -> Self {
        self.related_information.push(related);
        self
    }

    pub fn with_tag(
        mut self,
        tag: DiagnosticTag,
    ) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn with_action(
        mut self,
        action: CodeAction,
    ) -> Self {
        self.actions.push(action);
        self
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn code(&self) -> Option<&DiagnosticCode> {
        self.code.as_ref()
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn related_information(&self) -> &[RelatedDiagnostic] {
        &self.related_information
    }

    pub fn tags(&self) -> &[DiagnosticTag] {
        &self.tags
    }

    pub fn actions(&self) -> &[CodeAction] {
        &self.actions
    }
}

impl Event for Diagnostic {
    fn event_type(&self) -> EventType {
        EventType::Diagnostic
    }

    fn name(&self) -> &'static str {
        "Diagnostic"
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
        Some(self.span)
    }
}

/// 诊断清除事件
#[derive(Debug, Clone)]
pub struct DiagnosticsClear {
    uri: Option<String>,
    metadata: EventMetadata,
}

impl Default for DiagnosticsClear {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticsClear {
    pub fn new() -> Self {
        Self {
            uri: None,
            metadata: EventMetadata::default(),
        }
    }

    pub fn for_uri(
        mut self,
        uri: impl Into<String>,
    ) -> Self {
        self.uri = Some(uri.into());
        self
    }

    pub fn uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }
}

impl Event for DiagnosticsClear {
    fn event_type(&self) -> EventType {
        EventType::Diagnostic
    }

    fn name(&self) -> &'static str {
        "DiagnosticsClear"
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
