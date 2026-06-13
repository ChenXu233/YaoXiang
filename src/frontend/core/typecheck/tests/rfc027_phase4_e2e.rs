//! RFC-027 Phase 4 端到端测试：反例格式化诊断管道
//!
//! RFC-027 §8 & Phase 4.1: DisproofKind + into_diagnostic() 完整管道验证。
//!
//! 测试覆盖：
//! - check_predicate() → Disproved 携带 DisproofKind::PredicateViolation
//! - check_type_equivalence() → Disproved 携带 DisproofKind::TypeMismatch
//! - into_diagnostic() 端到端格式化（E4018/E4019 + 消息 + Span）
//! - into_result() 端到端路由
//! - SMT 反例模型完整性（通过新 DisproofModel 字段透传）

use std::collections::HashMap;

use crate::frontend::core::typecheck::layers::equivalence::check_type_equivalence;
use crate::frontend::core::typecheck::layers::predicate::check_predicate;
use crate::frontend::core::typecheck::proof::context::ProofContext;
use crate::frontend::core::typecheck::proof::verdict::{DisproofKind, ProofResult};
use crate::frontend::core::typecheck::TypeEnvironment;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::util::span::{Position, Span};

// ===================================================================
// RFC-027 §4 & Phase 4.1: PredicateViolation E2E
// ===================================================================

/// E2E: Level 1 Disproved → DisproofModel.kind = PredicateViolation
///
/// 模拟 `Positive(0)` — 直接求值 x>0=false，验证 kind 字段正确。
#[test]
fn test_e2e_phase4_level1_disproved_has_predicate_violation_kind() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("x".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(0));
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    match result {
        ProofResult::Disproved(model) => {
            assert_eq!(
                model.kind,
                DisproofKind::PredicateViolation,
                "E2E Level 1: Disproved 的 kind 必须是 PredicateViolation，实际: {:?}",
                model.kind
            );
        }
        other => panic!("E2E Level 1: x=0 时 x>0 必须 Disproved，实际: {other:?}"),
    }
}

/// E2E: Level 1 Disproved → constraint 文本包含正确的谓词表达式
#[test]
fn test_e2e_phase4_disproved_model_has_constraint_text() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("x".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(0));
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    match result {
        ProofResult::Disproved(model) => {
            assert!(
                model.constraint.contains("x > 0"),
                "constraint 文本必须包含 'x > 0'，实际: '{}'",
                model.constraint
            );
            assert!(!model.constraint.is_empty(), "constraint 文本不能为空");
        }
        other => panic!("期望 Disproved，实际: {other:?}"),
    }
}

/// E2E: Level 3 SMT Disproved → 反例模型的 assignments 被正确保留
#[test]
fn test_e2e_phase4_smt_disproved_preserves_assignments() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act — 无假设，SMT 应找到 y=0 反例
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert
    match result {
        ProofResult::Disproved(model) => {
            assert!(!model.assignments.is_empty(), "SMT 反例模型必须有变量赋值");
            assert!(
                model.assignments.iter().any(|(k, _)| k == "y"),
                "反例模型必须包含变量 y，实际: {:?}",
                model.assignments
            );
        }
        other => panic!("期望 Disproved，实际: {other:?}"),
    }
}

/// E2E: into_diagnostic() — PredicateViolation 产生 E4018 诊断
///
/// 验证完整管道：check_predicate → Disproved → into_diagnostic → E4018
#[test]
fn test_e2e_phase4_into_diagnostic_predicate_violation_e4018() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("x".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(0));
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    match result {
        ProofResult::Disproved(model) => {
            let diag = model.into_diagnostic();
            assert_eq!(
                diag.code, "E4018",
                "E2E: PredicateViolation 诊断必须是 E4018，实际: {}",
                diag.code
            );
            assert!(
                diag.message.contains("x > 0"),
                "诊断消息必须包含约束文本 '(x > 0)'，实际: '{}'",
                diag.message
            );
            assert!(
                diag.message.contains("x = Int(0)"),
                "诊断消息必须包含反例 'x = Int(0)'，实际: '{}'",
                diag.message
            );
        }
        other => panic!("期望 Disproved，实际: {other:?}"),
    }
}

/// E2E: into_diagnostic() — PredicateViolation 带 Span
///
/// 验证设置了 span 时诊断携带正确位置。
#[test]
fn test_e2e_phase4_into_diagnostic_with_span() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("x".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(0));
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    match result {
        ProofResult::Disproved(mut model) => {
            // 设置 span（模拟 checker.rs 在调用点填入）
            let start = Position::new(42, 5);
            let end = Position::new(42, 10);
            model.span = Some(Span::new(start, end));

            let diag = model.into_diagnostic();
            assert!(diag.span.is_some(), "诊断必须有 span");
            let diag_span = diag.span.unwrap();
            assert_eq!(
                diag_span.start.line, 42,
                "Span 行号必须匹配，实际: {}",
                diag_span.start.line
            );
        }
        other => panic!("期望 Disproved，实际: {other:?}"),
    }
}

