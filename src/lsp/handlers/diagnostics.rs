//! 诊断处理
//!
//! 将 YaoXiang 编译器诊断转换为 LSP Diagnostic 并发布。
//!
//! **状态**：阶段 2 实现
//!
//! 诊断管线：
//! ```text
//! 源代码 → tokenize → parse_with_recovery → check_module_collect_all
//!                          ↓                         ↓
//!                     ParseError[]              Diagnostic[]
//!                          ↓                         ↓
//!                    parse_error_to_diagnostic   to_lsp_diagnostics
//!                          ↓                         ↓
//!                          └─────── 合并 ────────────┘
//!                                    ↓
//!                          PublishDiagnosticsParams
//! ```

use std::str::FromStr;

use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Uri,
};
use tracing::{debug, warn};

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parser_state::ParseError;
use crate::frontend::core::parser::parse_with_recovery;
use crate::frontend::typecheck::check_module_collect_all;
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

/// 将 ParseError 转换为 YaoXiang Diagnostic
fn parse_error_to_diagnostic(err: &ParseError) -> Diagnostic {
    let (message, span) = match err {
        ParseError::ExpectedToken {
            expected,
            found,
            span,
        } => (
            format!("期望 {:?}，实际为 {:?}", expected, found),
            Some(*span),
        ),
        ParseError::UnexpectedToken { found, span } => {
            (format!("意外的 token: {:?}", found), Some(*span))
        }
        ParseError::Message(msg) => (msg.clone(), None),
    };

    Diagnostic::error("E0100".to_string(), message, String::new(), span)
}

/// 对文档内容运行完整诊断管线
///
/// 流程：tokenize → parse_with_recovery → check_module_collect_all
///
/// 任何阶段的错误都会收集为 LSP 诊断返回。
/// Lex 错误会短路（无法继续解析），但 parse 错误不影响 typecheck。
pub fn run_diagnostics(
    uri: &str,
    content: &str,
) -> PublishDiagnosticsParams {
    let mut all_diagnostics: Vec<LspDiagnostic> = Vec::new();

    // 1. 词法分析
    let tokens = match tokenize(content) {
        Ok(tokens) => tokens,
        Err(lex_err) => {
            warn!("词法分析失败: {} - {}", uri, lex_err);
            // Lex 错误 → 单条诊断，无法继续
            all_diagnostics.push(LspDiagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(lsp_types::NumberOrString::String("E0001".to_string())),
                source: Some("yaoxiang".to_string()),
                message: format!("词法错误: {}", lex_err),
                related_information: None,
                tags: None,
                code_description: None,
                data: None,
            });

            return make_publish_params(uri, all_diagnostics);
        }
    };

    // 2. 语法分析（含错误恢复）
    let parse_result = parse_with_recovery(&tokens);

    if parse_result.has_errors {
        debug!("解析错误 ({} 个): {}", parse_result.errors.len(), uri);
        let parse_diags: Vec<Diagnostic> = parse_result
            .errors
            .iter()
            .map(parse_error_to_diagnostic)
            .collect();
        all_diagnostics.extend(to_lsp_diagnostics(&parse_diags));
    }

    // 3. 类型检查（收集所有错误模式）
    match check_module_collect_all(&parse_result.module, &mut None) {
        Ok(_) => {
            debug!("类型检查通过: {}", uri);
        }
        Err(type_errors) => {
            debug!("类型错误 ({} 个): {}", type_errors.len(), uri);
            all_diagnostics.extend(to_lsp_diagnostics(&type_errors));
        }
    }

    debug!("诊断完成: {} ({} 条诊断)", uri, all_diagnostics.len());

    make_publish_params(uri, all_diagnostics)
}

/// 为关闭的文档生成空诊断（清除已有诊断）
pub fn clear_diagnostics(uri: &str) -> PublishDiagnosticsParams {
    make_publish_params(uri, Vec::new())
}

/// 构建 PublishDiagnosticsParams
fn make_publish_params(
    uri: &str,
    diagnostics: Vec<LspDiagnostic>,
) -> PublishDiagnosticsParams {
    PublishDiagnosticsParams {
        uri: Uri::from_str(uri).unwrap_or_else(|_| {
            warn!("无效的 URI: {}", uri);
            Uri::from_str("file:///invalid").unwrap()
        }),
        diagnostics,
        version: None,
    }
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

    // --- 阶段 2 新增测试 ---

    #[test]
    fn test_parse_error_to_diagnostic() {
        let err = ParseError::UnexpectedToken {
            found: crate::frontend::core::lexer::tokens::TokenKind::Plus,
            span: Span {
                start: YxPosition {
                    line: 1,
                    column: 5,
                    offset: 4,
                },
                end: YxPosition {
                    line: 1,
                    column: 6,
                    offset: 5,
                },
            },
        };

        let diag = parse_error_to_diagnostic(&err);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.code, "E0100");
        assert!(diag.message.contains("意外的 token"));
        assert!(diag.span.is_some());
    }

    #[test]
    fn test_run_diagnostics_valid_code() {
        // 合法的 YaoXiang 代码应产生零诊断
        let result = run_diagnostics("file:///test.yx", "x = 42\n");
        assert!(
            result.diagnostics.is_empty(),
            "合法代码不应有诊断，但得到: {:?}",
            result.diagnostics
        );
    }

    #[test]
    fn test_run_diagnostics_parse_error() {
        // 语法错误
        let result = run_diagnostics("file:///test.yx", "@ @ @\n");
        assert!(!result.diagnostics.is_empty(), "语法错误应产生诊断");
        // 所有诊断应来自 yaoxiang
        for d in &result.diagnostics {
            assert_eq!(d.source, Some("yaoxiang".to_string()));
        }
    }

    #[test]
    fn test_clear_diagnostics() {
        let result = clear_diagnostics("file:///test.yx");
        assert!(result.diagnostics.is_empty());
        assert_eq!(result.uri.as_str(), "file:///test.yx");
    }

    #[test]
    fn test_make_publish_params() {
        let params = make_publish_params("file:///hello.yx", vec![]);
        assert_eq!(params.uri.as_str(), "file:///hello.yx");
        assert!(params.diagnostics.is_empty());
        assert!(params.version.is_none());
    }
}
