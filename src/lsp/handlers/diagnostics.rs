//! 诊断处理
//!
//! 将 YaoXiang 编译器诊断转换为 LSP Diagnostic 并发布。
//!
//! **状态**：阶段 2 实现
//!
//! 诊断管线（4.2.6 起）：
//! ```text
//! 项目内文件 → orchestrator::check_source_in_project（Lsp 形态）
//! 单文件兜底 → Program{SingleFile, CollectAll} + Driver
//!                          ↓
//!                    Diagnostic[]（errors + warnings 合并）
//!                          ↓
//!                    to_lsp_diagnostics
//!                          ↓
//!                    PublishDiagnosticsParams
//! ```
//!
//! 手工 lex→parse→typecheck 序列已删除（02 §3 入口表）：CLI 与 LSP
//! 同一 Driver，proof（E4018）与警告（W 码）随统一管线进入 LSP。

use std::str::FromStr;

use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Uri,
};
use tracing::{debug, warn};

use crate::driver::{Aggregation, Driver, Program, ProgramKind, Unit};
use crate::frontend::config::CompileConfig;
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

/// 将 ParseError 转换为 YaoXiang Diagnostic
///
/// 本地 file URI → 磁盘路径（`file:///E:/a/b.yx` → `E:\a\b.yx`）。
///
/// ponytail: 仅处理 file scheme + %XX 解码；其余返回 None（退回单文件路径）。
pub(crate) fn uri_to_path(uri: &str) -> Option<std::path::PathBuf> {
    let parsed = Uri::from_str(uri).ok()?;
    if parsed.scheme().map(|s| s.as_str()) != Some("file") {
        return None;
    }
    let decoded = percent_decode(parsed.path().as_str());
    let trimmed = if cfg!(windows) {
        decoded.trim_start_matches('/').to_string()
    } else {
        decoded
    };
    Some(std::path::PathBuf::from(trimmed))
}

/// %XX 解码（UTF-8 安全）。
fn percent_decode(s: &str) -> String {
    fn hex(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push(high * 16 + low);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 对文档内容运行完整诊断管线
///
/// 项目内文件（向上能找到 `yaoxiang.toml`）走 orchestrator——
/// 与 `yaoxiang check` / `run` 同一条编译路径，跨文件 `use` 不再假报。
///
/// 单文件/非项目兜底（4.2.6，02 §3 入口表）：走
/// `Program { kind: SingleFile, aggregation: CollectAll }` + Driver，
/// 与 CLI 同一编译管线。CollectAll 语义下 parse 收全量错误、残缺 AST
/// 继续 typecheck（编辑器哲学）；proof（E4018）与警告（W 码）随统一
/// 管线进入 LSP（原手工 lex→parse→typecheck 序列已删除）。
///
/// ponytail: 每次编辑都重建项目注册表（全项目文件签名收集），大项目会偏慢；
/// 注册表缓存/增量是 RFC-029a 的活，此处不预支。
pub fn run_diagnostics(
    uri: &str,
    content: &str,
) -> PublishDiagnosticsParams {
    let path = uri_to_path(uri);
    if let Some(p) = &path {
        use crate::frontend::module::orchestrator;
        if orchestrator::find_project_root(p).is_some() {
            if let Ok(diagnostics) = orchestrator::check_source_in_project(p, content) {
                debug!("项目诊断完成: {} ({} 条)", uri, diagnostics.len());
                return make_publish_params(uri, to_lsp_diagnostics(&diagnostics));
            }
            // 磁盘不可读等 → 退回单文件路径
        }
    }

    // 单文件兜底：与 CLI 同一 Driver（SingleFile + CollectAll）
    let unit_path = path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| uri.to_string());
    let program = Program::new(
        ProgramKind::SingleFile,
        vec![Unit::new("<lsp>", unit_path, content)],
        CompileConfig::default(),
    )
    .with_aggregation(Aggregation::CollectAll);
    let outcome = match Driver::new().run(program) {
        Ok(outcome) => outcome,
        Err(e) => {
            // 结构性不可达（SingleFile 恒 1 单元、各臂全接线）——防御：退回空诊断
            warn!("driver 运行失败: {} - {}", uri, e);
            return make_publish_params(uri, Vec::new());
        }
    };
    let diagnostics: Vec<Diagnostic> = outcome
        .result
        .errors
        .iter()
        .filter_map(|e| e.diagnostic())
        .chain(outcome.result.warnings.iter().cloned())
        .collect();

    debug!("诊断完成: {} ({} 条诊断)", uri, diagnostics.len());

    make_publish_params(uri, to_lsp_diagnostics(&diagnostics))
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
