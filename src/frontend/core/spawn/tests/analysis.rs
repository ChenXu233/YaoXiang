//! spawn 分析测试 — 基于 RFC-024 和 RFC-018
//!
//! RFC-024 §2.1: spawn 块的直接子表达式创建并行任务
//! RFC-024 §2.3: 编译期 DAG 分析，依赖驱动执行
//! RFC-024 §2.5: Resource 类型感知，同资源自动串行
//! RFC-024 §2.4: spawn for 数据并行
//! RFC-018 §5: spawn 块代码生成

use crate::frontend::core::parser::ast::{
    BinOp, Block, Expr, FStringSegment, MatchArm, Pattern, Stmt, StmtKind,
};
use crate::frontend::core::spawn::analysis::{
    analyze_reads_writes, analyze_spawn_body, analyze_spawn_for, build_execution_plan,
    is_direct_child,
};
use crate::frontend::core::types::{MonoType, TraitTable};
use crate::util::span::Span;
use std::collections::{HashMap, HashSet};

// ============================================================================
// 辅助函数
// ============================================================================

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
    }
}

fn call_expr(
    func: &str,
    args: Vec<Expr>,
) -> Expr {
    Expr::Call {
        func: Box::new(var_expr(func)),
        args,
        named_args: vec![],
        span: dummy_span(),
    }
}

fn resource_trait_table() -> (TraitTable, HashMap<String, MonoType>) {
    use crate::frontend::core::types::{TraitDefinition, TraitImplementation};
    let mut trait_table = TraitTable::default();
    trait_table.add_trait(TraitDefinition {
        name: "Resource".to_string(),
        methods: HashMap::new(),
        parent_traits: Vec::new(),
        generic_params: vec![],
        span: None,
        is_marker: true,
    });
    trait_table.add_impl(TraitImplementation {
        trait_name: "Resource".to_string(),
        for_type_name: "FilePath".to_string(),
        methods: HashMap::new(),
    });
    let mut local_var_types = HashMap::new();
    local_var_types.insert(
        "file".to_string(),
        MonoType::TypeRef("FilePath".to_string()),
    );
    (trait_table, local_var_types)
}

fn spawn_body(stmts: Vec<Stmt>) -> Block {
    Block {
        stmts,
        span: dummy_span(),
    }
}

// ============================================================================
// is_direct_child
// ============================================================================

#[test]
fn test_is_direct_child_with_expr_stmt() {
    // RFC-024 §2.1: 直接子表达式是顶层 StmtKind::Expr
    // Arrange
    let stmt = Stmt {
        kind: StmtKind::Expr(Box::new(var_expr("x"))),
        span: dummy_span(),
    };

    // Act
    let result = is_direct_child(&stmt);

    // Assert
    assert!(result, "StmtKind::Expr 应被视为直接子表达式");
}

#[test]
fn test_is_direct_child_with_var_decl_is_false() {
    // RFC-024 §2.1: 变量声明不是直接子表达式
    // Arrange
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

    // Act
    let result = is_direct_child(&stmt);

    // Assert
    assert!(!result, "StmtKind::Var 不应被视为直接子表达式");
}

// ============================================================================
// analyze_reads_writes — 基础
// ============================================================================

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

