//! 编译期 DAG 分析：分析 spawn 块内的依赖关系，生成执行计划
//!
//! RFC-024 核心：spawn 块的直接子表达式创建并行任务。
//! 编译器在编译期分析这些子表达式的读写依赖，通过拓扑排序
//! 将无依赖的任务分组，生成 ExecutionPlan。

use std::collections::HashSet;

use crate::frontend::core::parser::ast::{BinOp, Expr, Stmt, StmtKind};
use crate::middle::core::ir::{ExecutionPlan, TaskGroup};

/// spawn 块分析结果
pub struct SpawnAnalysis {
    /// 每个任务的表达式（从直接子表达式提取）
    pub task_exprs: Vec<Expr>,
    /// 每个任务的赋值目标变量名（如果直接子表达式是赋值）
    pub task_targets: Vec<Option<String>>,
    /// 执行计划
    pub plan: ExecutionPlan,
}

/// 判断语句是否是 spawn 块的直接子表达式
///
/// spawn 块内的直接子表达式是顶层表达式语句（`StmtKind::Expr`），
/// 这些语句将被包装为闭包并作为并行任务执行。
///
/// 其他语句类型（如变量声明、绑定等）不是直接子表达式，
/// 它们在 spawn 块内按顺序执行。
pub fn is_direct_child(stmt: &Stmt) -> bool {
    matches!(stmt.kind, StmtKind::Expr(_))
}

/// 分析表达式的变量读写集
pub fn analyze_reads_writes(expr: &Expr) -> (HashSet<String>, HashSet<String>) {
    let mut reads = HashSet::new();
    let mut writes = HashSet::new();
    collect_reads_writes(expr, &mut reads, &mut writes);
    (reads, writes)
}

fn collect_reads_writes(
    expr: &Expr,
    reads: &mut HashSet<String>,
    writes: &mut HashSet<String>,
) {
    match expr {
        Expr::Var(name, _) => {
            reads.insert(name.clone());
        }
        Expr::BinOp {
            op: BinOp::Assign,
            left,
            right,
            ..
        } => {
            if let Expr::Var(name, _) = left.as_ref() {
                writes.insert(name.clone());
            }
            collect_reads_writes(right, reads, writes);
        }
        Expr::Call { func, args, .. } => {
            collect_reads_writes(func, reads, writes);
            for arg in args {
                collect_reads_writes(arg, reads, writes);
            }
        }
        Expr::FieldAccess { expr, .. } => {
            collect_reads_writes(expr, reads, writes);
        }
        Expr::BinOp { left, right, .. } => {
            collect_reads_writes(left, reads, writes);
            collect_reads_writes(right, reads, writes);
        }
        Expr::UnOp { expr, .. } => {
            collect_reads_writes(expr, reads, writes);
        }
        _ => {}
    }
}

/// 构建依赖 DAG，拓扑排序生成执行计划
pub fn build_execution_plan(
    read_write_sets: &[(HashSet<String>, HashSet<String>)]
) -> ExecutionPlan {
    let n = read_write_sets.len();
    if n == 0 {
        return ExecutionPlan { groups: vec![] };
    }

    // 构建依赖关系：deps[i] = 任务 i 必须等待的任务列表
    let mut deps: Vec<Vec<usize>> = vec![vec![]; n];
    for i in 0..n {
        for j in 0..i {
            // 任务 i 读取了任务 j 写入的变量 → i 依赖 j（写后读）
            let reads_i = &read_write_sets[i].0;
            let writes_j = &read_write_sets[j].1;
            if !reads_i.is_disjoint(writes_j) {
                deps[i].push(j);
            }
            // 任务 i 写入了任务 j 写入的变量 → i 依赖 j（写后写）
            let writes_i = &read_write_sets[i].1;
            if !writes_i.is_disjoint(writes_j) && !deps[i].contains(&j) {
                deps[i].push(j);
            }
        }
    }

    // BFS 分层拓扑排序
    let mut groups = Vec::new();
    let mut remaining: HashSet<usize> = (0..n).collect();

    loop {
        // 找出所有依赖已满足的任务（可立即执行）
        let ready: Vec<usize> = remaining
            .iter()
            .filter(|&&i| deps[i].iter().all(|d| !remaining.contains(d)))
            .copied()
            .collect();

        if ready.is_empty() {
            break;
        }

        for &task in &ready {
            remaining.remove(&task);
        }
        groups.push(TaskGroup {
            task_indices: ready,
        });
    }

    // 处理剩余任务（环依赖）
    if !remaining.is_empty() {
        groups.push(TaskGroup {
            task_indices: remaining.into_iter().collect(),
        });
    }

    ExecutionPlan { groups }
}

