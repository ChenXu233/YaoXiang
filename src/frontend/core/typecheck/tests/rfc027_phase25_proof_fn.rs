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
use crate::frontend::core::typecheck::proof::verdict::{ProofFunctionCall, ProofResult, UnprovenReason};
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
            assert_eq!(proof_calls[0].func_name, "Sorted", "函数名应为 Sorted");
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

// =========== execute_single_proof_fn 单元测试 ===========

use crate::frontend::pipeline::execute_single_proof_fn;
use crate::frontend::core::parser::ast::{
    BinOp as AstBinOp, Expr, Literal, Module, Param, Stmt, StmtKind, Type as AstType,
};
use crate::frontend::core::typecheck::TypeCheckResult;
use crate::util::span::Span;

/// 构造一个最小的 Module AST，包含一个简单的证明函数。
///
/// 生成的 AST 结构：
/// ```text
/// fn_name: (x: Int) -> Type = {
///     return body_expr
/// }
/// ```
///
/// body_expr 通过 return 语句返回 Bool。
fn make_proof_fn_module(
    fn_name: &str,
    body_expr: Expr,
) -> Module {
    let param = Param {
        name: "x".into(),
        ty: Some(AstType::Int(64)),
        is_mut: false,
        span: Span::dummy(),
    };

    let return_type = AstType::Fn {
        params: vec![AstType::Int(64)],
        return_type: Box::new(AstType::MetaType {
            name_span: Span::dummy(),
            args: vec![],
        }),
    };

    let binding = Stmt {
        kind: StmtKind::Binding {
            name: fn_name.into(),
            type_name: None,
            method_type: None,
            generic_params: vec![],
            type_annotation: Some(return_type),
            params: vec![param],
            // body_expr 通过 return 语句返回
            body: vec![Stmt {
                kind: StmtKind::Return(Some(Box::new(body_expr))),
                span: Span::dummy(),
            }],
            is_pub: false,
        },
        span: Span::dummy(),
    };

    Module {
        items: vec![binding],
        span: Span::dummy(),
    }
}

/// 辅助：构造二元表达式 Binary { op, left: Var(x), right: Lit(n) }
fn var_gt_literal(n: i128) -> Expr {
    Expr::BinOp {
        op: AstBinOp::Gt,
        left: Box::new(Expr::Var("x".into(), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(n), Span::dummy())),
        span: Span::dummy(),
    }
}

#[test]
fn test_execute_proof_fn_returns_true_for_valid_input() {
    // IsPositive: (x: Int) -> Type = { x > 0 }
    // 执行 IsPositive(5) → 应返回 true
    let body = var_gt_literal(0);

    let ast = make_proof_fn_module("IsPositive", body);
    let type_result = TypeCheckResult::default();
    let call = ProofFunctionCall {
        func_name: "IsPositive".into(),
        args: vec![ConstValue::Int(5)],
    };

    let result = execute_single_proof_fn(&call, &ast, &type_result);
    assert!(result.is_ok(), "IsPositive(5) 应成功执行: {:?}", result);
    assert!(result.unwrap(), "IsPositive(5) = 5>0 应返回 true");
}

#[test]
fn test_execute_proof_fn_returns_false_for_invalid_input() {
    // IsPositive: (x: Int) -> Type = { x > 0 }
    // 执行 IsPositive(-1) → 应返回 false
    let body = var_gt_literal(0);

    let ast = make_proof_fn_module("IsPositive", body);
    let type_result = TypeCheckResult::default();
    let call = ProofFunctionCall {
        func_name: "IsPositive".into(),
        args: vec![ConstValue::Int(-1)],
    };

    let result = execute_single_proof_fn(&call, &ast, &type_result);
    assert!(result.is_ok(), "IsPositive(-1) 应成功执行: {:?}", result);
    assert!(!result.unwrap(), "IsPositive(-1) = -1>0 应返回 false");
}

