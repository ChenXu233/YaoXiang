//! `util::diagnostic::emitter::json` 模块的单元测试
//!
//! 覆盖 `JsonEmitter` 的单个/多个诊断渲染、严重级别映射等功能。

use crate::util::diagnostic::emitter::json::{JsonEmitter, LspDiagnostic, LspDiagnosticSeverity};
use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::codes::ErrorCodeDefinition;
use crate::util::span::{Span, Position};

#[test]
fn test_render_single_diagnostic() {
    let span = Span::new(Position::new(1, 5), Position::new(1, 8));

    let diagnostic = ErrorCodeDefinition::invalid_character("@").at(span).build();

    let json = JsonEmitter::render(&diagnostic);

    // 验证 JSON 是有效的
    let parsed: LspDiagnostic = serde_json::from_str(&json).expect("Valid JSON");
    assert_eq!(parsed.code, Some("E0001".to_string()));
    assert!(parsed.message.contains("Invalid character"));
}

#[test]
fn test_render_multiple_diagnostics() {
    let diagnostics: Vec<Diagnostic> = vec![
        ErrorCodeDefinition::invalid_character("@").build(),
        ErrorCodeDefinition::invalid_number_literal("1_2_").build(),
    ];

    let json = JsonEmitter::render_all(&diagnostics);

    // 验证 JSON 是有效的
    let parsed: Vec<LspDiagnostic> = serde_json::from_str(&json).expect("Valid JSON");
    assert_eq!(parsed.len(), 2);
}

#[test]
fn test_severity_mapping() {
    let error = ErrorCodeDefinition::invalid_character("@").build();

    let error_json = JsonEmitter::render(&error);
    let error_parsed: LspDiagnostic = serde_json::from_str(&error_json).unwrap();

    assert_eq!(error_parsed.severity, Some(LspDiagnosticSeverity::Error));
}
