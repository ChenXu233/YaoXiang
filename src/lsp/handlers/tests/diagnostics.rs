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

use lsp_types::{DiagnosticSeverity, Range};

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

// --- 项目上下文诊断（与 check/run 同路径） ---

/// 在临时目录创建含 `yaoxiang.toml` 的项目。
fn create_lsp_project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"p\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
    dir
}

fn file_uri(path: &std::path::Path) -> String {
    format!("file:///{}", path.display().to_string().replace('\\', "/"))
}

#[test]
fn test_uri_to_path_windows_file_uri() {
    // Arrange + Act
    let path = crate::lsp::handlers::diagnostics::uri_to_path("file:///E:/a/b.yx");

    // Assert
    assert_eq!(path, Some(std::path::PathBuf::from("E:/a/b.yx")));
}

#[test]
fn test_uri_to_path_percent_decoded() {
    // Arrange + Act - 空格编码为 %20
    let path = crate::lsp::handlers::diagnostics::uri_to_path("file:///E:/my%20dir/b.yx");

    // Assert
    assert_eq!(path, Some(std::path::PathBuf::from("E:/my dir/b.yx")));
}

#[test]
fn test_uri_to_path_non_file_scheme_is_none() {
    let path = crate::lsp::handlers::diagnostics::uri_to_path("https://example.com/a.yx");
    assert_eq!(path, None);
}

#[test]
fn test_run_diagnostics_project_cross_file_import() {
    // Arrange - 项目内多文件：lib.yx 提供 add_one，main.yx 导入它
    let dir = create_lsp_project(&[
        ("lib.yx", "add_one: (x: Int) -> Int = (x) => x + 1\n"),
        (
            "main.yx",
            "use std.assert\nuse lib.{add_one}\n\nmain = {\n    assert.assert(add_one(41) == 42, \"ok\")\n}\n",
        ),
    ]);
    let main = dir.path().join("main.yx");
    let content = std::fs::read_to_string(&main).unwrap();

    // Act
    let result = run_diagnostics(&file_uri(&main), &content);

    // Assert - 此前单文件路径报 E1001 unknown variable 假错
    assert!(
        result.diagnostics.is_empty(),
        "跨文件导入不应有诊断，得到: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_run_diagnostics_project_missing_module_reports_e5001() {
    // Arrange - 项目内导入不存在的模块
    let dir = create_lsp_project(&[("main.yx", "use nosuch.{thing}\n\nmain = {\n}\n")]);
    let main = dir.path().join("main.yx");
    let content = std::fs::read_to_string(&main).unwrap();

    // Act
    let result = run_diagnostics(&file_uri(&main), &content);

    // Assert
    let codes: Vec<String> = result
        .diagnostics
        .iter()
        .filter_map(|d| match &d.code {
            Some(lsp_types::NumberOrString::String(code)) => Some(code.clone()),
            _ => None,
        })
        .collect();
    assert!(
        codes.contains(&"E5001".to_string()),
        "缺失模块应报 E5001，得到: {codes:?}"
    );
}
