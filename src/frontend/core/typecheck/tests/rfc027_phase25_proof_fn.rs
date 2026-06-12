//! RFC-027 Section 4.2 Phase 2.5 证明函数集成测试
//!
//! 测试第四级分派——ConstExpr::Call 被识别为证明函数调用。
//!
//! 四级分派路径：
//!   1. Evaluator 直接求值（Phase 1）
//!   2. 假设栈蕴含（Phase 2A）
//!   3. Z3 SMT 求解（Phase 2B）
//!   4. **证明函数调用（Phase 2.5）** ← 本文件覆盖

use std::collections::HashMap;

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::typecheck::layers::predicate::check_predicate;
use crate::frontend::core::typecheck::proof::context::ProofContext;
use crate::frontend::core::typecheck::proof::verdict::{ProofResult, UnprovenReason};
use crate::frontend::core::typecheck::TypeEnvironment;

// =========== 函数调用识别（第四级） ===========

#[test]
fn test_call_constraint_produces_proof_fn_call() {
    // 约束: Sorted(42) — ConstExpr::Call 形式
    // Phase 1-2 无法处理 → 应返回 Unproven + proof_calls
    let call_expr = ConstExpr::Call {
        func: "Sorted".into(),
        args: vec![ConstExpr::Lit(ConstValue::Int(42))],
    };

    let refined = MonoType::Refined {
        base: Box::new(MonoType::Bool),
        constraint: call_expr,
    };

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    match result {
        ProofResult::Unproven {
            ref proof_calls,
            ref reason,
            ..
        } => {
            assert!(
                !proof_calls.is_empty(),
                "ConstExpr::Call 应产生至少一个 ProofFunctionCall，实际: {proof_calls:?}"
            );
            assert_eq!(
                proof_calls[0].func_name, "Sorted",
                "函数名应为 Sorted"
            );
            assert_eq!(proof_calls[0].args.len(), 1, "应有一个实参");
            assert!(
                matches!(reason, UnprovenReason::ProofFunctionRequired),
                "原因应为 ProofFunctionRequired，实际: {reason:?}"
            );
        }
        other => panic!("期望 Unproven + proof_calls，实际: {other:?}"),
    }
}

#[test]
fn test_literal_constraint_does_not_produce_proof_call() {
    // 5 > 0 — 纯字面量，eval_expr 直接处理
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    };

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    assert!(
        result.is_proved(),
        "5 > 0 应在第 1 级 eval_expr 中 Proved，不应产生 proof_calls"
    );
}

#[test]
fn test_non_call_unproven_has_empty_proof_calls() {
    // 含未绑定变量 → eval_expr 失败 → SMT Unknown → Unproven
    // 但不是 Call 形式，proof_calls 应为空
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: ConstExpr::BinOp {
            op: BinOp::Gt,
            left: Box::new(ConstExpr::NamedVar("unknown".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        },
    };

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &HashMap::new());

    // 不是 Proved → 可能是 Disproved（Z3 找到反例）或 Unproven
    if let ProofResult::Unproven { proof_calls, .. } = &result {
        assert!(
            proof_calls.is_empty(),
            "非 Call 约束不应产生 proof_calls，实际: {proof_calls:?}"
        );
    }
}

#[test]
fn test_call_with_named_var_args() {
    // 约束: Sorted(arr) 其中 arr 是已绑定变量
    let call_expr = ConstExpr::Call {
        func: "Sorted".into(),
        args: vec![ConstExpr::NamedVar("arr".into())],
    };

    let refined = MonoType::Refined {
        base: Box::new(MonoType::Bool),
        constraint: call_expr,
    };

    let mut bindings = HashMap::new();
    bindings.insert("arr".into(), ConstValue::Int(1));

    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let result = check_predicate(&ctx, &refined, &bindings);

    match result {
        ProofResult::Unproven { proof_calls, .. } => {
            assert!(
                !proof_calls.is_empty(),
                "Call + NamedVar 应产生 ProofFunctionCall，args 从 bindings 解析"
            );
        }
        other => panic!("期望 Unproven, got {other:?}"),
    }
}
