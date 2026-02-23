//! 诊断处理
//!
//! 将 YaoXiang 编译器诊断转换为 LSP Diagnostic 并发布。
//!
//! **状态**：阶段 2 实现

use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, Range};

use crate::util::diagnostic::{Diagnostic, Severity};
use crate::util::span::Span;

/// 将 YaoXiang Diagnostic 转换为 LSP Diagnostic
pub fn to_lsp_diagnostic(diag: &Diagnostic) -> LspDiagnostic {
    let severity = match diag.severity {
        Severity::Error => Some(DiagnosticSeverity::ERROR),
        Severity::Warning => Some(DiagnosticSeverity::WARNING),
        Severity::Info => Some(DiagnosticSeverity::INFORMATION),
        Severity::Hint => Some(DiagnosticSeverity::HINT),
    };

    let range = match &diag.span {
        Some(span) => span_to_range(span),
        None => Range::default(),
    };

    let code = if diag.code.is_empty() {
        None
    } else {
        Some(lsp_types::NumberOrString::String(diag.code.clone()))
    };

    LspDiagnostic {
        range,
        severity,
        code,
        source: Some("yaoxiang".to_string()),
        message: diag.message.clone(),
        related_information: None,
        tags: None,
        code_description: None,
        data: None,
    }
}

/// 将 YaoXiang Span 转换为 LSP Range
///
/// LSP 使用 0-indexed 行号和列号。
fn span_to_range(span: &Span) -> Range {
    Range {
        start: Position {
            line: span.start.line.saturating_sub(1) as u32,
            character: span.start.column.saturating_sub(1) as u32,
        },
        end: Position {
            line: span.end.line.saturating_sub(1) as u32,
            character: span.end.column.saturating_sub(1) as u32,
        },
    }
}

/// 批量转换诊断
pub fn to_lsp_diagnostics(diagnostics: &[Diagnostic]) -> Vec<LspDiagnostic> {
    diagnostics.iter().map(to_lsp_diagnostic).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::span::Position as YxPosition;

    fn make_diag(
        severity: Severity,
        code: &str,
        message: &str,
        span: Option<Span>,
    ) -> Diagnostic {
        Diagnostic {
            severity,
            code: code.to_string(),
            message: message.to_string(),
            help: String::new(),
            span,
            related: vec![],
        }
    }

    #[test]
    fn test_severity_conversion() {
        let diag = make_diag(Severity::Error, "E0001", "type error", None);

        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(lsp_diag.source, Some("yaoxiang".to_string()));
        assert_eq!(lsp_diag.message, "type error");
    }

    #[test]
    fn test_span_to_range_zero_indexed() {
        let span = Span {
            start: YxPosition {
                line: 1,
                column: 1,
                offset: 0,
            },
            end: YxPosition {
                line: 1,
                column: 10,
                offset: 9,
            },
        };

        let range = span_to_range(&span);
        // LSP is 0-indexed
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 9);
    }

    #[test]
    fn test_batch_conversion() {
        let diagnostics = vec![
            make_diag(Severity::Error, "", "err1", None),
            make_diag(Severity::Warning, "", "warn1", None),
        ];

        let lsp_diags = to_lsp_diagnostics(&diagnostics);
        assert_eq!(lsp_diags.len(), 2);
        assert_eq!(lsp_diags[0].severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(lsp_diags[1].severity, Some(DiagnosticSeverity::WARNING));
    }

    #[test]
    fn test_empty_code_is_none() {
        let diag = make_diag(Severity::Error, "", "msg", None);
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert!(lsp_diag.code.is_none());
    }

    #[test]
    fn test_no_span_uses_default_range() {
        let diag = make_diag(Severity::Error, "E0001", "msg", None);
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.range, Range::default());
    }
}