#[test]
fn test_reads_writes_assignment_target_and_rhs() {
    // Arrange
    let expr = Expr::BinOp {
        op: BinOp::Assign,
        left: Box::new(var_expr("t1")),
        right: Box::new(call_expr("fetch", vec![var_expr("a")])),
        span: dummy_span(),
    };

    // Act
    let (reads, writes, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(writes.contains("t1"), "赋值目标应被记入 writes");
    assert!(reads.contains("fetch"), "函数名应被记入 reads");
    assert!(reads.contains("a"), "函数参数应被记入 reads");
}

#[test]
fn test_reads_writes_call_args() {
    // Arrange
    let expr = call_expr("process", vec![var_expr("x"), var_expr("y")]);

    // Act
    let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(reads.contains("process"), "函数名应被记入 reads");
    assert!(reads.contains("x"), "第一个参数应被记入 reads");
    assert!(reads.contains("y"), "第二个参数应被记入 reads");
}

// ============================================================================
// analyze_reads_writes — 表达式覆盖
// ============================================================================

#[test]
fn test_reads_writes_list_elements() {
    // Arrange
    let expr = Expr::List(
        vec![var_expr("a"), var_expr("b"), var_expr("c")],
        dummy_span(),
    );

    // Act
    let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(reads.contains("a"), "列表元素 a 应被记入 reads");
    assert!(reads.contains("b"), "列表元素 b 应被记入 reads");
    assert!(reads.contains("c"), "列表元素 c 应被记入 reads");
}

#[test]
fn test_reads_writes_tuple_elements() {
    // Arrange
    let expr = Expr::Tuple(vec![var_expr("x"), var_expr("y")], dummy_span());

    // Act
    let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(reads.contains("x"), "元组元素 x 应被记入 reads");
    assert!(reads.contains("y"), "元组元素 y 应被记入 reads");
}

#[test]
fn test_reads_writes_dict_pairs() {
    // Arrange
    let expr = Expr::Dict(vec![(var_expr("k"), var_expr("v"))], dummy_span());

    // Act
    let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(reads.contains("k"), "字典键 k 应被记入 reads");
    assert!(reads.contains("v"), "字典值 v 应被记入 reads");
}

#[test]
fn test_reads_writes_index_access() {
    // Arrange
    let expr = Expr::Index {
        expr: Box::new(var_expr("arr")),
        index: Box::new(var_expr("i")),
        span: dummy_span(),
    };

    // Act
    let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(reads.contains("arr"), "索引目标 arr 应被记入 reads");
    assert!(reads.contains("i"), "索引值 i 应被记入 reads");
}

#[test]
fn test_reads_writes_cast_expr() {
    // Arrange
    let expr = Expr::Cast {
        expr: Box::new(var_expr("x")),
        target_type: crate::frontend::core::parser::ast::Type::Name {
            name: "Int".to_string(),
            span: dummy_span(),
        },
        span: dummy_span(),
    };

    // Act
    let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(reads.contains("x"), "类型转换表达式中的变量应被记入 reads");
}

#[test]
fn test_reads_writes_match_expr_and_arms() {
    // Arrange
    let expr = Expr::Match {
        expr: Box::new(var_expr("val")),
        arms: vec![MatchArm {
            pattern: Pattern::Wildcard,
            body: Block {
                stmts: vec![Stmt {
                    kind: StmtKind::Expr(Box::new(var_expr("result"))),
                    span: dummy_span(),
                }],
                span: dummy_span(),
            },
            span: dummy_span(),
        }],
        span: dummy_span(),
    };

    // Act
    let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(reads.contains("val"), "match scrutinee 应被记入 reads");
    assert!(
        reads.contains("result"),
        "match arm body 中的变量应被记入 reads"
    );
}

#[test]
fn test_reads_writes_if_expr_branches() {
    // Arrange
    let expr = Expr::If {
        condition: Box::new(var_expr("cond")),
        then_branch: Box::new(Block {
            stmts: vec![Stmt {
                kind: StmtKind::Expr(Box::new(var_expr("a"))),
                span: dummy_span(),
            }],
            span: dummy_span(),
        }),
        elif_branches: vec![],
        else_branch: Some(Box::new(Block {
            stmts: vec![Stmt {
                kind: StmtKind::Expr(Box::new(var_expr("b"))),
                span: dummy_span(),
            }],
            span: dummy_span(),
        })),
        span: dummy_span(),
    };

    // Act
    let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(reads.contains("cond"), "if 条件应被记入 reads");
    assert!(reads.contains("a"), "then 分支变量应被记入 reads");
    assert!(reads.contains("b"), "else 分支变量应被记入 reads");
}

#[test]
fn test_reads_writes_fstring_interpolation() {
    // Arrange
    let expr = Expr::FString {
        segments: vec![FStringSegment::Interpolation {
            expr: Box::new(var_expr("name")),
            format_spec: None,
        }],
        span: dummy_span(),
    };

    // Act
    let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

    // Assert
    assert!(reads.contains("name"), "f-string 插值变量应被记入 reads");
}

// ============================================================================
// build_execution_plan
// ============================================================================

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

#[test]
fn test_plan_read_after_write_creates_dependency() {
    // RFC-024 §2.3: 任务 i 读取任务 j 写入的变量 → i 依赖 j
    // Arrange
    let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
        (HashSet::new(), HashSet::from(["t1".into()])),
        (HashSet::from(["t1".into()]), HashSet::from(["t2".into()])),
    ];
    let resource_sets: Vec<HashSet<String>> = vec![HashSet::new(), HashSet::new()];

    // Act
    let plan = build_execution_plan(&sets, &resource_sets);

    // Assert
    assert_eq!(plan.groups.len(), 2, "读写依赖应产生两个串行组");
    assert_eq!(plan.groups[0].task_indices, vec![0], "第一个任务应在第一组");
    assert_eq!(plan.groups[1].task_indices, vec![1], "依赖任务应在第二组");
}

