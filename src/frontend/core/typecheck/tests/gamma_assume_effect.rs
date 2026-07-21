//! GammaAssume 效应消费测试 — 基于 RFC-011 §4.3（编译期验证）
//!
//! 规范来源：
//! - RFC-011 §4.3: Assert(cond) 类型族 — true → Void（擦除）, false → Never
//! - spec 2026-07-15-type-body-block-effect-seed-design.md §4:
//!   assert(x > 0) 成功返回后将完整谓词 x > 0 注入流敏感 Γ
//!
//! 验证 ExpressionInferrer 在成功调用 `assert(x > 0)` 后把完整谓词
//! `x > 0` 注入流敏感 Γ。assert 通过 scope 内的 Fn 变量注册，
//! dep_env 注册 GammaAssume { predicate_arg: 0 }。

use std::collections::HashMap;

use crate::frontend::core::parser::ast::{BinOp as AstBinOp, Expr};
use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::typecheck::inference::ExpressionInferrer;
use crate::frontend::core::typecheck::passes::overload;
use crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::core::types::eval::dependent_types::{Effect, EffectSpec};
use crate::frontend::core::types::eval::dependent_types::DependentTypeEnv;
use crate::util::span::Span;

/// 构造 `x > 0` 的 AST 表达式
fn make_gt_zero() -> Expr {
    Expr::BinOp {
        op: AstBinOp::Gt,
        left: Box::new(Expr::Var("x".to_string(), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(0), Span::dummy())),
        span: Span::dummy(),
    }
}

/// 构造 `assert(x > 0)` 调用表达式
fn make_assert_call() -> Expr {
    Expr::Call {
        func: Box::new(Expr::Var("assert".to_string(), Span::dummy())),
        args: vec![make_gt_zero()],
        named_args: Vec::new(),
        span: Span::dummy(),
    }
}

/// 构造 Bool→Void 的函数类型（assert 的签名）
fn assert_fn_type() -> MonoType {
    MonoType::Fn {
        params: vec![MonoType::Bool],
        return_type: Box::new(MonoType::Void),
    }
}

#[test]
fn test_assert_call_injects_predicate_into_gamma() {
    // Arrange — scope 内注册 assert 和 x
    let mut scope = crate::frontend::core::typecheck::inference::scope::ScopeManager::new();
    scope.add_var(
        "assert".to_string(),
        PolyType::mono(assert_fn_type()),
        false,
        Span::default(),
    );
    scope.add_var(
        "x".to_string(),
        PolyType::mono(MonoType::Int(64)),
        false,
        Span::default(),
    );

    let mut solver = TypeConstraintSolver::default();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let native_signatures: HashMap<String, MonoType> = HashMap::new();

    let mut dep_env = DependentTypeEnv::new();
    dep_env.register_effect_spec(EffectSpec::new(
        "assert",
        vec![Effect::GammaAssume { predicate_arg: 0 }],
        true,
    ));

    let mut gamma = FlowSensitiveGamma::new();
    assert!(gamma.is_empty(), "Γ 初始应为空");

    let mut inferrer = ExpressionInferrer::with_native_signatures(
        &mut scope,
        &mut solver,
        &overload_candidates,
        &native_signatures,
    );
    inferrer.set_dep_env(&dep_env);
    inferrer.set_gamma(&mut gamma);

    // Act — 推断 assert(x > 0)
    let call_expr = make_assert_call();
    let result = inferrer.infer_expr(&call_expr);

    // Assert — 调用应成功返回 Void
    assert!(
        result.is_ok(),
        "assert(x > 0) 类型推断应成功: {:?}",
        result.err()
    );
    let resolved = solver.resolve_type(&result.unwrap());
    assert_eq!(
        resolved,
        MonoType::Void,
        "assert 返回类型应为 Void，实际: {:?}",
        resolved
    );

    // Assert — Γ 应包含完整谓词 x > 0
    let current = gamma.current();
    assert_eq!(
        current.len(),
        1,
        "Γ 应恰好包含 1 条假设，实际: {:?}",
        current
    );
    let expected = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("x".to_string())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    assert!(
        current.contains(&expected),
        "Γ 应包含完整谓词 (x > 0)，实际: {:?}",
        current
    );
}

/// 没有 dep_env / gamma 时，assert 调用仍应正常完成（不注入）
#[test]
fn test_assert_call_without_gamma_does_not_inject() {
    // Arrange — scope 注册 assert 与 x，但不设置 dep_env / gamma
    let mut scope = crate::frontend::core::typecheck::inference::scope::ScopeManager::new();
    scope.add_var(
        "assert".to_string(),
        PolyType::mono(assert_fn_type()),
        false,
        Span::default(),
    );
    scope.add_var(
        "x".to_string(),
        PolyType::mono(MonoType::Int(64)),
        false,
        Span::default(),
    );

    let mut solver = TypeConstraintSolver::default();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let native_signatures: HashMap<String, MonoType> = HashMap::new();

    let mut inferrer = ExpressionInferrer::with_native_signatures(
        &mut scope,
        &mut solver,
        &overload_candidates,
        &native_signatures,
    );
    // 不设置 dep_env / gamma

    // Act — 推断 assert(x > 0)
    let call_expr = make_assert_call();
    let result = inferrer.infer_expr(&call_expr);

    // Assert — 无 Γ 时调用仍应成功返回（不崩溃即合规）
    assert!(
        result.is_ok(),
        "无 Γ 时 assert 仍应成功: {:?}",
        result.err()
    );
}
