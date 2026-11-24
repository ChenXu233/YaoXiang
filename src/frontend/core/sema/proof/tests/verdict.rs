//! 证明结果测试 — 基于 RFC-027 §4.1, Phase 4.1
//!
//! RFC-027 §4.1: ProofResult 三值代数 (Proved / Disproved / Unproven)
//! Phase 4.1: DisproofKind + into_diagnostic() 诊断系统集成
//!
//! E4018: 精化谓词违反 → PredicateViolation → DisproofModel::into_diagnostic()
//! E4019: 类型等式不成立 → TypeMismatch → DisproofModel::into_diagnostic()

use crate::frontend::core::sema::proof::verdict::{
    BudgetReport, DisproofKind, DisproofModel, ProofFailure, ProofResultExt, UnprovenReason,
    UnprovenSubject,
};
use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::util::diagnostic::Severity;
use crate::util::span::{Position, Span};

/// TypeMismatch 反例模型夹具：`Int == Float`。
fn type_mismatch_model() -> DisproofModel {
    DisproofModel {
        kind: DisproofKind::TypeMismatch,
        assignments: vec![
            ("expected".into(), "Int".into()),
            ("found".into(), "Float".into()),
        ],
        constraint: "Int == Float".into(),
        span: None,
        predicate_span: None,
    }
}

// ProofResult 基本行为