#[test]
fn test_plan_diamond_dependency_two_groups() {
    // RFC-024 §2.3: 菱形依赖 — x,y 独立并行，z 依赖两者
    // Arrange
    let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
        (HashSet::new(), HashSet::from(["x".into()])),
        (HashSet::new(), HashSet::from(["y".into()])),
        (
            HashSet::from(["x".into(), "y".into()]),
            HashSet::from(["z".into()]),
        ),
    ];
    let resource_sets: Vec<HashSet<String>> = vec![HashSet::new(), HashSet::new(), HashSet::new()];

    // Act
    let plan = build_execution_plan(&sets, &resource_sets);

    // Assert
    assert_eq!(plan.groups.len(), 2, "菱形依赖应产生两个组");
    assert_eq!(
        plan.groups[0].task_indices.len(),
        2,
        "独立任务应并行（第一组 2 个）"
    );
    assert_eq!(plan.groups[1].task_indices, vec![2], "依赖任务应在第二组");
}

#[test]
fn test_plan_empty_input_no_groups() {
    // Arrange
    let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![];
    let resource_sets: Vec<HashSet<String>> = vec![];

    // Act
    let plan = build_execution_plan(&sets, &resource_sets);

    // Assert
    assert_eq!(plan.groups.len(), 0, "空输入应产生空执行计划");
}

#[test]
fn test_plan_write_write_conflict_serializes() {
    // RFC-024 §2.3: 写后写冲突 → 串行
    // Arrange
    let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
        (HashSet::new(), HashSet::from(["x".into()])),
        (HashSet::new(), HashSet::from(["x".into()])),
    ];
    let resource_sets: Vec<HashSet<String>> = vec![HashSet::new(), HashSet::new()];

    // Act
    let plan = build_execution_plan(&sets, &resource_sets);

    // Assert
    assert_eq!(plan.groups.len(), 2, "写后写冲突应产生两个串行组");
}

#[test]
fn test_plan_resource_conflict_serializes() {
    // RFC-024 §2.5: 同一 Resource 变量被多个任务使用 → 串行
    // Arrange
    let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
        (HashSet::from(["file".into()]), HashSet::from(["a".into()])),
        (HashSet::from(["file".into()]), HashSet::from(["b".into()])),
    ];
    let resource_sets: Vec<HashSet<String>> = vec![
        HashSet::from(["file".into()]),
        HashSet::from(["file".into()]),
    ];

    // Act
    let plan = build_execution_plan(&sets, &resource_sets);

    // Assert
    assert_eq!(plan.groups.len(), 2, "Resource 冲突应产生两个串行组");
}

