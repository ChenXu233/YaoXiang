//! assumptions 模块测试 — 基于语言规范 §3.11 & RFC-027 §3.2-3.3
//!
//! §3.2-3.3: 流敏感假设集 Γ — 路径条件收集与 kill set
//! spec 2026-07-12-assert-refinement-unification-design.md §4.1-4.2: Γ 语义 + mut 失效

use crate::frontend::core::sema::proof::assumptions::{AssumptionStack, FlowSensitiveGamma};
use crate::frontend::core::sema::test_util::binop;
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

// ── 6.9.4：证据边界（barrier）与常量等式事实 ────────────────────────────

/// 等式事实 `y == 42`
fn make_eq(
    var: &str,
    n: i128,
) -> ConstExpr {
    ConstExpr::BinOp {
        op: BinOp::Eq,
        left: Box::new(ConstExpr::NamedVar(var.into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(n))),
    }
}

/// T17（γ 层）：**体外事实不入体**——进入边界后，边界外注入的事实不可见。
#[test]
fn test_gamma_barrier_hides_outer_facts() {
    // Arrange — 模块层（无边界）注入一条等式事实
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_eq("y", 42));
    assert_eq!(gamma.visible().len(), 1, "边界外的等式事实应可见");

    // Act — 进入函数体 / 闭包体 / spawn 体边界
    gamma.enter_barrier();

    // Assert — 体外事实不可见（不是「被删」，而是「不在本边界口径内」）
    assert!(
        gamma.visible().is_empty(),
        "体外事实不得进入边界：实际 {:?}",
        gamma.visible()
    );
    assert!(gamma.is_empty(), "边界以上没有事实 ⇒ is_empty 为真");
    assert!(
        gamma.const_equalities().is_empty(),
        "体外等式不得成为界内证据：实际 {:?}",
        gamma.const_equalities()
    );
}

/// T18（γ 层）：**体内自产事实对体内可见**——边界内注入的事实是本边界的证据。
#[test]
fn test_gamma_barrier_exposes_inner_facts() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    let _outer = gamma.enter_barrier();

    // Act — 体内自产（裸 assert / 守卫）
    gamma.inject(make_eq("y", 42));

    // Assert
    assert_eq!(gamma.visible().len(), 1, "体内自产事实应对本边界可见");
    assert_eq!(
        gamma.const_equalities(),
        vec![("y".to_string(), ConstValue::Int(42))],
        "体内等式应作为常量等式事实读出"
    );
}

/// T19（γ 层）：**体内事实随体亡**——退出边界后体内事实不再可见，体外事实恢复。
#[test]
fn test_gamma_barrier_drops_inner_facts_on_restore() {
    // Arrange — 体外一条、体内一条
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_eq("outer", 1));
    let saved = gamma.enter_barrier();
    gamma.inject(make_eq("inner", 2));

    // Act
    gamma.restore_barrier(saved);

    // Assert — 只剩体外那条
    assert_eq!(
        gamma.visible(),
        vec![make_eq("outer", 1)],
        "体内事实应随体亡，体外事实应恢复"
    );
    assert!(
        gamma
            .const_equalities()
            .iter()
            .all(|(name, _)| name == "outer"),
        "体内等式不得越过边界成为证据：实际 {:?}",
        gamma.const_equalities()
    );
}

/// T20（γ 层）：嵌套体退出后边界还原到**外层体**深度，不是 0。
#[test]
fn test_gamma_barrier_nested_restores_outer_depth() {
    // Arrange — 外层体事实 + 嵌套体（lambda/spawn 里的另一道边界）
    let mut gamma = FlowSensitiveGamma::new();
    let outer = gamma.enter_barrier();
    gamma.inject(make_eq("y", 42));
    let inner = gamma.enter_barrier();
    gamma.inject(make_eq("z", 7));

    // Act
    gamma.restore_barrier(inner);

    // Assert — 嵌套体事实亡、外层体事实仍在
    assert_eq!(
        gamma.visible(),
        vec![make_eq("y", 42)],
        "嵌套体退出应还原到外层体边界，外层体事实仍可见"
    );
    let restored = gamma.const_equalities();
    assert_eq!(
        restored,
        vec![("y".to_string(), ConstValue::Int(42))],
        "嵌套体退出后只有外层体等式是证据"
    );
    gamma.restore_barrier(outer);
    assert!(gamma.is_empty(), "退出外层体后 Γ 应回到无事实状态");
}

/// 只有**等式**是第四证据源：谓词守卫（`y > 0`）不产值。
#[test]
fn test_gamma_const_equalities_ignores_predicate_facts() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_gt("y", 0));

    // Assert
    assert!(
        gamma.const_equalities().is_empty(),
        "谓词守卫不是常量等式，不应产值：实际 {:?}",
        gamma.const_equalities()
    );
}

/// 两侧都是名字（`y == z`）不是**常量**等式，不产值。
#[test]
fn test_gamma_const_equalities_skips_open_equality() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(ConstExpr::BinOp {
        op: BinOp::Eq,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::NamedVar("z".into())),
    });

    // Assert
    assert!(
        gamma.const_equalities().is_empty(),
        "开等式（两侧都有自由名）不产值：实际 {:?}",
        gamma.const_equalities()
    );
}

/// 反向书写同义：`42 == y` 与 `y == 42` 给出同一 (名字, 值)。
#[test]
fn test_gamma_const_equalities_reads_either_operand_order() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(ConstExpr::BinOp {
        op: BinOp::Eq,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(42))),
        right: Box::new(ConstExpr::NamedVar("y".into())),
    });

    // Assert
    assert_eq!(
        gamma.const_equalities(),
        vec![("y".to_string(), ConstValue::Int(42))],
        "`42 == y` 与 `y == 42` 同义"
    );
}

/// 闭常量侧走唯一归一化入口：`y == 1 + 1` 的等式值是 2（不是「不可判」）。
#[test]
fn test_gamma_const_equalities_normalizes_closed_side() {
    // Arrange — y == (1 + 1)
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(ConstExpr::BinOp {
        op: BinOp::Eq,
        left: Box::new(ConstExpr::NamedVar("y".into())),
        right: Box::new(ConstExpr::BinOp {
            op: BinOp::Add,
            left: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
            right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
        }),
    });

    // Assert
    assert_eq!(
        gamma.const_equalities(),
        vec![("y".to_string(), ConstValue::Int(2))],
        "闭常量侧应经唯一归一化入口求值"
    );
}

/// `kill` 同样作用于等式事实（mut 重赋值 / `&mut` 借出后不再成立）。
#[test]
fn test_gamma_kill_removes_equality_fact() {
    // Arrange
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(make_eq("y", 42));

    // Act
    gamma.kill("y");

    // Assert
    assert!(
        gamma.const_equalities().is_empty(),
        "被 kill 的等式不得再作为证据：实际 {:?}",
        gamma.const_equalities()
    );
}
