//! dispatch 分派管道测试 — spec §2.2-2.5 + RFC-027 §3.2-3.3 + §11
//!
//! §2.2: dispatch 先于证明发生
//! §2.3: 编译期可及 = 值编译期已知 ∪ Γ 中假设可推
//! §2.5: 运行时轨道理论必要
//! spec 2026-07-12-assert-refinement-unification-design.md §2.2-2.5

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
    assert_eq!(vars, vec!["x"], "extract_free_vars should find 'x' in x>0");
}
/// 自由变量提取：纯字面量 → 空
#[test]
fn test_extract_free_vars_literal_only() {
    let expr = ConstExpr::Lit(ConstValue::Int(42));
    let vars = extract_free_vars(&expr);
    assert!(
        vars.is_empty(),
        "literal-only expr should have no free vars"
    );
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
    assert_eq!(
        vars,
        vec!["x", "y"],
        "extract_free_vars should find both 'x' and 'y' in x>0 AND y>5"
    );
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

    // Act
    let mode = dispatch(&constraint, &bindings, &gamma);
    // Assert
    assert_eq!(
        mode,
        DispatchMode::CompileTime,
        "var in bindings → CompileTime"
    );
}

/// 无绑定的未知变量 → Runtime
#[test]
fn test_dispatch_no_bindings_runtime() {
    let constraint = make_gt("x", 0);
    let bindings = HashMap::new();
    let gamma = FlowSensitiveGamma::new();
    // Act
    let mode = dispatch(&constraint, &bindings, &gamma);
    // Assert
    assert_eq!(mode, DispatchMode::Runtime, "unknown var → Runtime");
}

/// 变量在 Γ 中 → CompileTime
#[test]
fn test_dispatch_in_gamma_compiletime() {
    let constraint = make_gt("x", 0);
    let bindings = HashMap::new();
    let mut gamma = FlowSensitiveGamma::new();
    // 注入 x > 5 到 Γ → x 被引用 → CompileTime
    gamma.inject(make_gt("x", 5));
    // Act
    let mode = dispatch(&constraint, &bindings, &gamma);
    // Assert
    assert_eq!(mode, DispatchMode::CompileTime, "var in Γ → CompileTime");
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
    // Act
    let mode = dispatch(&constraint, &bindings, &gamma);
    // Assert
    assert_eq!(
        mode,
        DispatchMode::CompileTime,
        "literal-only constraint → CompileTime"
    );
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
    // Act
    let mode = dispatch(&expr, &bindings, &gamma);
    // Assert
    assert_eq!(mode, DispatchMode::Runtime, "partial bindings → Runtime");
}

// ===================================================================
// dispatch_runtime — Γ 注入
// ===================================================================

/// Runtime 路径注入 Γ 并返回 InsertCheck
#[test]
fn test_dispatch_runtime_injects_gamma() {
    // Arrange
    let constraint = make_gt("x", 0);
    let mut gamma = FlowSensitiveGamma::new();

    // Act
    let outcome = dispatch_runtime(&mut gamma, &constraint);

    // Assert
    assert!(
        matches!(&outcome, RuntimeOutcome::InsertCheck { .. }),
        "Runtime 路径应返回 InsertCheck"
    );
    let current = gamma.current();
    assert!(current.contains(&constraint), "Γ 应包含注入的约束表达式");
}

/// 多次注入 Γ 可累积
#[test]
fn test_dispatch_runtime_multiple_injects() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    let c1 = make_gt("x", 0);
    let c2 = make_gt("y", 5);

    // Act
    dispatch_runtime(&mut gamma, &c1);
    dispatch_runtime(&mut gamma, &c2);

    // Assert
    let current = gamma.current();
    assert_eq!(current.len(), 2, "Γ 应包含两个注入的约束");
    assert!(current.contains(&c1), "Γ 应包含首次注入的约束 c1");
    assert!(current.contains(&c2), "Γ 应包含二次注入的约束 c2");
}

// ===================================================================
// dispatch_pipeline — 完整管道
// ===================================================================

/// Runtime 路径 → dispatch_pipeline 返回 Runtime(InsertCheck)
#[test]
fn test_dispatch_pipeline_runtime_path() {
    // Arrange
    let constraint = make_gt("x", 0);
    let bindings = HashMap::new();
    let mut gamma = FlowSensitiveGamma::new();
    let env = TypeEnvironment::new();
    let ctx = ProofContext::new(&env);
    let refined = MonoType::Refined {
        base: Box::new(MonoType::Int(64)),
        constraint: constraint.clone(),
    };

    // Act
    let result = dispatch_pipeline(&mut gamma, &ctx, &constraint, &refined, &bindings);

    // Assert
    assert!(
        matches!(
            &result,
            DispatchOutcome::Runtime(RuntimeOutcome::InsertCheck { .. })
        ),
        "未知变量应走 Runtime 路径: {:?}",
        result
    );
    let current = gamma.current();
    assert_eq!(current.len(), 1, "Runtime 路径应向 Γ 注入约束");
}

/// CompileTime 路径（变量在 bindings 中）
#[test]
fn test_dispatch_pipeline_compiletime_with_bindings() {
    // Arrange
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

    // Act
    let result = dispatch_pipeline(&mut gamma, &ctx, &constraint, &refined, &bindings);

    // Assert
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
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_gt("x", 0));

    // Act
    let before = gamma.current();
    assert_eq!(before.len(), 1, "kill 前 Γ 应包含一条假设");
    gamma.kill("x");

    // Assert
    let after = gamma.current();
    assert!(after.is_empty(), "kill 后 Γ 应清空含 x 的假设");
}

/// gamma.kill：无关变量不受影响
#[test]
fn test_gamma_kill_unrelated_var_unaffected() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_gt("x", 0));
    gamma.inject(make_gt("y", 5));

    // Act
    gamma.kill("x");

    // Assert
    let current = gamma.current();
    assert_eq!(current.len(), 1, "只含 x 的假设应被 kill");
    assert!(current.contains(&make_gt("y", 5)), "不含 x 的假设应保留");
}

/// gamma.kill：Γ 为空时 kill 不 panic
#[test]
fn test_gamma_kill_empty_gamma() {
    let mut gamma = FlowSensitiveGamma::new();
    gamma.kill("x"); // 不应 panic
    assert!(gamma.current().is_empty(), "为空 Γ 执行 kill 后应仍为空");
}

// ===================================================================
// 完整的 dispatch → Runtime → Γ inject 链路
// ===================================================================

/// dispatch → dispatch_runtime → gamma.current() 包含约束
#[test]
fn test_dispatch_to_runtime_chain() {
    // Arrange
    let constraint = make_gt("z", 10);
    let bindings = HashMap::new();
    let mut gamma = FlowSensitiveGamma::new();

    // Act
    let mode = dispatch(&constraint, &bindings, &gamma);
    let outcome = dispatch_runtime(&mut gamma, &constraint);

    // Assert
    assert_eq!(mode, DispatchMode::Runtime, "unknown var → Runtime");
    assert!(
        matches!(outcome, RuntimeOutcome::InsertCheck { .. }),
        "Runtime 注入应返回 InsertCheck"
    );
    let current = gamma.current();
    assert!(
        current.contains(&constraint),
        "Runtime 链路应在 Γ 中添加约束"
    );
}