#[test]
fn test_plan_non_resource_read_read_allows_parallel() {
    // Arrange
    let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
        (HashSet::from(["x".into()]), HashSet::from(["a".into()])),
        (HashSet::from(["x".into()]), HashSet::from(["b".into()])),
    ];
    let resource_sets: Vec<HashSet<String>> = vec![HashSet::new(), HashSet::new()];

    // Act
    let plan = build_execution_plan(&sets, &resource_sets);

    // Assert
    assert_eq!(
        plan.groups.len(),
        1,
        "非 Resource 变量的读-读不冲突，应允许并行"
    );
}

// ============================================================================
// analyze_spawn_body 端到端
// ============================================================================

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

#[test]
fn test_spawn_body_with_read_after_write_dependency() {
    // RFC-024 §2.3: x = f(), y = g(x) → y 依赖 x
    // Arrange
    let body = spawn_body(vec![
        assign_stmt("x", call_expr("f", vec![])),
        assign_stmt("y", call_expr("g", vec![var_expr("x")])),
    ]);

    // Act
    let analysis = analyze_spawn_body(&body, &empty_trait_table(), &empty_var_types());

    // Assert
    assert_eq!(analysis.plan.groups.len(), 2, "读写依赖应产生两个串行组");
    assert_eq!(
        analysis.plan.groups[0].task_indices,
        vec![0],
        "第一个任务应在第一组"
    );
    assert_eq!(
        analysis.plan.groups[1].task_indices,
        vec![1],
        "依赖任务应在第二组"
    );
}

#[test]
fn test_spawn_body_resource_conflict_serializes() {
    // RFC-024 §2.5: 同一 Resource 类型变量 → 自动串行
    // Arrange
    let (trait_table, local_var_types) = resource_trait_table();
    let body = spawn_body(vec![
        assign_stmt("a", call_expr("read_file", vec![var_expr("file")])),
        assign_stmt("b", call_expr("stat_file", vec![var_expr("file")])),
    ]);

    // Act
    let analysis = analyze_spawn_body(&body, &trait_table, &local_var_types);

    // Assert
    assert_eq!(
        analysis.plan.groups.len(),
        2,
        "Resource 冲突应产生两个串行组"
    );
}

#[test]
fn test_spawn_body_task_info_exposes_reads_writes() {
    // Arrange
    let body = spawn_body(vec![assign_stmt("t1", call_expr("f", vec![var_expr("a")]))]);

    // Act
    let analysis = analyze_spawn_body(&body, &empty_trait_table(), &empty_var_types());

    // Assert
    let task = &analysis.tasks[0];
    assert!(task.reads.contains("f"), "任务 reads 应包含函数名 f");
    assert!(task.reads.contains("a"), "任务 reads 应包含参数 a");
    assert!(task.writes.contains("t1"), "任务 writes 应包含赋值目标 t1");
}

// ============================================================================
// analyze_spawn_for
// ============================================================================

#[test]
fn test_spawn_for_reads_writes() {
    // RFC-024 §2.4: spawn for 展开为 N 个任务
    // Arrange
    let body = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(call_expr("process", vec![var_expr("item")]))),
            span: dummy_span(),
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

// ============================================================================
// 补充边界测试
// ============================================================================

#[test]
fn test_spawn_body_mixed_children_and_non_children() {
    // RFC-024 §2.1: spawn 块内 var 声明不是直接子表达式，应被跳过
    // Arrange
    let body = Block {
        stmts: vec![
            // 非直接子表达式：var 声明
            Stmt {
                kind: StmtKind::Var {
                    name: "local_var".to_string(),
                    name_span: dummy_span(),
                    type_annotation: None,
                    initializer: Some(Box::new(Expr::Lit(
                        crate::frontend::core::lexer::tokens::Literal::Int(0),
                        dummy_span(),
                    ))),
                    is_mut: false,
                },
                span: dummy_span(),
            },
            // 直接子表达式：赋值
            assign_stmt("t1", call_expr("f", vec![])),
        ],
        span: dummy_span(),
    };

    // Act
    let analysis = analyze_spawn_body(&body, &empty_trait_table(), &empty_var_types());

    // Assert
    assert_eq!(
        analysis.tasks.len(),
        1,
        "var 声明应被跳过，只识别出 1 个任务"
    );
    assert_eq!(
        analysis.tasks[0].target.as_deref(),
        Some("t1"),
        "唯一任务目标应为 t1"
    );
}

