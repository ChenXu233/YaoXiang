//! γ 等式事实的**绑定身份**判据（6.9.4 修复 F-1）
//!
//! 规范锚：RFC-027 §3.2-3.3（流敏感假设集）/ RFC-039 D62（6.9.4 q=0 第四证据源）；
//! 09a §P6 6.9.4 修复轮 F-1。
//!
//! 为什么单独立测：`const_equalities_resolved` 的**绑定校验**是 F-1 的实质修复点，
//! 端到端只能看到「遮蔽形态被拒绝」，看不到三件事：①同一绑定下事实仍可用；
//! ②无快照/无定义事件的事实被判为**非证据**（fail-closed）；③证据边界的可见性
//! 判据没有被绑定校验挤掉。这三条必须直驱钉住。
//!
//! 口径先例（同源）：6.9.2 轮 t23（F1'）把编译期值挂在 per-binding 的
//! `VarInfo::const_value` 上、由 `get_var_info` 取最内层——本轮把**事实的出处**
//! 也按同一条解析链记成 `VarInfo::definition_span`，不新建名字表。

use crate::frontend::core::sema::inference::scope::ScopeManager;
use crate::frontend::core::sema::proof::assumptions::{
    BindingKey, FlowSensitiveGamma, binding_snapshot,
};
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::types::PolyType;
use crate::util::span::{Position, Span};

/// `name == n` 形态的等式事实。
fn eq(
    name: &str,
    n: i128,
) -> ConstExpr {
    ConstExpr::BinOp {
        op: BinOp::Eq,
        left: Box::new(ConstExpr::NamedVar(name.into())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(n))),
    }
}

/// 一组合法 span（模拟「某个绑定的定义事件」）。
fn key(
    line: usize,
    column: usize,
) -> Span {
    Span {
        start: Position {
            line,
            column,
            offset: line * 10 + column,
        },
        end: Position {
            line,
            column: column + 1,
            offset: line * 10 + column + 1,
        },
    }
}

/// 注入一条带绑定快照的事实（快照按给定解析器算）。
fn inject_with(
    gamma: &mut FlowSensitiveGamma,
    fact: ConstExpr,
    resolve: &dyn Fn(&str) -> Option<BindingKey>,
) {
    let names = crate::frontend::core::types::eval::const_eval::const_expr_free_names(&fact);
    let snapshot = names
        .into_iter()
        .map(|name| {
            let bound = resolve(&name);
            (name, bound)
        })
        .collect();
    gamma.inject_boundings(fact, snapshot);
}

/// 性质 ①：同一绑定下（解析回同一身份）事实可用。
#[test]
fn test_equalities_resolved_accepts_fact_under_same_binding() {
    // Arrange
    let binding_a = key(1, 5);
    let mut gamma = FlowSensitiveGamma::new();
    inject_with(&mut gamma, eq("y", 42), &|_| Some(binding_a));

    // Act
    let facts = gamma.const_equalities_resolved(&|_| Some(binding_a));

    // Assert
    assert_eq!(
        facts,
        vec![("y".to_string(), ConstValue::Int(42))],
        "同一绑定下的事实必须仍产值（绑定校验不是全量禁用）"
    );
}

/// 性质 ②（F-1 核心）：解析到**另一次同名绑定**时事实不产值。
#[test]
fn test_equalities_resolved_rejects_fact_under_other_binding() {
    // Arrange — 事实记的是外层绑定，读取时同名名字解析到臂绑定
    let outer = key(1, 5);
    let arm = key(9, 11);
    let mut gamma = FlowSensitiveGamma::new();
    inject_with(&mut gamma, eq("y", 42), &|_| Some(outer));

    // Act
    let facts = gamma.const_equalities_resolved(&|_| Some(arm));

    // Assert
    assert!(
        facts.is_empty(),
        "同名不同绑定必须视为两条事实（F-1 串事实的根因）；实际: {facts:?}"
    );
}

