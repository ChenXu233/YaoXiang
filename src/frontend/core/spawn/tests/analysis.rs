//! spawn 分析测试 — 基于 RFC-024 和 RFC-018
//!
//! RFC-024 §2.1: spawn 块的直接子表达式创建并行任务
//! RFC-024 §2.3: 编译期 DAG 分析，依赖驱动执行
//! RFC-024 §2.5: Resource 类型感知，同资源自动串行
//! RFC-024 §2.4: spawn for 数据并行
//! RFC-018 §5: spawn 块代码生成

use crate::ast::{BinOp, Block, Expr, PlanId, Stmt, StmtKind};
use crate::frontend::core::spawn::analysis::{
    analyze_reads_writes, analyze_spawn_body, analyze_spawn_for, build_execution_plan,
    is_direct_child, is_spawn_tail_expr,
};
use crate::frontend::core::types::{MonoType, TraitTable};
use crate::util::span::Span;
use std::collections::{HashMap, HashSet};

// 辅助函数

fn dummy_span() -> Span {
    Span::dummy()
}

fn empty_trait_table() -> TraitTable {
    TraitTable::default()
}

fn empty_var_types() -> HashMap<String, MonoType> {
    HashMap::new()
}

fn var_expr(name: &str) -> Expr {
    Expr::Var(name.to_string(), dummy_span())
}

fn assign_stmt(
    target: &str,
    rhs: Expr,
) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(var_expr(target)),
            right: Box::new(rhs),
            span: dummy_span(),
        })),
        span: dummy_span(),
        plan_id: PlanId::UNPLANNED,
    }
}

fn call_expr(
    func: &str,
    args: Vec<Expr>,
) -> Expr {
    Expr::call(
        var_expr(func),
        args,
        vec![],
        dummy_span(),
        PlanId::UNPLANNED,
    )
}

fn spawn_body(stmts: Vec<Stmt>) -> Block {
    Block {
        stmts,
        span: dummy_span(),
    }
}

#[test]
fn test_is_direct_child_with_expr_stmt() {
    // RFC-024 §2.1: 直接子表达式是顶层 StmtKind::Expr
    // Arrange
    let stmt = Stmt {
        kind: StmtKind::Expr(Box::new(var_expr("x"))),
        span: dummy_span(),
        plan_id: PlanId::UNPLANNED,
    };

    // Act
    let result = is_direct_child(&stmt);

    // Assert
    assert!(result, "StmtKind::Expr 应被视为直接子表达式");
}

// analyze_reads_writes — 基础

#[test]
fn test_reads_writes_var_reference() {
    // Arrange
    let expr = var_expr("x");

    // Act
    let (reads, writes, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(reads.contains("x"), "变量引用应被记入 reads");
    assert!(writes.is_empty(), "变量引用不应产生 writes");
}

// build_execution_plan

#[test]
fn test_plan_independent_tasks_single_group() {
    // RFC-024 §2.1: 无依赖的直接子表达式可并行执行
    // Arrange
    let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
        (HashSet::from(["a".into()]), HashSet::from(["t1".into()])),
        (HashSet::from(["b".into()]), HashSet::from(["t2".into()])),
    ];
    let resource_sets: Vec<HashSet<String>> = vec![HashSet::new(), HashSet::new()];

    // Act
    let plan = build_execution_plan(&sets, &resource_sets);

    // Assert
    assert_eq!(plan.groups.len(), 1, "无依赖任务应在同一并行组");
    assert_eq!(
        plan.groups[0].task_indices.len(),
        2,
        "两个独立任务应都在组内"
    );
}

// analyze_spawn_body 端到端

#[test]
fn test_spawn_body_independent_tasks() {
    // RFC-024 §2.1: t1 = f(), t2 = g() → 无依赖，并行执行
    // Arrange
    let body = spawn_body(vec![
        assign_stmt("t1", call_expr("f", vec![])),
        assign_stmt("t2", call_expr("g", vec![])),
    ]);

    // Act
    let analysis = analyze_spawn_body(&body, &empty_trait_table(), &empty_var_types());

    // Assert
    assert_eq!(analysis.tasks.len(), 2, "应识别出 2 个任务");
    assert_eq!(
        analysis.tasks[0].target.as_deref(),
        Some("t1"),
        "第一个任务目标应为 t1"
    );
    assert_eq!(
        analysis.tasks[1].target.as_deref(),
        Some("t2"),
        "第二个任务目标应为 t2"
    );
    assert_eq!(analysis.plan.groups.len(), 1, "无依赖任务应在同一并行组");
}

// analyze_spawn_for

#[test]
fn test_spawn_for_reads_writes() {
    // RFC-024 §2.4: spawn for 展开为 N 个任务
    // Arrange
    let body = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(call_expr("process", vec![var_expr("item")]))),
            span: dummy_span(),
            plan_id: PlanId::UNPLANNED,
        }],
        span: dummy_span(),
    };
    let iterable = var_expr("items");

    // Act
    let analysis = analyze_spawn_for(
        "item",
        &iterable,
        &body,
        &empty_trait_table(),
        &empty_var_types(),
    );

    // Assert
    assert_eq!(analysis.iter_var, "item", "迭代变量名应为 item");
    assert!(
        analysis.reads.contains("process"),
        "循环体 reads 应包含函数名 process"
    );
    assert!(
        analysis.reads.contains("item"),
        "循环体 reads 应包含迭代变量 item"
    );
}

// is_spawn_tail_expr（#365）

/// 构造裸表达式语句
fn expr_stmt(expr: Expr) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(expr)),
        span: dummy_span(),
        plan_id: PlanId::UNPLANNED,
    }
}

#[test]
fn test_spawn_tail_expr_single_expression() {
    // #365: 唯一语句且为表达式 → 尾表达式（块值出口，不是并行任务）
    // Arrange
    let body = spawn_body(vec![expr_stmt(var_expr("x"))]);

    // Act
    let result = is_spawn_tail_expr(&body, 0);

    // Assert
    assert!(result, "末位表达式应为尾表达式");
}

#[test]
fn test_spawn_body_excludes_tail_expr_from_tasks() {
    // #365 回归: 尾表达式不得成为并行任务
    // Arrange
    let body = spawn_body(vec![
        assign_stmt("a", call_expr("fetch", vec![])),
        expr_stmt(var_expr("a")),
    ]);

    // Act
    let analysis = analyze_spawn_body(&body, &empty_trait_table(), &empty_var_types());

    // Assert
    assert_eq!(
        analysis.tasks.len(),
        1,
        "只有赋值是任务，尾表达式不是；修复前会误产出 2 个任务"
    );
    assert_eq!(
        analysis.tasks[0].target.as_deref(),
        Some("a"),
        "唯一任务目标是赋值目标 a"
    );
}
