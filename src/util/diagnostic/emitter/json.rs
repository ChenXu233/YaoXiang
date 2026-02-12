//! JSON 诊断渲染器
//!
//! 提供符合 Language Server Protocol (LSP) 规范的 JSON 输出

use serde::{Serialize, Deserialize};
use serde_json::to_string_pretty;
use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::Severity;
use crate::util::span::Span;

/// LSP 诊断严重级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LspDiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// LSP 诊断标签
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticTag {
    Unnecessary = 1,
    Deprecated = 2,
}

/// LSP 位置范围
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// LSP 位置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

/// LSP 相关诊断信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspRelatedDiagnosticInformation {
    pub location: LspRange,
    pub message: String,
}

/// LSP 代码操作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspCodeAction {
    pub title: String,
    pub kind: Option<String>,
    pub edit: Option<LspTextEdit>,
    pub command: Option<LspCommand>,
    pub is_preferred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspTextEdit {
    pub range: LspRange,
    pub new_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspCommand {
    pub title: String,
    pub command: String,
    pub arguments: Vec<serde_json::Value>,
}

/// LSP 诊断结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspDiagnostic {
    /// 诊断范围
    pub range: LspRange,
    /// 严重级别
    pub severity: Option<LspDiagnosticSeverity>,
    /// 错误代码
    pub code: Option<String>,
    /// 错误代码来源
    pub source: String,
    /// 诊断消息
    pub message: String,
    /// 相关诊断信息
    pub related_information: Option<Vec<LspRelatedDiagnosticInformation>>,
    /// 诊断标签
    pub tags: Option<Vec<DiagnosticTag>>,
    /// 代码操作
    pub code_actions: Option<Vec<LspCodeAction>>,
}

impl From<LspDiagnosticSeverity> for i32 {
    fn from(val: LspDiagnosticSeverity) -> Self {
        match val {
            LspDiagnosticSeverity::Error => 1,
            LspDiagnosticSeverity::Warning => 2,
            LspDiagnosticSeverity::Information => 3,
            LspDiagnosticSeverity::Hint => 4,
        }
    }
}

/// JSON 诊断渲染器
#[derive(Debug, Clone)]
pub struct JsonEmitter;

impl JsonEmitter {
    /// 渲染诊断为 JSON 字符串
    pub fn render(diagnostic: &Diagnostic) -> String {
        let lsp_diagnostic = Self::to_lsp_diagnostic(diagnostic);
        to_string_pretty(&lsp_diagnostic).unwrap_or_else(|_| "{}".to_string())
    }

    /// 渲染多个诊断
    pub fn render_all(diagnostics: &[Diagnostic]) -> String {
        let lsp_diagnostics: Vec<LspDiagnostic> = diagnostics
            .iter()
            .map(|d| Self::to_lsp_diagnostic(d))
            .collect();
        to_string_pretty(&lsp_diagnostics).unwrap_or_else(|_| "[]".to_string())
    }

    /// 转换为 LSP 诊断结构
    fn to_lsp_diagnostic(diagnostic: &Diagnostic) -> LspDiagnostic {
        LspDiagnostic {
            range: Self::span_to_range(diagnostic.span.as_ref()),
            severity: Some(match diagnostic.severity {
                Severity::Error => LspDiagnosticSeverity::Error,
                Severity::Warning => LspDiagnosticSeverity::Warning,
                Severity::Info => LspDiagnosticSeverity::Information,
                Severity::Hint => LspDiagnosticSeverity::Hint,
            }),
            code: Some(diagnostic.code.clone()),
            source: "yaoxiang".to_string(),
            message: diagnostic.message.clone(),
            related_information: None,
            tags: None,
            code_actions: None,
        }
    }

    /// 转换 Span 到 LSP Range
    fn span_to_range(span: Option<&Span>) -> LspRange {
        if let Some(s) = span {
            if s.is_dummy() {
                return Self::dummy_range();
            }
            LspRange {
                start: LspPosition {
                    line: (s.start.line.saturating_sub(1)) as u32,
                    character: (s.start.column.saturating_sub(1)) as u32,
                },
                end: LspPosition {
                    line: (s.end.line.saturating_sub(1)) as u32,
                    character: (s.end.column.saturating_sub(1)) as u32,
                },
            }
        } else {
            Self::dummy_range()
        }
    }

    /// 创建虚拟 Range（用于无位置的错误）
    fn dummy_range() -> LspRange {
        LspRange {
            start: LspPosition { line: 0, character: 0 },
            end: LspPosition { line: 0, character: 0 },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::diagnostic::codes::{ErrorCodeDefinition, I18nRegistry};
    use crate::util::span::Position;

    #[test]
    fn test_render_single_diagnostic() {
        let span = Span::new(
            Position::new(1, 5),
            Position::new(1, 8),
        );

        let i18n = I18nRegistry::en();
        let diagnostic = ErrorCodeDefinition::invalid_character("@")
            .at(span)
            .build(i18n);

        let json = JsonEmitter::render(&diagnostic);

        // 验证 JSON 是有效的
        let parsed: LspDiagnostic = serde_json::from_str(&json).expect("Valid JSON");
        assert_eq!(parsed.code, Some("E0001".to_string()));
        assert!(parsed.message.contains("Invalid character"));
    }

    #[test]
    fn test_render_multiple_diagnostics() {
        let i18n = I18nRegistry::en();
        let diagnostics: Vec<Diagnostic> = vec![
            ErrorCodeDefinition::invalid_character("@").build(i18n),
            ErrorCodeDefinition::invalid_number_literal("1_2_").build(i18n),
        ];

        let json = JsonEmitter::render_all(&diagnostics);

        // 验证 JSON 是有效的
        let parsed: Vec<LspDiagnostic> = serde_json::from_str(&json).expect("Valid JSON");
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_severity_mapping() {
        let i18n = I18nRegistry::en();
        let error = ErrorCodeDefinition::invalid_character("@")
            .build(i18n);

        let error_json = JsonEmitter::render(&error);
        let error_parsed: LspDiagnostic = serde_json::from_str(&error_json).unwrap();

        assert_eq!(error_parsed.severity, Some(LspDiagnosticSeverity::Error));
    }
}
