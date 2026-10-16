//! 诊断处理
//!
//! 将 YaoXiang 编译器诊断转换为 LSP Diagnostic 并构造发布参数。
//!
//! **状态**：阶段 2 实现
//!
//! 诊断与语义数据的**同一次编译**在 `lsp/analysis.rs`（WBS 4.6.1）；
//! 本模块只负责协议面转换（Diagnostic → LSP Diagnostic、发布参数构造）。
//! 分析管线（4.6.1 起）：
//! ```text
//! 项目内文件 → orchestrator::analyze_source_in_project（Lsp 形态）
//! 单文件兜底 → Program{SingleFile, CollectAll} + Driver
//!                          ↓ 同一趟编译
//!            Diagnostic[] + SemanticDB（规范化 URI 键）
//! ```
//!
//! 手工 lex→parse→typecheck 序列已删除（02 §3 入口表）：CLI 与 LSP
//! 同一 Driver，proof（E4018）与警告（W 码）随统一管线进入 LSP。

use std::str::FromStr;

use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Uri,
};
use tracing::warn;

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
pub(crate) fn span_to_range(span: &Span) -> Range {
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

/// 为关闭的文档生成空诊断（清除已有诊断）
pub fn clear_diagnostics(uri: &str) -> PublishDiagnosticsParams {
    make_publish_params(uri, Vec::new())
}

/// 构建 PublishDiagnosticsParams
pub(crate) fn make_publish_params(
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
