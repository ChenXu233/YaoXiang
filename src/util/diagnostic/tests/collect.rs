//! ErrorCollector 测试 — 基于 check-improvement 设计规范
//!
//! §4.4: 错误处理修复

use crate::util::diagnostic::collect::{ErrorCollector, Warning};
use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::codes::ErrorCodeDefinition;

fn make_test_diagnostic() -> Diagnostic {
    ErrorCodeDefinition::invalid_character("@").build()
}

#[test]
fn test_collector_extend_errors_adds_multiple() {
    let mut collector = ErrorCollector::<Diagnostic>::new();
    let errors = vec![make_test_diagnostic(), make_test_diagnostic()];
    collector.extend_errors(errors);
    assert_eq!(
        collector.error_count(),
        2,
        "should have 2 errors after extend"
    );
}

#[test]
fn test_collector_clear_removes_all_errors() {
    let mut collector = ErrorCollector::<Diagnostic>::new();
    collector.add_error(make_test_diagnostic());
    assert!(collector.has_errors());
    collector.clear();
    assert_eq!(
        collector.error_count(),
        0,
        "should have 0 errors after clear"
    );
    assert!(!collector.has_errors());
}

#[test]
fn test_warning_from_diagnostic_display() {
    let diag = make_test_diagnostic();
    let warning = Warning::from_diagnostic(diag.clone());
    assert_eq!(
        warning.to_string(),
        diag.message,
        "Display should show diagnostic message"
    );
}
