//! 证明结果测试 — 基于 RFC-027 §4.1, Phase 4.1
//!
//! RFC-027 §4.1: ProofResult 三值代数 (Proved / Disproved / Unproven)
//! Phase 4.1: DisproofKind + into_diagnostic() 诊断系统集成
//!
//! E4018: 精化谓词违反 → PredicateViolation → DisproofModel::into_diagnostic()
//! E4019: 类型等式不成立 → TypeMismatch → DisproofModel::into_diagnostic()

use crate::frontend::core::typecheck::proof::verdict::{
    BudgetReport, DisproofKind, DisproofModel, ProofResult, UnprovenReason,
};
use crate::util::diagnostic::Severity;
use crate::util::span::{Position, Span};

// ProofResult 基本行为

#[test]
fn test_proved_is_proved_returns_true() {
    // Arrange
    let result = ProofResult::Proved;

    // Act & Assert
    assert!(result.is_proved(), "Proved::is_proved() must return true");
}

// DisproofModel::into_diagnostic() — PredicateViolation

#[test]
fn test_into_diagnostic_predicate_violation_basic() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
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
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
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
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
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

// DisproofModel::into_diagnostic() — TypeMismatch

#[test]
fn test_into_diagnostic_type_mismatch_basic() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
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
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
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
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
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

// ProofResult::into_result()

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
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
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

// UnprovenReason 的诊断映射（能力边界 vs ICE）

/// 循环终止性无法证明必须走 E4021，**不得**降级为 E8001 ICE
///
/// 回归性质：`LoopTerminationUnproven` 变体引入前，终止检查用
/// `BeyondKernel("循环无法证明终止…")`，`into_result()` 将其一律转成
/// `E8001`（Internal 类 ICE，文案引导用户「报告此错误」）。
/// 编译器推理能力边界被误诊为编译器故障。
#[test]
fn test_into_result_loop_termination_unproven_is_user_domain_code() {
    // Arrange
    let result = ProofResult::Unproven {
        reason: UnprovenReason::LoopTerminationUnproven,
        proof_calls: vec![],
        budget: BudgetReport {
            steps_used: 0,
            steps_limit: 0,
        },
    };

    // Act
    let outcome = result.into_result();

    // Assert — 必须是用户域码 E4021，而非 ICE E8001
    let err = outcome.expect_err("Unproven 必须转成 Err(Diagnostic)");
    assert_eq!(
        err.code, "E4021",
        "终止性无法证明须映射到 E4021（用户域），不得为 E8001 ICE。实际: '{}'",
        err.code
    );
}

/// 未分类的 `Unproven` 仍走 ICE — 保证兜底未被本改动误伤
#[test]
fn test_into_result_unclassified_unproven_still_reports_ice() {
    // Arrange — BeyondKernel 目前无专属码，属未分类情况
    let result = ProofResult::Unproven {
        reason: UnprovenReason::BeyondKernel("未分类情形".into()),
        proof_calls: vec![],
        budget: BudgetReport {
            steps_used: 0,
            steps_limit: 0,
        },
    };

    // Act
    let outcome = result.into_result();

    // Assert — 兜底保持 E8001（指引报 issue 是正确出路）
    let err = outcome.expect_err("Unproven 必须转成 Err(Diagnostic)");
    assert_eq!(
        err.code, "E8001",
        "未分类的 Unproven 应保持 E8001 兜底。实际: '{}'",
        err.code
    );
}
