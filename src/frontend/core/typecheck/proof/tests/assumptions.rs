//! assumptions 模块测试 — 基于语言规范 §3.11 & RFC-027 §3.2-3.3
//!
//! §3.2-3.3: 流敏感假设集 Γ — 路径条件收集与 kill set
//! spec 2026-07-12-assert-refinement-unification-design.md §4.1-4.2: Γ 语义 + mut 失效

use crate::frontend::core::typecheck::proof::assumptions::{AssumptionStack, FlowSensitiveGamma};
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

fn make_gt(
    var: &str,
    n: i128,
) -> ConstExpr {
    ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar(var.into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(n))),
    }
}

// ============================================================
// 基本 push/pop 行为（向后兼容 API）
// ============================================================

#[test]
fn test_push_pop() {
    // Arrange
    let mut stack = AssumptionStack::new();

    // Act & Assert
    assert!(stack.is_empty(), "newly created gamma should be empty");
    stack.enter_scope();
    stack.inject(make_gt("y", 0));
    assert_eq!(
        stack.current().len(),
        1,
        "one injected assumption should be present"
    );
    stack.exit_scope();
    assert!(stack.is_empty(), "stack should be empty after exit_scope");
}

#[test]
fn test_nested_push() {
    // Arrange
    let mut stack = AssumptionStack::new();
    stack.enter_scope();

    // Act
    stack.inject(make_gt("y", 0));
    stack.inject(make_gt("z", 5));

    // Assert
    assert_eq!(
        stack.current().len(),
        2,
        "two injected assumptions should be present"
    );
    stack.exit_scope();
    assert!(stack.is_empty(), "stack should be empty after exit_scope");
}

#[test]
fn test_contains_true() {
    // Arrange
    let mut stack = AssumptionStack::new();
    let cond = make_gt("y", 0);

    // Act
    stack.inject(cond.clone());

    // Assert
    assert!(stack.contains(&cond), "injected assumption should be found");
}

#[test]
fn test_contains_false() {
    // Arrange
    let mut stack = AssumptionStack::new();
    stack.inject(make_gt("y", 0));

    // Act
    let result = stack.contains(&make_gt("z", 5));

    // Assert
    assert!(!result, "non-injected assumption should not be found");
}

#[test]
fn test_contains_empty_stack() {
    // Arrange
    let stack = AssumptionStack::new();

    // Act
    let result = stack.contains(&make_gt("y", 0));

    // Assert
    assert!(!result, "empty stack should not contain any assumption");
}

// ============================================================
// FlowSensitiveGamma 新 API
// ============================================================

#[test]
fn test_gamma_inject_and_current() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();

    // Assert initial state
    assert!(gamma.is_empty(), "newly created gamma should be empty");

    // Act
    gamma.inject(make_gt("x", 0));
    gamma.inject(make_gt("y", 5));

    // Assert
    let alive = gamma.current();
    assert_eq!(alive.len(), 2, "two injected assumptions should be alive");
    assert!(
        alive.contains(&make_gt("x", 0)),
        "assumption x > 0 should be alive"
    );
    assert!(
        alive.contains(&make_gt("y", 5)),
        "assumption y > 5 should be alive"
    );
}

#[test]
fn test_gamma_kill_removes_dependent() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_gt("x", 0));
    gamma.inject(make_gt("x", 5));

    // Act
    gamma.kill("x");

    // Assert
    assert!(
        gamma.current().is_empty(),
        "both assumptions involving x should be killed"
    );
}

#[test]
fn test_gamma_kill_preserves_independent() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_gt("x", 0));
    gamma.inject(make_gt("y", 5));

    // Act
    gamma.kill("x");

    // Assert
    let alive = gamma.current();
    assert_eq!(
        alive.len(),
        1,
        "only one assumption should survive after kill"
    );
    assert!(
        alive.contains(&make_gt("y", 5)),
        "assumption y > 5 should be preserved"
    );
}

#[test]
fn test_gamma_exit_scope_removes_all() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.enter_scope();
    gamma.inject(make_gt("x", 0));
    gamma.inject(make_gt("y", 5));

    // Act
    gamma.exit_scope();

    // Assert
    assert!(
        gamma.is_empty(),
        "all scope-local assumptions should be removed on exit_scope"
    );
}

#[test]
fn test_gamma_contains_ignores_dead() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    let cond = make_gt("x", 0);
    gamma.inject(cond.clone());

    // Act
    gamma.kill("x");

    // Assert
    assert!(
        !gamma.contains(&cond),
        "killed assumption should not be found by contains"
    );
}

#[test]
fn test_gamma_is_empty_with_dead() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_gt("x", 0));

    // Act
    gamma.kill("x");

    // Assert
    assert!(
        gamma.is_empty(),
        "gamma with only dead entries should be empty"
    );
}

#[test]
fn test_gamma_nested_scope_kill_only_inner() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_gt("x", 0));
    gamma.enter_scope();
    gamma.inject(make_gt("x", 5));

    // Act: kill x — kills both inner and outer since both reference x
    gamma.kill("x");

    // Assert: both scopes' x are dead
    assert!(
        gamma.current().is_empty(),
        "all entries referencing x should be dead after kill"
    );
    gamma.exit_scope();
    assert!(
        gamma.current().is_empty(),
        "outer scope entry referencing x should also be dead"
    );
}
