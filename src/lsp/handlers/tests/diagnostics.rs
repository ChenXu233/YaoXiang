//! 诊断处理器测试
//!
//! 测试覆盖：
//! - 严重级别转换
//! - Span 到 Range 转换
//! - 批量转换
//! - 空代码处理
//! - 无 Span 处理
//! - 完整诊断管线
//! - 清除诊断

use std::str::FromStr;

use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Uri,
};

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_recovery;
use crate::frontend::core::typecheck::check_module_collect_all;
use crate::lsp::handlers::diagnostics::{
    to_lsp_diagnostic, to_lsp_diagnostics, run_diagnostics, clear_diagnostics,
};
use crate::util::diagnostic::{Diagnostic, Severity};
use crate::util::span::Span;
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

    let range = crate::lsp::handlers::diagnostics::span_to_range(&span);
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
    let params = crate::lsp::handlers::diagnostics::make_publish_params("file:///hello.yx", vec![]);
    assert_eq!(params.uri.as_str(), "file:///hello.yx");
    assert!(params.diagnostics.is_empty());
    assert!(params.version.is_none());
}
