//! 分发管道测试
//!
//! 测试覆盖：
//! - dispatch 模式判断（CompileTime vs Runtime）
//! - dispatch_compiletime 的 ProofResult 映射
//! - dispatch_runtime 的 Γ 注入
//! - dispatch_pipeline 完整流程
//! - gamma.kill 在 mut reassign 时触发

use std::collections::HashMap;

use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::typecheck::layers::dispatch::{
    dispatch, dispatch_pipeline, dispatch_runtime, CompileTimeOutcome, DispatchMode,
    DispatchOutcome, RuntimeOutcome, extract_free_vars,
};
use crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma;
use crate::frontend::core::typecheck::proof::context::ProofContext;
use crate::frontend::core::typecheck::environment::TypeEnvironment;

// ===================================================================
// 辅助函数
// ===================================================================

/// 构造 x > val 约束
fn make_gt(
    var: &str,
    val: i128,
) -> ConstExpr {
    ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar(var.to_string())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(val))),
    }
}

// ===================================================================
// extract_free_vars
// ===================================================================

/// 自由变量提取：NamedVar
#[test]
fn test_extract_free_vars_named_var() {
    let expr = make_gt("x", 0);
    let vars = extract_free_vars(&expr);
    assert_eq!(vars, vec!["x"]);
}

/// 自由变量提取：纯字面量 → 空
#[test]
fn test_extract_free_vars_literal_only() {
    let expr = ConstExpr::Lit(ConstValue::Int(42));
    let vars = extract_free_vars(&expr);
    assert!(vars.is_empty());
}

/// 自由变量提取：二元运算中的多个变量
#[test]
fn test_extract_free_vars_multi_vars() {
    let expr = ConstExpr::BinOp {
        op: BinOp::And,
        left: Box::new(make_gt("x", 0)),
        right: Box::new(make_gt("y", 5)),
    };
    let vars = extract_free_vars(&expr);
    assert_eq!(vars, vec!["x", "y"]);
}

// ===================================================================
// dispatch — 分派模式判断
// ===================================================================

/// 所有变量都在 bindings 中 → CompileTime
#[test]
fn test_dispatch_all_bindings_compiletime() {
    let constraint = make_gt("x", 0);
    let mut bindings = HashMap::new();
    bindings.insert("x".to_string(), ConstValue::Int(5));
    let gamma = FlowSensitiveGamma::new();

    let mode = dispatch(&constraint, &bindings, &gamma);
    assert_eq!(mode, DispatchMode::CompileTime);
}

/// 无绑定的未知变量 → Runtime
#[test]
fn test_dispatch_no_bindings_runtime() {
    let constraint = make_gt("x", 0);
    let bindings = HashMap::new();
    let gamma = FlowSensitiveGamma::new();

    let mode = dispatch(&constraint, &bindings, &gamma);
    assert_eq!(mode, DispatchMode::Runtime);
}

/// 变量在 Γ 中 → CompileTime
#[test]
fn test_dispatch_in_gamma_compiletime() {
    let constraint = make_gt("x", 0);
    let bindings = HashMap::new();
    let mut gamma = FlowSensitiveGamma::new();
    // 注入 x > 5 到 Γ → x 被引用 → CompileTime
    gamma.inject(make_gt("x", 5));

    let mode = dispatch(&constraint, &bindings, &gamma);
    assert_eq!(mode, DispatchMode::CompileTime);
}

/// 纯字面量约束（无变量）→ CompileTime
#[test]
fn test_dispatch_literal_only_compiletime() {
    let constraint = ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(5))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    };
    let bindings = HashMap::new();
    let gamma = FlowSensitiveGamma::new();

    let mode = dispatch(&constraint, &bindings, &gamma);
    assert_eq!(mode, DispatchMode::CompileTime);
}

/// 部分变量在 bindings，部分不在 → Runtime
#[test]
fn test_dispatch_partial_bindings_runtime() {
    let expr = ConstExpr::BinOp {
        op: BinOp::And,
        left: Box::new(make_gt("x", 0)),
        right: Box::new(make_gt("y", 5)),
    };
    let mut bindings = HashMap::new();
    bindings.insert("x".to_string(), ConstValue::Int(5));
    let gamma = FlowSensitiveGamma::new();

    let mode = dispatch(&expr, &bindings, &gamma);
    assert_eq!(mode, DispatchMode::Runtime);
}

// ===================================================================
// dispatch_runtime — Γ 注入
// ===================================================================

/// Runtime 路径注入 Γ 并返回 InsertCheck
#[test]
fn test_dispatch_runtime_injects_gamma() {
    let constraint = make_gt("x", 0);
    let mut gamma = FlowSensitiveGamma::new();

    let outcome = dispatch_runtime(&mut gamma, &constraint);

    // 返回值检查
    assert!(
        matches!(&outcome, RuntimeOutcome::InsertCheck { .. }),
        "Runtime 路径应返回 InsertCheck"
    );

    // Γ 应包含注入的约束
    let current = gamma.current();
    assert!(current.contains(&constraint), "Γ 应包含注入的约束表达式");
}