/// 分析 spawn 块，生成执行计划
pub fn analyze_spawn_body(body: &crate::frontend::core::parser::ast::Block) -> SpawnAnalysis {
    let mut task_exprs = Vec::new();
    let mut task_targets = Vec::new();

    for stmt in &body.stmts {
        if !is_direct_child(stmt) {
            continue;
        }
        if let StmtKind::Expr(expr) = &stmt.kind {
            // 检查是否是赋值表达式：t1 = fetch(...)
            if let Expr::BinOp {
                op: BinOp::Assign,
                left,
                right,
                ..
            } = expr.as_ref()
            {
                if let Expr::Var(name, _) = left.as_ref() {
                    task_targets.push(Some(name.clone()));
                    task_exprs.push((**right).clone());
                } else {
                    task_targets.push(None);
                    task_exprs.push((**expr).clone());
                }
            } else {
                // 非赋值直接子表达式
                task_targets.push(None);
                task_exprs.push((**expr).clone());
            }
        }
    }

    // 分析读写依赖（对赋值表达式使用完整表达式以捕获写入目标）
    let read_write_sets: Vec<(HashSet<String>, HashSet<String>)> = body
        .stmts
        .iter()
        .filter(|stmt| is_direct_child(stmt))
        .filter_map(|stmt| {
            if let StmtKind::Expr(expr) = &stmt.kind {
                Some(analyze_reads_writes(expr))
            } else {
                None
            }
        })
        .collect();

    // 构建依赖 DAG 并拓扑排序
    let plan = build_execution_plan(&read_write_sets);

    SpawnAnalysis {
        task_exprs,
        task_targets,
        plan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::parser::ast::{Expr, Stmt, StmtKind};
    use crate::util::span::Span;

    fn dummy_span() -> Span {
        Span::dummy()
    }

    #[test]
    fn test_is_direct_child_with_expr_stmt() {
        let stmt = Stmt {
            kind: StmtKind::Expr(Box::new(Expr::Var("x".to_string(), dummy_span()))),
            span: dummy_span(),
        };
        assert!(is_direct_child(&stmt));
    }

    #[test]
    fn test_is_direct_child_with_var_decl() {
        let stmt = Stmt {
            kind: StmtKind::Var {
                name: "x".to_string(),
                name_span: dummy_span(),
                type_annotation: None,
                initializer: Some(Box::new(Expr::Lit(
                    crate::frontend::core::lexer::tokens::Literal::Int(42),
                    dummy_span(),
                ))),
                is_mut: false,
            },
            span: dummy_span(),
        };
        assert!(!is_direct_child(&stmt));
    }

    fn var_expr(name: &str) -> Expr {
        Expr::Var(name.to_string(), dummy_span())
    }

    #[test]
    fn test_var_read() {
        let expr = var_expr("x");
        let (reads, writes) = analyze_reads_writes(&expr);
        assert!(reads.contains("x"));
        assert!(writes.is_empty());
    }

    #[test]
    fn test_assign_write_and_rhs_read() {
        // t1 = fetch(a) → writes: {t1}, reads: {fetch, a}
        let expr = Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(var_expr("t1")),
            right: Box::new(Expr::Call {
                func: Box::new(var_expr("fetch")),
                args: vec![var_expr("a")],
                named_args: vec![],
                span: dummy_span(),
            }),
            span: dummy_span(),
        };
        let (reads, writes) = analyze_reads_writes(&expr);
        assert!(writes.contains("t1"));
        assert!(reads.contains("fetch"));
        assert!(reads.contains("a"));
    }

    #[test]
    fn test_call_args_read() {
        let expr = Expr::Call {
            func: Box::new(var_expr("process")),
            args: vec![var_expr("x"), var_expr("y")],
            named_args: vec![],
            span: dummy_span(),
        };
        let (reads, _) = analyze_reads_writes(&expr);
        assert!(reads.contains("process"));
        assert!(reads.contains("x"));
        assert!(reads.contains("y"));
    }

    #[test]
    fn test_no_deps_single_group() {
        // 两个独立任务 → 一个并行组
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
            (HashSet::from(["a".into()]), HashSet::from(["t1".into()])),
            (HashSet::from(["b".into()]), HashSet::from(["t2".into()])),
        ];
        let plan = build_execution_plan(&sets);
        assert_eq!(plan.groups.len(), 1);
        assert_eq!(plan.groups[0].task_indices.len(), 2);
    }

    #[test]
    fn test_raw_dep_two_groups() {
        // t2 读取 t1 → 两个串行组
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
            (HashSet::new(), HashSet::from(["t1".into()])),
            (HashSet::from(["t1".into()]), HashSet::from(["t2".into()])),
        ];
        let plan = build_execution_plan(&sets);
        assert_eq!(plan.groups.len(), 2);
        assert_eq!(plan.groups[0].task_indices, vec![0]);
        assert_eq!(plan.groups[1].task_indices, vec![1]);
    }

    #[test]
    fn test_diamond_two_groups() {
        // x, y 独立并行，z 依赖两者
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
            (HashSet::new(), HashSet::from(["x".into()])),
            (HashSet::new(), HashSet::from(["y".into()])),
            (
                HashSet::from(["x".into(), "y".into()]),
                HashSet::from(["z".into()]),
            ),
        ];
        let plan = build_execution_plan(&sets);
        assert_eq!(plan.groups.len(), 2);
        assert_eq!(plan.groups[0].task_indices.len(), 2);
        assert_eq!(plan.groups[1].task_indices, vec![2]);
    }

    #[test]
    fn test_empty_input() {
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![];
        let plan = build_execution_plan(&sets);
        assert_eq!(plan.groups.len(), 0);
    }

    #[test]
    fn test_write_write_conflict() {
        // 两个任务写同一变量 → 串行
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
            (HashSet::new(), HashSet::from(["x".into()])),
            (HashSet::new(), HashSet::from(["x".into()])),
        ];
        let plan = build_execution_plan(&sets);
        assert_eq!(plan.groups.len(), 2);
    }

    #[test]
    fn test_full_spawn_analysis_independent() {
        // t1 = f(), t2 = g() → 1 group, 2 tasks
        let body = crate::frontend::core::parser::ast::Block {
            stmts: vec![
                Stmt {
                    kind: StmtKind::Expr(Box::new(Expr::BinOp {
                        op: BinOp::Assign,
                        left: Box::new(Expr::Var("t1".into(), dummy_span())),
                        right: Box::new(Expr::Call {
                            func: Box::new(Expr::Var("f".into(), dummy_span())),
                            args: vec![],
                            named_args: vec![],
                            span: dummy_span(),
                        }),
                        span: dummy_span(),
                    })),
                    span: dummy_span(),
                },
                Stmt {
                    kind: StmtKind::Expr(Box::new(Expr::BinOp {
                        op: BinOp::Assign,
                        left: Box::new(Expr::Var("t2".into(), dummy_span())),
                        right: Box::new(Expr::Call {
                            func: Box::new(Expr::Var("g".into(), dummy_span())),
                            args: vec![],
                            named_args: vec![],
                            span: dummy_span(),
                        }),
                        span: dummy_span(),
                    })),
                    span: dummy_span(),
                },
            ],
            expr: None,
            span: dummy_span(),
        };
        let analysis = analyze_spawn_body(&body);
        assert_eq!(analysis.task_exprs.len(), 2);
        assert_eq!(
            analysis.task_targets,
            vec![Some("t1".into()), Some("t2".into())]
        );
        assert_eq!(analysis.plan.groups.len(), 1);
        let mut indices = analysis.plan.groups[0].task_indices.clone();
        indices.sort();
        assert_eq!(indices, vec![0, 1]);
    }

    #[test]
    fn test_full_spawn_analysis_with_dependency() {
        // x = f(), y = g(x) → 2 groups
        let body = crate::frontend::core::parser::ast::Block {
            stmts: vec![
                Stmt {
                    kind: StmtKind::Expr(Box::new(Expr::BinOp {
                        op: BinOp::Assign,
                        left: Box::new(Expr::Var("x".into(), dummy_span())),
                        right: Box::new(Expr::Call {
                            func: Box::new(Expr::Var("f".into(), dummy_span())),
                            args: vec![],
                            named_args: vec![],
                            span: dummy_span(),
                        }),
                        span: dummy_span(),
                    })),
                    span: dummy_span(),
                },
                Stmt {
                    kind: StmtKind::Expr(Box::new(Expr::BinOp {
                        op: BinOp::Assign,
                        left: Box::new(Expr::Var("y".into(), dummy_span())),
                        right: Box::new(Expr::Call {
                            func: Box::new(Expr::Var("g".into(), dummy_span())),
                            args: vec![Expr::Var("x".into(), dummy_span())],
                            named_args: vec![],
                            span: dummy_span(),
                        }),
                        span: dummy_span(),
                    })),
                    span: dummy_span(),
                },
            ],
            expr: None,
            span: dummy_span(),
        };
        let analysis = analyze_spawn_body(&body);
        assert_eq!(analysis.plan.groups.len(), 2);
        assert_eq!(analysis.plan.groups[0].task_indices, vec![0]);
        assert_eq!(analysis.plan.groups[1].task_indices, vec![1]);
    }
}