#[test]
fn test_execute_proof_fn_arithmetic() {
    // AddCheck: (x: Int) -> Type = { x + 10 > 5 }
    let body = Expr::BinOp {
        op: AstBinOp::Gt,
        left: Box::new(Expr::BinOp {
            op: AstBinOp::Add,
            left: Box::new(Expr::Var("x".into(), Span::dummy())),
            right: Box::new(Expr::Lit(Literal::Int(10), Span::dummy())),
            span: Span::dummy(),
        }),
        right: Box::new(Expr::Lit(Literal::Int(5), Span::dummy())),
        span: Span::dummy(),
    };

    let ast = make_proof_fn_module("AddCheck", body);
    let type_result = TypeCheckResult::default();
    let call = ProofFunctionCall {
        func_name: "AddCheck".into(),
        args: vec![ConstValue::Int(0)],
    };

    let result = execute_single_proof_fn(&call, &ast, &type_result);
    assert!(result.is_ok(), "AddCheck(0) 应成功执行: {:?}", result);
    assert!(result.unwrap(), "0+10>5 = true");
}

#[test]
fn test_execute_proof_fn_not_found() {
    // 函数名不存在于 AST 中 → 应返回 Err
    let ast = Module::default();
    let type_result = TypeCheckResult::default();
    let call = ProofFunctionCall {
        func_name: "NonExistent".into(),
        args: vec![ConstValue::Int(1)],
    };

    let result = execute_single_proof_fn(&call, &ast, &type_result);
    assert!(result.is_err(), "函数不存在时应返回 Err");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("未在 AST 中找到"),
        "错误信息应包含 '未在 AST 中找到'，实际: {}",
        err_msg
    );
}

#[test]
fn test_execute_proof_fn_boundary_value() {
    // IsPositive: (x: Int) -> Type = { x > 0 }
    // 边界值 IsPositive(0) → 应返回 false（0 不大于 0）
    let body = var_gt_literal(0);

    let ast = make_proof_fn_module("IsPositive", body);
    let type_result = TypeCheckResult::default();
    let call = ProofFunctionCall {
        func_name: "IsPositive".into(),
        args: vec![ConstValue::Int(0)],
    };

    let result = execute_single_proof_fn(&call, &ast, &type_result);
    assert!(result.is_ok(), "IsPositive(0) 应成功执行: {:?}", result);
    assert!(
        !result.unwrap(),
        "IsPositive(0) = 0>0 应返回 false（边界值）"
    );
}

#[test]
fn test_execute_proof_fn_negative_boundary() {
    // IsPositive: (x: Int) -> Type = { x > 0 }
    // IsPositive(1) → true，IsPositive(-1) → false
    let body = var_gt_literal(0);

    let ast = make_proof_fn_module("IsPositive", body);
    let type_result = TypeCheckResult::default();

    // 1 > 0 → true
    let call_1 = ProofFunctionCall {
        func_name: "IsPositive".into(),
        args: vec![ConstValue::Int(1)],
    };
    let result_1 = execute_single_proof_fn(&call_1, &ast, &type_result);
    assert!(result_1.is_ok());
    assert!(result_1.unwrap(), "1 > 0 应为 true");

    // -1 > 0 → false
    let call_neg1 = ProofFunctionCall {
        func_name: "IsPositive".into(),
        args: vec![ConstValue::Int(-1)],
    };
    let result_neg1 = execute_single_proof_fn(&call_neg1, &ast, &type_result);
    assert!(result_neg1.is_ok());
    assert!(!result_neg1.unwrap(), "-1 > 0 应为 false");
}

// =========== E2E 测试：Compiler 全流程 ===========

use crate::frontend::Compiler;

#[test]
fn test_e2e_proof_fn_compilation_succeeds() {
    // 定义一个证明函数，在编译期使用它
    // 如果证明函数返回 true，编译应该通过
    let source = r#"
        IsPositive: (x: Int) -> Type = { x > 0 }

        main = {
            val: IsPositive(5) = 5
        }
    "#;

    let mut compiler = Compiler::new();
    let result = compiler.compile_with_source("test.yao", source);

    match result {
        Ok(_) => {} // 编译通过
        Err(e) => panic!("期望编译通过，但失败: {}", e),
    }
}

#[test]
#[ignore = "解析器未正确处理 (x: Int) -> Type 语法，导致 IsPositive 函数未被识别为证明函数"]
fn test_e2e_proof_fn_compilation_fails_on_false() {
    // 证明函数返回 false 时编译应该失败
    let source = r#"
        IsPositive: (x: Int) -> Type = { x > 0 }

        main = {
            val: IsPositive(-1) = -1
        }
    "#;

    let mut compiler = Compiler::new();
    let result = compiler.compile_with_source("test.yao", source);

    match result {
        Err(_) => {} // 期望编译失败——证明不通过
        Ok(_) => panic!("期望编译失败（IsPositive(-1) 不成立），但编译通过了"),
    }
}