#[test]
fn test_proved_is_proved_returns_true() {
    // Arrange
    let result = Ok(());

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
    // issue #469 处 2a：模型未绑定值时反例段整体缺席（此前输出悬空的
    // "Counterexample:" 标题 + "(no variable assignments)" 占位）。
    // 断言双语段标题都不出现——段标题现在只随有值反例一起由 locales 渲染。
    assert!(
        !diag.message.contains("Counterexample:") && !diag.message.contains("反例："),
        "模型未绑定值时不得输出反例段（issue #469 处 2a）. Got: '{}'",
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
    let model = type_mismatch_model();

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

// ProofResult::into_result()

#[test]
fn test_into_result_proved_returns_ok() {
    // Arrange
    let result = Ok(());

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
    let result = Err(ProofFailure::Disproved(model));

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
    let result = Err(ProofFailure::Unproven {
        reason: UnprovenReason::LoopTerminationUnproven {
            span: crate::util::span::Span::default(),
        },
        proof_calls: vec![],
        budget: BudgetReport {
            steps_used: 0,
            steps_limit: 0,
        },
        // 终止性原因自带 span（E4021 用它），失败值侧的载荷留空
        subject: None,
        span: None,
    });

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

/// t64：一般性 `Unproven` 走**专属 E4023**，消息含命题与原因，不再复用 E2031。
#[test]
fn test_into_result_general_unproven_uses_e4023() {
    // Arrange — BeyondKernel 属一般性「无法证明」；带命题 `false` 与位置
    let result = Err(ProofFailure::Unproven {
        reason: UnprovenReason::BeyondKernel("未分类情形".into()),
        proof_calls: vec![],
        budget: BudgetReport {
            steps_used: 0,
            steps_limit: 0,
        },
        subject: Some(UnprovenSubject::Term(ConstExpr::Lit(ConstValue::Bool(
            false,
        )))),
        span: None,
    });

    // Act
    let outcome = result.into_result();

    // Assert — t64：一般性 Unproven 落**专属 E4023**（不再复用 E2031 的空串形态）
    let err = outcome.expect_err("Unproven 必须转成 Err(Diagnostic)");
    assert_eq!(
        err.code, "E4023",
        "一般性 Unproven 应有专属码 E4023，不得复用 E2031。实际: '{}'",
        err.code
    );
}

/// 测度判伪必须走 E4022，**不得**降级为 E8001 ICE。
///
/// 对照 `test_into_result_loop_termination_unproven_is_user_domain_code`：
/// 那条走 `UnprovenReason`（未证明），这条走 `DisproofKind`（判伪）。两者是
/// 不同的证明结果分支，各自不得落到 ICE 兜底。
#[test]
fn test_into_result_measure_not_decreasing_is_user_domain_code() {
    use crate::frontend::core::sema::proof::verdict::{DisproofKind, DisproofModel};

    // Arrange —— 递减义务被判伪，反例 b = 0
    let result = Err(ProofFailure::Disproved(DisproofModel {
        kind: DisproofKind::MeasureNotDecreasing,
        assignments: vec![("b".to_string(), "0".to_string())],
        constraint: "b".to_string(),
        span: Some(crate::util::span::Span::default()),
        predicate_span: None,
    }));

    // Act
    let outcome = result.into_result();

    // Assert —— 必须是用户域码 E4022，而非 ICE E8001
    let err = outcome.expect_err("Disproved 必须转成 Err(Diagnostic)");
    assert_eq!(
        err.code, "E4022",
        "测度判伪须映射到 E4022（用户域），不得为 E8001 ICE。实际: '{}'",
        err.code
    );
}

/// t64 验收：三条「一般性」原因都走 **E4023**，消息含非空命题、原因文本可读，
/// 且**不出现空占位**（`''` / `（）` 里空串）——命题缺失时用中性主语「该义务」。
#[test]
fn test_unproven_general_variants_render_e4023_with_proposition() {
    // Arrange — (原因, 消息里必须出现的原因文本)
    let cases = vec![
        (UnprovenReason::Symbolic("k".into()), "k"),
        (UnprovenReason::BudgetExceeded, "预算耗尽"),
        (
            UnprovenReason::BeyondKernel("超出内核能力".into()),
            "超出内核能力",
        ),
    ];

    for (reason, reason_text) in cases {
        // Act
        let result = Err(ProofFailure::Unproven {
            reason,
            proof_calls: vec![],
            budget: BudgetReport {
                steps_used: 3,
                steps_limit: 10,
            },
            subject: Some(UnprovenSubject::Term(ConstExpr::Lit(ConstValue::Bool(
                false,
            )))),
            span: None,
        });
        let outcome = result.into_result();

        // Assert
        let err = outcome.expect_err("unproven 必须转成 Err(Diagnostic)");
        assert_eq!(
            err.code, "E4023",
            "一般性原因必须落 E4023，实际: {}",
            err.code
        );
        assert!(
            err.message.contains("false"),
            "消息必须带出非空命题（`false`），实际: '{}'",
            err.message
        );
        assert!(
            err.message.contains(reason_text),
            "消息必须带出可读原因（{reason_text}），实际: '{}'",
            err.message
        );
        assert!(
            !err.message.contains("''") && !err.message.contains("（）"),
            "消息不得出现空占位，实际: '{}'",
            err.message
        );
    }
}

/// t64 验收：命题缺失时用中性主语「该义务」，**不伪造**命题、不留空占位。
#[test]
fn test_unproven_general_without_proposition_uses_neutral_subject() {
    // Arrange
    let result = Err(ProofFailure::Unproven {
        reason: UnprovenReason::BeyondKernel("超出内核能力".into()),
        proof_calls: vec![],
        budget: BudgetReport {
            steps_used: 0,
            steps_limit: 10,
        },
        subject: None,
        span: None,
    });

    // Act
    let outcome = result.into_result();

    // Assert
    let err = outcome.expect_err("unproven 必须转成 Err(Diagnostic)");
    assert_eq!(err.code, "E4023", "实际: {}", err.code);
    assert!(
        err.message.contains("该义务"),
        "无命题时应使用中性主语，实际: '{}'",
        err.message
    );
    assert!(
        !err.message.contains("''"),
        "不得出现空占位，实际: '{}'",
        err.message
    );
}

/// t66 验收：**类型等式**命题（`A ≡ B`）可渲染——码 E4023、消息含 `≡` 与两侧
/// 类型文本，且**不含**中性主语「该义务」。
///
/// t66 之前 `proposition: Option<ConstExpr>` 装不下等式命题，`equivalence` 的
/// 判等失败点只能传 `None` ⇒ 消息退化成「无法证明：该义务（…）」，读不出在证什么。
#[test]
fn test_unproven_type_equivalence_renders_equiv_relation() {
    // Arrange — 等式义务：`int64 ≡ int64::3`
    let lhs = MonoType::Int(64);
    let rhs = MonoType::Literal {
        name: "3".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: ConstValue::Int(3),
    };
    let result = Err(ProofFailure::Unproven {
        reason: UnprovenReason::BeyondKernel("两侧归一化后不相等".into()),
        proof_calls: vec![],
        budget: BudgetReport {
            steps_used: 2,
            steps_limit: 10,
        },
        subject: Some(UnprovenSubject::TypeEquivalence(
            Box::new(lhs.clone()),
            Box::new(rhs.clone()),
        )),
        span: None,
    });

    // Act
    let outcome = result.into_result();

    // Assert
    let err = outcome.expect_err("unproven 必须转成 Err(Diagnostic)");
    assert_eq!(err.code, "E4023", "实际: {}", err.code);
    assert!(
        err.message.contains("≡"),
        "类型等式命题必须渲染出 `≡`，实际: '{}'",
        err.message
    );
    assert!(
        err.message.contains(&format!("{}", lhs)) && err.message.contains(&format!("{}", rhs)),
        "消息必须带出两侧类型文本，实际: '{}'",
        err.message
    );
    assert!(
        !err.message.contains("该义务"),
        "有结构化命题时不得退化成中性主语，实际: '{}'",
        err.message
    );
}