#[test]
fn test_spawn_body_no_direct_children() {
    // RFC-024 §2.1: spawn 块内只有 var 声明，无直接子表达式
    // Arrange
    let body = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Var {
                name: "x".to_string(),
                name_span: dummy_span(),
                type_annotation: None,
                initializer: None,
                is_mut: false,
            },
            span: dummy_span(),
        }],
        span: dummy_span(),
    };

    // Act
    let analysis = analyze_spawn_body(&body, &empty_trait_table(), &empty_var_types());

    // Assert
    assert_eq!(analysis.tasks.len(), 0, "无直接子表达式时应产生 0 个任务");
    assert_eq!(analysis.plan.groups.len(), 0, "无任务时应产生空执行计划");
}

#[test]
fn test_spawn_body_write_write_conflict_serializes() {
    // RFC-024 §2.3: 两个任务写同一变量 → 写后写冲突 → 串行
    // Arrange
    let body = spawn_body(vec![
        assign_stmt("x", call_expr("f", vec![])),
        assign_stmt("x", call_expr("g", vec![])),
    ]);

    // Act
    let analysis = analyze_spawn_body(&body, &empty_trait_table(), &empty_var_types());

    // Assert
    assert_eq!(analysis.plan.groups.len(), 2, "写后写冲突应产生两个串行组");
}

#[test]
fn test_spawn_body_assignment_target_is_resource_type() {
    // RFC-024 §2.5: 赋值目标是 Resource 类型时，writes 中应包含该变量
    // Arrange
    let (trait_table, mut local_var_types) = resource_trait_table();
    local_var_types.insert("db".to_string(), MonoType::TypeRef("FilePath".to_string()));
    let body = spawn_body(vec![
        assign_stmt("db", call_expr("open_db", vec![])),
        assign_stmt("result", call_expr("query", vec![var_expr("db")])),
    ]);

    // Act
    let analysis = analyze_spawn_body(&body, &trait_table, &local_var_types);

    // Assert
    assert!(
        analysis.tasks[0].writes.contains("db"),
        "赋值目标 db 应被记入 writes"
    );
    assert!(
        analysis.tasks[0].resource_vars.contains("db"),
        "Resource 类型赋值目标 db 应被记入 resource_vars"
    );
    assert_eq!(
        analysis.plan.groups.len(),
        2,
        "Resource 类型赋值目标导致的冲突应产生两个串行组"
    );
}

#[test]
fn test_spawn_for_reads_outer_variable() {
    // RFC-024 §2.4: spawn for 循环体引用外部变量
    // Arrange
    let body = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(call_expr(
                "combine",
                vec![var_expr("item"), var_expr("prefix")],
            ))),
            span: dummy_span(),
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
    assert!(
        analysis.reads.contains("prefix"),
        "循环体 reads 应包含外部变量 prefix"
    );
    assert!(
        analysis.reads.contains("item"),
        "循环体 reads 应包含迭代变量 item"
    );
    assert!(
        analysis.reads.contains("combine"),
        "循环体 reads 应包含函数名 combine"
    );
}

// ============================================================================
// spawn for 端到端测试
// ============================================================================

#[test]
fn test_spawn_for_end_to_end_independent_iterations() {
    // RFC-024 §2.4: spawn for 数据并行，无依赖迭代应并行
    // Arrange
    let body = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(call_expr("process", vec![var_expr("item")]))),
            span: dummy_span(),
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
    assert_eq!(analysis.iter_var, "item");
    assert!(analysis.reads.contains("process"));
    assert!(analysis.reads.contains("item"));
    assert!(analysis.writes.is_empty(), "无写操作时应允许并行");
}
