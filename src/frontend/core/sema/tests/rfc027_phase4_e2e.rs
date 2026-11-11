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

use crate::frontend::core::sema::test_util::{binop, refined_int};
use std::collections::HashMap;

use crate::frontend::core::sema::layers::equivalence::check_type_equivalence;
use crate::frontend::core::sema::layers::predicate::check_predicate;
use crate::frontend::core::sema::proof::context::ProofContext;
use crate::frontend::core::sema::proof::verdict::{DisproofKind, ProofFailure, ProofResultExt};
use crate::frontend::core::sema::TypeEnvironment;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;

/// 单变量约束 `var <op> n` 的精化类型 + 反例绑定 `var = bound`。
fn constrained(
    var: &str,
    op: BinOp,
    n: i128,
    bound: i128,
) -> (MonoType, HashMap<String, ConstValue>) {
    let constraint = binop(
        op,
        ConstExpr::NamedVar(var.into()),
        ConstExpr::Lit(ConstValue::Int(n)),
    );
    let mut bindings = HashMap::new();
    bindings.insert(var.into(), ConstValue::Int(bound));
    (refined_int(constraint), bindings)
}

/// E2E: Level 1 Disproved → constraint 文本包含正确的谓词表达式
#[test]
fn test_e2e_phase4_disproved_model_has_constraint_text() {
    // Arrange
    let constraint = binop(
        BinOp::Gt,
        ConstExpr::NamedVar("x".into()),
        ConstExpr::Lit(ConstValue::Int(0)),
    );
    let refined = refined_int(constraint);
    let mut bindings = HashMap::new();
    bindings.insert("x".into(), ConstValue::Int(0));
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    match result {
        Err(ProofFailure::Disproved(model)) => {
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
    let constraint = binop(
        BinOp::Gt,
        ConstExpr::NamedVar("y".into()),
        ConstExpr::Lit(ConstValue::Int(0)),
    );
    let refined = refined_int(constraint);
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act — 无假设，SMT 应找到 y=0 反例
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // Assert
    match result {
        Err(ProofFailure::Disproved(model)) => {
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
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange
    let (refined, bindings) = constrained("x", BinOp::Gt, 0, 0);
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_predicate(&ctx, &refined, &bindings);

    // Assert
    match result {
        Err(ProofFailure::Disproved(model)) => {
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

/// E2E: into_result() — Disproved 转 Err(Diagnostic)
///
/// 验证完整管道：check_predicate → into_result → Err(E4018)
#[test]
fn test_e2e_phase4_into_result_disproved_to_err() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange
    let constraint = binop(
        BinOp::Gt,
        ConstExpr::NamedVar("x".into()),
        ConstExpr::Lit(ConstValue::Int(0)),
    );
    let refined = refined_int(constraint);
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

// RFC-027 §4 & Phase 4.1: TypeMismatch E2E

/// E2E: check_type_equivalence → TypeMismatch
///
/// 两个不同类型归约后不等 → Disproved(TypeMismatch)
#[test]
fn test_e2e_phase4_type_equivalence_type_mismatch() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange
    let lhs = MonoType::Int(64);
    let rhs = MonoType::Float(64);
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result = check_type_equivalence(&ctx, &lhs, &rhs);

    // Assert
    match result {
        Err(ProofFailure::Disproved(model)) => {
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

/// E2E: 批量 DisproofModel — 多个证明结果各自独立转换
#[test]
fn test_e2e_phase4_multiple_disproved_models_independent() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange — 两个独立的约束
    let (refined1, bindings1) = constrained("a", BinOp::Gt, 0, 0);
    let (refined2, bindings2) = constrained("b", BinOp::Lt, 10, 20);

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);

    // Act
    let result1 = check_predicate(&ctx, &refined1, &bindings1);
    let result2 = check_predicate(&ctx, &refined2, &bindings2);

    // Assert — 两个独立 Disproved 各自诊断正确
    match (&result1, &result2) {
        (Err(ProofFailure::Disproved(m1)), Err(ProofFailure::Disproved(m2))) => {
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