/// 多次注入 Γ 可累积
#[test]
fn test_dispatch_runtime_multiple_injects() {
    let mut gamma = FlowSensitiveGamma::new();
    let c1 = make_gt("x", 0);
    let c2 = make_gt("y", 5);

    dispatch_runtime(&mut gamma, &c1);
    dispatch_runtime(&mut gamma, &c2);

    let current = gamma.current();
    assert_eq!(current.len(), 2, "Γ 应包含两个注入的约束");
    assert!(current.contains(&c1));
    assert!(current.contains(&c2));
}

// ===================================================================
// dispatch_pipeline — 完整管道
// ===================================================================

/// Runtime 路径 → dispatch_pipeline 返回 Runtime(InsertCheck)
#[test]
fn test_dispatch_pipeline_runtime_path() {
    let constraint = make_gt("x", 0);
    let bindings = HashMap::new();
    let mut gamma = FlowSensitiveGamma::new();
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: constraint.clone(),
    };

    let result = dispatch_pipeline(&mut gamma, &ctx, &constraint, &refined, &bindings);

    assert!(
        matches!(
            &result,
            DispatchOutcome::Runtime(RuntimeOutcome::InsertCheck { .. })
        ),
        "未知变量应走 Runtime 路径: {:?}",
        result
    );

    // 验证 Γ 已注入
    let current = gamma.current();
    assert_eq!(current.len(), 1, "Runtime 路径应向 Γ 注入约束");
}

/// CompileTime 路径（变量在 bindings 中）
#[test]
fn test_dispatch_pipeline_compiletime_with_bindings() {
    let constraint = make_gt("x", 0);
    let mut bindings = HashMap::new();
    bindings.insert("x".to_string(), ConstValue::Int(5));
    let mut gamma = FlowSensitiveGamma::new();
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: constraint.clone(),
    };

    let result = dispatch_pipeline(&mut gamma, &ctx, &constraint, &refined, &bindings);

    match &result {
        DispatchOutcome::CompileTime(outcome) => {
            // x=5, 约束 x>0 → 应 Erased（直接求值可证明）
            assert!(
                matches!(outcome, CompileTimeOutcome::Erased),
                "x=5, x>0 应在编译期证明: {:?}",
                outcome
            );
        }
        other => {
            // 无 SMT 后端或 evaluator 限制时可能不会 Proved → 接受其他结果
            // 核心测试目标是检查走的是 CompileTime 路径而非 Runtime
            panic!("应为 CompileTime 路径: {:?}", other);
        }
    }
}

// ===================================================================
// FlowSensitiveGamma — kill 行为验证
// ===================================================================

/// gamma.kill：mut 变量重新赋值后，相关假设被标记为 dead
#[test]
fn test_gamma_kill_on_mut_reassign() {
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_gt("x", 0));

    // 验证 kill 前假设活跃
    let before = gamma.current();
    assert_eq!(before.len(), 1, "kill 前 Γ 应包含一条假设");

    // kill x → 依赖 x 的假设被标记 dead
    gamma.kill("x");

    // 验证 kill 后假设不活跃
    let after = gamma.current();
    assert!(after.is_empty(), "kill 后 Γ 应清空含 x 的假设");
}

/// gamma.kill：无关变量不受影响
#[test]
fn test_gamma_kill_unrelated_var_unaffected() {
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_gt("x", 0));
    gamma.inject(make_gt("y", 5));

    gamma.kill("x");

    let current = gamma.current();
    assert_eq!(current.len(), 1, "只含 x 的假设应被 kill");
    assert!(current.contains(&make_gt("y", 5)), "不含 x 的假设应保留");
}

/// gamma.kill：Γ 为空时 kill 不 panic
#[test]
fn test_gamma_kill_empty_gamma() {
    let mut gamma = FlowSensitiveGamma::new();
    gamma.kill("x"); // 不应 panic
    assert!(gamma.current().is_empty());
}

// ===================================================================
// 完整的 dispatch → Runtime → Γ inject 链路
// ===================================================================

/// dispatch → dispatch_runtime → gamma.current() 包含约束
#[test]
fn test_dispatch_to_runtime_chain() {
    let constraint = make_gt("z", 10);
    let bindings = HashMap::new();
    let mut gamma = FlowSensitiveGamma::new();

    // 1. dispatch 判断为 Runtime
    let mode = dispatch(&constraint, &bindings, &gamma);
    assert_eq!(mode, DispatchMode::Runtime);

    // 2. dispatch_runtime 注入
    let outcome = dispatch_runtime(&mut gamma, &constraint);
    assert!(matches!(outcome, RuntimeOutcome::InsertCheck { .. }));

    // 3. gamma 包含约束
    let current = gamma.current();
    assert!(
        current.contains(&constraint),
        "Runtime 链路应在 Γ 中添加约束"
    );
}
