//! 诊断处理
//!
//! 将 YaoXiang 编译器诊断转换为 LSP Diagnostic 并发布。
//!
//! **状态**：阶段 2 实现
//!
//! 诊断管线：
//! ```text
//! 源代码 → tokenize → parse → check_module_collect_all
//!                          ↓                ↓
//!                    ParseError[]      Diagnostic[]
//!                          ↓                ↓
//!                    parse_error_to_diagnostic   to_lsp_diagnostics
//!                          ↓                ↓
//!                          └──── 合并 ────┘
//!                                    ↓
//!                          PublishDiagnosticsParams
//! ```

use std::str::FromStr;

use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Uri,
};
use tracing::{debug, warn};

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::check_module_collect_all;
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
/// 流程：tokenize → parse → check_module_collect_all
///
/// 任何阶段的错误都会收集为 LSP 诊断返回。
/// Lex 错误会短路（无法继续解析），但 parse 错误不影响 typecheck。
///
/// 项目内文件（向上能找到 `yaoxiang.toml`）改走 orchestrator——
/// 与 `yaoxiang check` / `run` 同一条编译路径，跨文件 `use` 不再假报。
///
/// ponytail: 每次编辑都重建项目注册表（全项目文件签名收集），大项目会偏慢；
/// 注册表缓存/增量是 RFC-029a 的活，此处不预支。
pub fn run_diagnostics(
    uri: &str,
    content: &str,
) -> PublishDiagnosticsParams {
    if let Some(path) = uri_to_path(uri) {
        use crate::frontend::module::orchestrator;
        if orchestrator::find_project_root(&path).is_some() {
            if let Ok(diagnostics) = orchestrator::check_source_in_project(&path, content) {
                debug!("项目诊断完成: {} ({} 条)", uri, diagnostics.len());
                return make_publish_params(uri, to_lsp_diagnostics(&diagnostics));
            }
            // 磁盘不可读等 → 退回单文件路径
        }
    }

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

    // 2. 语法分析
    let parse_result = parse(&tokens);

    if parse_result.has_errors {
        debug!("解析错误 ({} 个): {}", parse_result.errors.len(), uri);
        let parse_diags: Vec<Diagnostic> = parse_result.errors.to_vec();
        all_diagnostics.extend(to_lsp_diagnostics(&parse_diags));
    }

    // 3. 类型检查（收集所有错误模式）
    let type_result = check_module_collect_all(&parse_result.module, &mut None);
    if !type_result.diagnostics.is_empty() {
        debug!("类型错误 ({} 个): {}", type_result.diagnostics.len(), uri);
        all_diagnostics.extend(to_lsp_diagnostics(&type_result.diagnostics));
    } else {
        debug!("类型检查通过: {}", uri);
    }

    debug!("诊断完成: {} ({} 条诊断)", uri, all_diagnostics.len());

    make_publish_params(uri, all_diagnostics)
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
