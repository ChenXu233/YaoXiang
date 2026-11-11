//! assumptions 模块测试 — 基于语言规范 §3.11 & RFC-027 §3.2-3.3
//!
//! §3.2-3.3: 流敏感假设集 Γ — 路径条件收集与 kill set
//! spec 2026-07-12-assert-refinement-unification-design.md §4.1-4.2: Γ 语义 + mut 失效

use crate::frontend::core::sema::test_util::{binop};
use crate::frontend::core::sema::proof::assumptions::{AssumptionStack, FlowSensitiveGamma};
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

fn make_gt(
    var: &str,
    n: i128,
) -> ConstExpr {
    binop(
        BinOp::Gt,
        ConstExpr::NamedVar(var.into()),
        ConstExpr::Lit(ConstValue::Int(n)),
    )
}

// 基本 push/pop 行为（向后兼容 API）

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

// FlowSensitiveGamma 新 API

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