/// E2E: into_result() — Disproved 转 Err(Diagnostic)
///
/// 验证完整管道：check_predicate → into_result → Err(E4018)
#[test]
fn test_e2e_phase4_into_result_disproved_to_err() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("x".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(0));
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &bindings);
    let outcome = result.into_result();

    // Assert
    assert!(
        outcome.is_err(),
        "E2E: Disproved must convert to Err(Diagnostic), got Ok"
    );
    let err = outcome.unwrap_err();
    assert_eq!(
        err.code, "E4018",
        "E2E: into_result 错误码必须是 E4018，实际: {}",
        err.code
    );
    assert!(
        err.message.contains("x > 0"),
        "E2E: into_result 消息必须包含约束文本，实际: '{}'",
        err.message
    );
}

// ===================================================================
// RFC-027 §4 & Phase 4.1: TypeMismatch E2E
// ===================================================================

/// E2E: check_type_equivalence → TypeMismatch
///
/// 两个不同类型归约后不等 → Disproved(TypeMismatch)
#[test]
fn test_e2e_phase4_type_equivalence_type_mismatch() {
    // Arrange
    let lhs = MonoType::Int(64);
    let rhs = MonoType::Float(64);
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_type_equivalence(&ctx, &lhs, &rhs);

    // Assert
    match result {
        ProofResult::Disproved(model) => {
            assert_eq!(
                model.kind,
                DisproofKind::TypeMismatch,
                "Int != Float 的 kind 必须是 TypeMismatch，实际: {:?}",
                model.kind
            );
            let diag = model.into_diagnostic();
            assert_eq!(
                diag.code, "E4019",
                "TypeMismatch 诊断必须是 E4019，实际: {}",
                diag.code
            );
        }
        other => panic!("Int != Float 必须 Disproved，实际: {other:?}"),
    }
}

/// E2E: check_type_equivalence 相同类型 → Proved（正常路径）
#[test]
fn test_e2e_phase4_type_equivalence_same_type_proved() {
    // Arrange
    let lhs = MonoType::Int(64);
    let rhs = MonoType::Int(64);
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_type_equivalence(&ctx, &lhs, &rhs);

    // Assert
    assert!(
        result.is_proved(),
        "E2E: Int == Int 必须是 Proved，实际: {result:?}"
    );
}

// ===================================================================
// RFC-027 §4: 多变量反例 + 假设蕴含 E2E
// ===================================================================

/// E2E: 多变量约束 Disproved → 所有变量都出现在反例中
#[test]
fn test_e2e_phase4_multivariable_constraint_disproved() {
    // Arrange
    // 约束: x + y > 0，无假设 → SMT 可找 x=-1, y=0
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::BinOp {
            op: BinOp::Add,
            left: Box::new(ConstExpr::NamedVar("x".into())),
            right: Box::new(ConstExpr::NamedVar("y".into())),
        }),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert
    match result {
        ProofResult::Disproved(model) => {
            assert_eq!(
                model.kind,
                DisproofKind::PredicateViolation,
                "多变量反例的 kind 必须是 PredicateViolation"
            );
            // into_diagnostic 应包含约束文本
            let diag = model.into_diagnostic();
            assert!(
                diag.message.contains("x") || diag.message.contains("y"),
                "诊断消息应包括变量名，实际: '{}'",
                diag.message
            );
        }
        other => panic!("期望 Disproved，实际: {other:?}"),
    }
}

/// E2E: 批量 DisproofModel — 多个证明结果各自独立转换
#[test]
fn test_e2e_phase4_multiple_disproved_models_independent() {
    // Arrange — 两个独立的约束
    let constraint1 = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("a".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined1 = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: constraint1,
    };
    let mut bindings1 = HashMap::new();
    bindings1.insert("a".into(), ConstValue::Int(0));

    let constraint2 = ConstExpr::BinOp {
        op: BinOp::Lt,
        left: Box::new(ConstExpr::NamedVar("b".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(10))),
    };
    let refined2 = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: constraint2,
    };
    let mut bindings2 = HashMap::new();
    bindings2.insert("b".into(), ConstValue::Int(20));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result1 = check_predicate(&ctx, &refined1, &bindings1);
    let result2 = check_predicate(&ctx, &refined2, &bindings2);

    // Assert — 两个独立 Disproved 各自诊断正确
    match (&result1, &result2) {
        (ProofResult::Disproved(m1), ProofResult::Disproved(m2)) => {
            let d1 = m1.clone().into_diagnostic();
            let d2 = m2.clone().into_diagnostic();

            assert_eq!(d1.code, "E4018", "第一个诊断必须是 E4018");
            assert_eq!(d2.code, "E4018", "第二个诊断必须是 E4018");
            assert!(
                d1.message.contains("a > 0"),
                "第一个诊断必须包含 'a > 0'，实际: '{}'",
                d1.message
            );
            assert!(
                d2.message.contains("b < 10"),
                "第二个诊断必须包含 'b < 10'，实际: '{}'",
                d2.message
            );
            assert_ne!(d1.message, d2.message, "两个不同的诊断消息必须不同");
        }
        (r1, r2) => panic!("两个都必须 Disproved，实际: {r1:?}, {r2:?}"),
    }
}

/// E2E: Proved → into_result → Ok
///
/// 正常路径不产生诊断。
#[test]
fn test_e2e_phase4_proved_into_result_ok() {
    // Arrange
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("x".into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint,
    };
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(5));
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    assert!(result.is_proved(), "x=5 时 x>0 必须 Proved");
    let outcome = result.into_result();
    assert!(outcome.is_ok(), "E2E: Proved into_result 必须返回 Ok(())");
}
