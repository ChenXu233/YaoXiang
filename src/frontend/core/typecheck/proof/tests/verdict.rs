//! 证明结果测试 — 基于 RFC-027 §4.1, Phase 4.1
//!
//! RFC-027 §4.1: ProofResult 三值代数 (Proved / Disproved / Unproven)
//! Phase 4.1: DisproofKind + into_diagnostic() 诊断系统集成
//!
//! E4018: 精化谓词违反 → PredicateViolation → DisproofModel::into_diagnostic()
//! E4019: 类型等式不成立 → TypeMismatch → DisproofModel::into_diagnostic()

use crate::frontend::core::typecheck::proof::verdict::{DisproofKind, DisproofModel, ProofResult};
use crate::util::diagnostic::Severity;
use crate::util::span::{Position, Span};

// ============================================================
// ProofResult 基本行为
// ============================================================

#[test]
fn test_proved_is_proved_returns_true() {
    // Arrange
    let result = ProofResult::Proved;

    // Act & Assert
    assert!(result.is_proved(), "Proved::is_proved() must return true");
}

// ============================================================
// DisproofModel::into_diagnostic() — PredicateViolation
// ============================================================

#[test]
fn test_into_diagnostic_predicate_violation_basic() {
    // Arrange
    let model = DisproofModel {
        kind: DisproofKind::PredicateViolation,
        assignments: vec![("x".into(), "0".into())],
        constraint: "x > 0".into(),
        span: None,
        predicate_span: None,
    };

    // Act
    let diag = model.into_diagnostic();

    // Assert
    assert_eq!(
        diag.severity,
        Severity::Error,
        "PredicateViolation diagnostic must be Error severity"
    );
    assert_eq!(
        diag.code, "E4018",
        "PredicateViolation must use E4018 error code"
    );
    assert!(
        diag.message.contains("x > 0"),
        "Diagnostic message must contain the constraint text 'x > 0'. Got: '{}'",
        diag.message
    );
    assert!(
        diag.message.contains("x = 0"),
        "Diagnostic message must contain the counterexample 'x = 0'. Got: '{}'",
        diag.message
    );
}

#[test]
fn test_into_diagnostic_predicate_violation_multiple_assignments() {
    // Arrange
    let model = DisproofModel {
        kind: DisproofKind::PredicateViolation,
        assignments: vec![("x".into(), "0".into()), ("y".into(), "5".into())],
        constraint: "(x > 0) and (y < 0)".into(),
        span: None,
        predicate_span: None,
    };

    // Act
    let diag = model.into_diagnostic();

    // Assert
    assert!(
        diag.message.contains("x = 0"),
        "Message must contain 'x = 0'. Got: '{}'",
        diag.message
    );
    assert!(
        diag.message.contains("y = 5"),
        "Message must contain 'y = 5'. Got: '{}'",
        diag.message
    );
    assert!(
        diag.message.contains("(x > 0) and (y < 0)"),
        "Message must contain the constraint text. Got: '{}'",
        diag.message
    );
}

#[test]
fn test_into_diagnostic_predicate_violation_empty_assignments() {
    // Arrange
    let model = DisproofModel {
        kind: DisproofKind::PredicateViolation,
        assignments: vec![],
        constraint: "false".into(),
        span: None,
        predicate_span: None,
    };

    // Act
    let diag = model.into_diagnostic();

    // Assert
    assert_eq!(
        diag.code, "E4018",
        "Empty assignments must still produce E4018 error"
    );
    assert!(
        diag.message.contains("no variable assignments"),
        "Empty assignments must indicate '(no variable assignments)'. Got: '{}'",
        diag.message
    );
}