/// 性质 ③（fail-closed）：没有绑定快照的事实**不是证据**。
#[test]
fn test_equalities_resolved_rejects_fact_without_binding_snapshot() {
    // Arrange — 走不带快照的注入口
    let mut gamma = FlowSensitiveGamma::new();
    gamma.inject(eq("y", 42));

    // Act
    let facts = gamma.const_equalities_resolved(&|_| Some(key(1, 5)));

    // Assert
    assert!(
        facts.is_empty(),
        "无出处的等式不得当证据（宁可少证，不可错证）；实际: {facts:?}"
    );
}

/// 性质 ④（fail-closed）：dummy 定义事件不构成可核身份。
#[test]
fn test_equalities_resolved_rejects_dummy_binding_identity() {
    // Arrange — 快照记录的是 dummy span
    let mut gamma = FlowSensitiveGamma::new();
    inject_with(&mut gamma, eq("y", 42), &|_| Some(Span::default()));

    // Act
    let facts = gamma.const_equalities_resolved(&|_| Some(Span::default()));

    // Assert
    assert!(
        facts.is_empty(),
        "dummy 定义事件无法区分绑定，必须按不同一绑定处理；实际: {facts:?}"
    );
}

/// 性质 ⑤：未遮蔽的**其它名字**不受影响（只按名逐一校验）。
#[test]
fn test_equalities_resolved_keeps_other_names_fact() {
    // Arrange — z 的事实记外层绑定，y 的事实记臂绑定
    let outer_y = key(1, 5);
    let outer_z = key(2, 5);
    let arm_y = key(9, 11);
    let mut gamma = FlowSensitiveGamma::new();
    inject_with(&mut gamma, eq("y", 42), &|name| {
        Some(if name == "y" { outer_y } else { outer_z })
    });
    inject_with(&mut gamma, eq("z", 7), &|_| Some(outer_z));

    // Act
    let facts =
        gamma.const_equalities_resolved(&|name| Some(if name == "y" { arm_y } else { outer_z }));

    // Assert
    assert_eq!(
        facts,
        vec![("z".to_string(), ConstValue::Int(7))],
        "遮蔽 y 不影响 z 的事实"
    );
}

/// 性质 ⑥：证据边界判据仍在（边界外的事实不入体）。
#[test]
fn test_equalities_resolved_ignores_fact_outside_barrier() {
    // Arrange — 体外注入，再进体
    let binding = key(1, 5);
    let mut gamma = FlowSensitiveGamma::new();
    inject_with(&mut gamma, eq("y", 42), &|_| Some(binding));
    let prev = gamma.enter_barrier();

    // Act
    let inside = gamma.const_equalities_resolved(&|_| Some(binding));

    // Assert
    assert!(
        inside.is_empty(),
        "体外事实不得入体（barrier 判据未被绑定校验挤掉）"
    );
    gamma.restore_barrier(prev);
}

/// 性质 ⑦：永久 kill 仍生效（绑定校验不绕过 kill）。
#[test]
fn test_equalities_resolved_keeps_killed_fact_dead() {
    // Arrange
    let binding = key(1, 5);
    let mut gamma = FlowSensitiveGamma::new();
    inject_with(&mut gamma, eq("y", 42), &|_| Some(binding));
    gamma.kill("y");

    // Act
    let facts = gamma.const_equalities_resolved(&|_| Some(binding));

    // Assert
    assert!(
        facts.is_empty(),
        "重赋值 kill 的事实不得因绑定身份相符而复活；实际: {facts:?}"
    );
}

/// 捕获侧：快照按自由名解析，逐名记录**此刻**的最内层绑定。
#[test]
fn test_binding_snapshot_records_innermost_binding_per_name() {
    // Arrange — 作用域里只有一个绑定 y
    let definition = key(3, 7);
    let mut scope = ScopeManager::new();
    scope.add_var(
        "y".to_string(),
        PolyType::mono(MonoType::Int(64)),
        false,
        definition,
    );

    // Act
    let snapshot = binding_snapshot(&scope, &eq("y", 42));

    // Assert
    assert_eq!(
        snapshot,
        vec![("y".to_string(), Some(definition))],
        "快照必须记录该名字此刻最内层绑定的定义事件"
    );
}