#[test]
fn test_into_diagnostic_predicate_violation_with_span() {
    // Arrange
    let start = Position::new(3, 10);
    let end = Position::new(3, 15);
    let span = Span::new(start, end);
    let model = DisproofModel {
        kind: DisproofKind::PredicateViolation,
        assignments: vec![("x".into(), "-1".into())],
        constraint: "x > 0".into(),
        span: Some(span),
        predicate_span: None,
    };

    // Act
    let diag = model.into_diagnostic();

    // Assert
    assert!(
        diag.span.is_some(),
        "Diagnostic must have a span when DisproofModel.span is set"
    );
    let diag_span = diag.span.unwrap();
    assert_eq!(
        diag_span.start.line, 3,
        "Span start line must match — expected 3, got {}",
        diag_span.start.line
    );
    assert_eq!(
        diag_span.start.column, 10,
        "Span start column must match — expected 10, got {}",
        diag_span.start.column
    );
}

// ============================================================
// DisproofModel::into_diagnostic() — TypeMismatch
// ============================================================

#[test]
fn test_into_diagnostic_type_mismatch_basic() {
    // Arrange
    let model = DisproofModel {
        kind: DisproofKind::TypeMismatch,
        assignments: vec![
            ("expected".into(), "Int".into()),
            ("found".into(), "Float".into()),
        ],
        constraint: "Int == Float".into(),
        span: None,
        predicate_span: None,
    };

    // Act
    let diag = model.into_diagnostic();

    // Assert
    assert_eq!(
        diag.code, "E4019",
        "TypeMismatch must use E4019 error code. Got: '{}'",
        diag.code
    );
    assert_eq!(
        diag.severity,
        Severity::Error,
        "TypeMismatch diagnostic must be Error severity"
    );
    assert!(
        diag.message.contains("Int"),
        "Message must contain expected type 'Int'. Got: '{}'",
        diag.message
    );
    assert!(
        diag.message.contains("Float"),
        "Message must contain found type 'Float'. Got: '{}'",
        diag.message
    );
}

#[test]
fn test_into_diagnostic_type_mismatch_single_assignment() {
    // Arrange
    let model = DisproofModel {
        kind: DisproofKind::TypeMismatch,
        assignments: vec![("expected".into(), "Bool".into())],
        constraint: "Bool".into(),
        span: None,
        predicate_span: None,
    };

    // Act
    let diag = model.into_diagnostic();

    // Assert
    assert_eq!(
        diag.code, "E4019",
        "Single assignment must still produce E4019 error"
    );
}

#[test]
fn test_into_diagnostic_type_mismatch_empty_assignments() {
    // Arrange
    let model = DisproofModel {
        kind: DisproofKind::TypeMismatch,
        assignments: vec![],
        constraint: String::new(),
        span: None,
        predicate_span: None,
    };

    // Act
    let diag = model.into_diagnostic();

    // Assert
    assert_eq!(
        diag.code, "E4019",
        "Empty assignments must still produce E4019 error"
    );
    assert!(
        diag.message.contains("expected") || !diag.message.is_empty(),
        "Diagnostic message must not panic with empty assignments. Got: '{}'",
        diag.message
    );
}

// ============================================================
// ProofResult::into_result()
// ============================================================

#[test]
fn test_into_result_proved_returns_ok() {
    // Arrange
    let result = ProofResult::Proved;

    // Act
    let outcome = result.into_result();

    // Assert
    assert!(outcome.is_ok(), "Proved must convert to Ok(()), got Err");
}

#[test]
fn test_into_result_disproved_returns_diagnostic_error() {
    // Arrange
    let model = DisproofModel {
        kind: DisproofKind::PredicateViolation,
        assignments: vec![("x".into(), "0".into())],
        constraint: "x > 0".into(),
        span: None,
        predicate_span: None,
    };
    let result = ProofResult::Disproved(model);

    // Act
    let outcome = result.into_result();

    // Assert
    assert!(
        outcome.is_err(),
        "Disproved must convert to Err(Diagnostic), got Ok"
    );
    let err = outcome.unwrap_err();
    assert_eq!(
        err.code, "E4018",
        "Disproved error must be E4018. Got: '{}'",
        err.code
    );
}
