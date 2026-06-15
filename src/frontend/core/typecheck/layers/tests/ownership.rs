//! 所有权检查测试 — 基于 RFC-009 + RFC-009a
//!
//! RFC-009  §2.7: 品牌机制
//! RFC-009a §品牌树: 令牌派生关系与冲突检测
//! RFC-009a §系统谓词清单: 5 种命题
//! RFC-009a §用例分析: 线性代码 / if-else / 循环

use crate::frontend::core::typecheck::layers::ownership::{
    BrandId, BrandTree, ControlFlowGraph, EdgeKind, FastPathResult, emit_move_predicate,
    emit_drop_predicate, emit_double_drop_predicate, emit_mut_predicate, fast_path_check,
};
use crate::frontend::core::typecheck::proof::verdict::{DisproofKind, ProofResult};
use crate::util::span::Span;

// ── BrandId 前缀匹配 ──────────────────────────────────

#[test]
fn test_prefix_matching() {
    // Arrange
    let root = BrandId::root(0);
    let field = root.derive_field("x");
    let deep = field.derive_field("y");

    // Act & Assert
    assert!(root.is_prefix_of(&field));
    assert!(root.is_prefix_of(&deep));
    assert!(field.is_prefix_of(&deep));
    assert!(!field.is_prefix_of(&root));
}

#[test]
fn test_different_roots_no_prefix_relation() {
    let a = BrandId::root(0);
    let b = BrandId::root(1);
    assert!(!a.is_prefix_of(&b));
    assert!(!b.is_prefix_of(&a));
}

#[test]
fn test_root_id_extraction() {
    assert_eq!(BrandId::root(42).root_id(), "#42");
    assert_eq!(BrandId::root(42).derive_field("x").root_id(), "#42");
    assert_eq!(
        BrandId::root(42)
            .derive_field("x")
            .derive_field("y")
            .root_id(),
        "#42"
    );
}

// ── 冲突判断（RFC-009a §品牌树） ──────────────────────

#[test]
fn test_read_vs_read_no_conflict() {
    let mut tree = BrandTree::new();
    let r1 = tree.create_read_token("x".into());
    let r2 = tree.create_read_token("x".into());
    assert!(!tree.conflicts(&r1, &r2));
}

#[test]
fn test_read_vs_write_conflict() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let w = tree.create_write_token("x".into());
    assert!(tree.conflicts(&r, &w));
}

#[test]
fn test_write_vs_write_conflict() {
    let mut tree = BrandTree::new();
    let w1 = tree.create_write_token("x".into());
    let w2 = tree.create_write_token("x".into());
    assert!(tree.conflicts(&w1, &w2));
}

#[test]
fn test_different_source_no_conflict() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let w = tree.create_write_token("y".into());
    assert!(!tree.conflicts(&r, &w));
}

#[test]
fn test_derived_read_vs_write_root_conflict() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let r_field = tree.derive_field(&r, "field").unwrap();
    let w = tree.create_write_token("x".into());
    // 同源 + 有写 = 冲突，与派生关系无关
    assert!(tree.conflicts(&r_field, &w));
}

#[test]
fn test_derived_read_vs_derived_read_no_conflict() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let rx = tree.derive_field(&r, "a").unwrap();
    let ry = tree.derive_field(&r, "b").unwrap();
    // 同源但都读 → 不冲突
    assert!(!tree.conflicts(&rx, &ry));
}

// ── 级联删除 ──────────────────────────────────────────

#[test]
fn test_remove_cascades_to_children() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let r_field = tree.derive_field(&r, "field").unwrap();
    assert!(tree.get(&r_field).is_some());

    tree.remove(&r);
    assert!(tree.get(&r).is_none());
    assert!(tree.get(&r_field).is_none());
}

#[test]
fn test_remove_cleans_up_parent_children_set() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let child = tree.derive_field(&r, "field").unwrap();
    assert!(tree.get(&r).unwrap().children.contains(&child));

    tree.remove(&child);
    assert!(!tree.get(&r).unwrap().children.contains(&child));
}

// ── 消费者追踪 ────────────────────────────────────────

#[test]
fn test_consumer_tracking() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    tree.add_consumer(&r, 3);
    tree.add_consumer(&r, 5);

    let c = tree.consumers(&r);
    assert!(c.contains(&3));
    assert!(c.contains(&5));
    assert_eq!(c.len(), 2);
}

#[test]
fn test_consumer_unknown_token_returns_empty() {
    let tree = BrandTree::new();
    assert!(tree.consumers(&BrandId::root(999)).is_empty());
}

// ── conflicting_with ──────────────────────────────────

#[test]
fn test_conflicting_with_returns_all_conflicts() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let _w1 = tree.create_write_token("x".into());
    let _w2 = tree.create_write_token("x".into());

    let conflicts = tree.conflicting_with(&r);
    assert_eq!(conflicts.len(), 2);
}

// ── 快速通道（RFC-009a §用例分析） ──────────────────────

#[test]
fn test_linear_code_read_then_write_no_conflict() {
    // Arrange: data = vec![...]; view = &data; x = view.total_count; data.push(4)
    let mut tree = BrandTree::new();
    let read = tree.create_read_token("data".into());
    tree.add_consumer(&read, 2);

    let mut cfg = ControlFlowGraph::new();
    for _ in 0..4 {
        cfg.add_node(None);
    }
    cfg.add_edge(0, 1, EdgeKind::Normal);
    cfg.add_edge(1, 2, EdgeKind::Normal);
    cfg.add_edge(2, 3, EdgeKind::Normal);

    // Act
    let write = tree.create_write_token("data".into());
    let result = fast_path_check(&tree, &cfg, &write, 3);

    // Assert: view 已在节点 2 被消费 → 写安全
    assert!(matches!(result, FastPathResult::Safe));
}

#[test]
fn test_read_and_write_conflict_when_consumer_not_executed() {
    // Arrange: 读令牌有消费者在节点 3，写操作在节点 2（消费者之前）
    let mut tree = BrandTree::new();
    let read = tree.create_read_token("data".into());
    tree.add_consumer(&read, 3);

    let mut cfg = ControlFlowGraph::new();
    for _ in 0..5 {
        cfg.add_node(None);
    }
    cfg.add_edge(0, 1, EdgeKind::Normal);
    cfg.add_edge(1, 2, EdgeKind::Normal);
    cfg.add_edge(2, 3, EdgeKind::Normal);
    cfg.add_edge(3, 4, EdgeKind::Normal);

    // Act
    let write = tree.create_write_token("data".into());
    let result = fast_path_check(&tree, &cfg, &write, 2);

    // Assert: 从消费者(节点3)反向 BFS 可达节点 2
    assert!(matches!(result, FastPathResult::Unsafe { .. }));
}

#[test]
fn test_loop_with_break_cuts_back_edge() {
    // Arrange: loop { use(view); if is_last { push; break } }
    let mut tree = BrandTree::new();
    let read = tree.create_read_token("data".into());
    tree.add_consumer(&read, 2);

    let mut cfg = ControlFlowGraph::new();
    for _ in 0..6 {
        cfg.add_node(None);
    }
    cfg.add_edge(0, 1, EdgeKind::Normal);
    cfg.add_edge(1, 2, EdgeKind::Normal);
    cfg.add_edge(2, 3, EdgeKind::Normal);
    cfg.add_edge(3, 4, EdgeKind::Normal);
    cfg.add_edge(4, 5, EdgeKind::Break);
    cfg.add_edge(3, 1, EdgeKind::BackEdge);

    // Act
    let write = tree.create_write_token("data".into());
    let result = fast_path_check(&tree, &cfg, &write, 4);

    // Assert: break 切断 → Safe
    assert!(matches!(result, FastPathResult::Safe));
}

#[test]
fn test_loop_without_break_is_unsafe() {
    // Arrange: loop { use(view); push } — 无 break
    let mut tree = BrandTree::new();
    let read = tree.create_read_token("data".into());
    tree.add_consumer(&read, 0);

    let mut cfg = ControlFlowGraph::new();
    for _ in 0..3 {
        cfg.add_node(None);
    }
    cfg.add_edge(0, 1, EdgeKind::Normal);
    cfg.add_edge(1, 2, EdgeKind::Normal);
    cfg.add_edge(2, 0, EdgeKind::BackEdge);

    // Act
    let write = tree.create_write_token("data".into());
    let result = fast_path_check(&tree, &cfg, &write, 1);

    // Assert: 回边穿越 → write_node 在 unsafe
    assert!(matches!(result, FastPathResult::Unsafe { .. }));
}

// ── 系统谓词（RFC-009a §系统谓词清单） ──────────────────

#[test]
fn test_use_after_move_rejected() {
    let result = emit_move_predicate("x", true, Span::dummy());
    assert!(matches!(result, ProofResult::Disproved { .. }));
}

#[test]
fn test_use_before_move_allowed() {
    let result = emit_move_predicate("x", false, Span::dummy());
    assert!(matches!(result, ProofResult::Proved));
}

#[test]
fn test_use_after_drop_rejected() {
    let result = emit_drop_predicate("x", true, Span::dummy());
    assert!(matches!(result, ProofResult::Disproved { .. }));
}

#[test]
fn test_double_drop_rejected() {
    let result = emit_double_drop_predicate("x", true, Span::dummy());
    assert!(matches!(result, ProofResult::Disproved { .. }));
}

#[test]
fn test_mut_violation_rejected() {
    let result = emit_mut_predicate("x", false, Span::dummy());
    assert!(matches!(result, ProofResult::Disproved { .. }));
}

#[test]
fn test_mut_allowed() {
    let result = emit_mut_predicate("x", true, Span::dummy());
    assert!(matches!(result, ProofResult::Proved));
}

// ── E2E 集成测试（RFC-009a §用例分析） ──────────────────

use crate::frontend::core::parser::ast::{BinOp, Block, Expr, Literal, Module, Param, Stmt, StmtKind};
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::typecheck::layers::ownership::OwnershipChecker;

fn make_var(name: &str) -> Expr {
    Expr::Var(name.into(), Span::default())
}

fn make_lit(n: i64) -> Expr {
    Expr::Lit(Literal::Int(n as i128), Span::default())
}

fn make_var_stmt(
    name: &str,
    init: Expr,
) -> Stmt {
    Stmt {
        kind: StmtKind::Var {
            name: name.into(),
            name_span: Span::default(),
            type_annotation: None,
            initializer: Some(Box::new(init)),
            is_mut: false,
        },
        span: Span::default(),
    }
}

fn make_mut_var_stmt(
    name: &str,
    init: Expr,
) -> Stmt {
    Stmt {
        kind: StmtKind::Var {
            name: name.into(),
            name_span: Span::default(),
            type_annotation: None,
            initializer: Some(Box::new(init)),
            is_mut: true,
        },
        span: Span::default(),
    }
}

fn make_expr_stmt(expr: Expr) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(expr)),
        span: Span::default(),
    }
}

fn make_binding(
    name: &str,
    params: Vec<String>,
    body: Vec<Stmt>,
) -> Stmt {
    Stmt {
        kind: StmtKind::Binding {
            name: name.into(),
            type_name: None,
            method_type: None,
            generic_params: vec![],
            type_annotation: None,
            params: params
                .into_iter()
                .map(|n| Param {
                    name: n,
                    ty: None,
                    is_mut: false,
                    span: Span::default(),
                })
                .collect(),
            body,
            is_pub: false,
        },
        span: Span::default(),
    }
}

fn make_module(items: Vec<Stmt>) -> Module {
    Module {
        items,
        span: Span::default(),
    }
}

fn make_test_env() -> TypeEnvironment {
    TypeEnvironment::new_with_module("test".into())
}

fn make_block(stmts: Vec<Stmt>) -> Block {
    Block {
        stmts,
        span: Span::default(),
    }
}

fn make_call(
    func_name: &str,
    args: Vec<Expr>,
) -> Expr {
    Expr::Call {
        func: Box::new(Expr::Var(func_name.into(), Span::default())),
        args,
        named_args: vec![],
        span: Span::default(),
    }
}

fn make_bool_lit(b: bool) -> Expr {
    Expr::Lit(Literal::Bool(b), Span::default())
}

#[test]
fn test_e2e_use_after_move_detected() {
    // Arrange: { x = 42; y = x; use(x) }
    // x 被 move 给 y 后再次使用 → 应报 Disproved
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_var_stmt("y", make_var("x")),
            make_expr_stmt(make_var("x")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(!errors.is_empty(), "应该检测到 use after move，但结果为空");
}

#[test]
fn test_e2e_valid_move_no_error() {
    // Arrange: { x = 42; y = x }
    // x 被 move 给 y 后不再使用 → 不应有错误
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_var_stmt("y", make_var("x")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(errors.is_empty(), "不应有错误，得: {:?}", errors);
}

#[test]
fn test_e2e_argument_passed_to_function_is_moved() {
    // Arrange: { x = 42; f(x); use(x) }
    // x 作为参数传给 f 后不能再使用
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_expr_stmt(Expr::Call {
                func: Box::new(make_var("f")),
                args: vec![make_var("x")],
                named_args: vec![],
                span: Span::default(),
            }),
            make_expr_stmt(make_var("x")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(!errors.is_empty(), "应该检测到 x 被 move 进 f 后再使用");
}

// ── E2E 借用冲突测试 ────────────────────────────────

#[test]
fn test_e2e_borrow_conflict_detected() {
    // Arrange: { mut x = 42; y = &x; z = &mut x }
    // &x 创建 ReadToken(x)，&mut x 创建 WriteToken(x)
    // add_consumer_for_var("x") 在 &mut x 时为 ReadToken 添加消费者 → 反向 BFS 可达
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_mut_var_stmt("x", make_lit(42)),
            make_var_stmt(
                "y",
                Expr::Borrow {
                    mutable: false,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
            make_var_stmt(
                "z",
                Expr::Borrow {
                    mutable: true,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let borrow_errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::BorrowConflict))
        })
        .collect();
    assert!(
        !borrow_errors.is_empty(),
        "应该检测到 &x 和 &mut x 的借用冲突，但结果为空"
    );
}

#[test]
fn test_e2e_write_write_conflict_detected() {
    // Arrange: { mut x = 42; a = &mut x; b = &mut x }
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_mut_var_stmt("x", make_lit(42)),
            make_var_stmt(
                "a",
                Expr::Borrow {
                    mutable: true,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
            make_var_stmt(
                "b",
                Expr::Borrow {
                    mutable: true,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let borrow_errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::BorrowConflict))
        })
        .collect();
    assert!(
        !borrow_errors.is_empty(),
        "应该检测到 &mut x 和 &mut x 的借用冲突"
    );
}

#[test]
fn test_e2e_read_read_no_conflict() {
    // Arrange: { x = 42; a = &x; b = &x }
    // 两个 ReadToken 不冲突
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_var_stmt(
                "a",
                Expr::Borrow {
                    mutable: false,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
            make_var_stmt(
                "b",
                Expr::Borrow {
                    mutable: false,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(
        errors.is_empty(),
        "两个 &x 不应冲突，但检测到错误: {:?}",
        errors
    );
}

// ── E2E 可变性检查测试 ──────────────────────────────────

#[test]
fn test_e2e_mut_borrow_on_non_mut_var() {
    // Arrange: { x = 42; y = &mut x }
    // x 未声明 mut → &mut 应报 mut_violation
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_var_stmt(
                "y",
                Expr::Borrow {
                    mutable: true,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let mut_errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::MutViolation))
        })
        .collect();
    assert!(
        !mut_errors.is_empty(),
        "应该检测到 &mut x 的可变性违规，但结果为空"
    );
}

#[test]
fn test_e2e_mut_borrow_on_mut_var() {
    // Arrange: { mut x = 42; y = &mut x }
    // x 声明为 mut → &mut 不应报错
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_mut_var_stmt("x", make_lit(42)),
            make_var_stmt(
                "y",
                Expr::Borrow {
                    mutable: true,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(
        errors.is_empty(),
        "mut x 的 &mut 不应报错，但检测到: {:?}",
        errors
    );
}

#[test]
fn test_e2e_assign_to_non_mut_var() {
    // Arrange: { x = 42; x = 43 }
    // x 未声明 mut → 赋值应报 mut_violation
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_expr_stmt(Expr::BinOp {
                op: BinOp::Assign,
                left: Box::new(make_var("x")),
                right: Box::new(make_lit(43)),
                span: Span::default(),
            }),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let mut_errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::MutViolation))
        })
        .collect();
    assert!(
        !mut_errors.is_empty(),
        "应该检测到 x = 43 的可变性违规（x 非 mut），但结果为空"
    );
}

#[test]
fn test_e2e_assign_to_mut_var() {
    // Arrange: { mut x = 42; x = 43 }
    // x 声明为 mut → 赋值不应报错
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_mut_var_stmt("x", make_lit(42)),
            make_expr_stmt(Expr::BinOp {
                op: BinOp::Assign,
                left: Box::new(make_var("x")),
                right: Box::new(make_lit(43)),
                span: Span::default(),
            }),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(
        errors.is_empty(),
        "mut x 的赋值不应报错，但检测到: {:?}",
        errors
    );
}

#[test]
fn test_e2e_non_mut_param_borrow_mut() {
    // Arrange: fn f(x: i32) { &mut x }
    // x 参数未声明 mut → &mut 应报 mut_violation
    let module = make_module(vec![make_binding(
        "f",
        vec!["x".into()],
        vec![make_var_stmt(
            "y",
            Expr::Borrow {
                mutable: true,
                expr: Box::new(make_var("x")),
                span: Span::default(),
            },
        )],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let mut_errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::MutViolation))
        })
        .collect();
    assert!(
        !mut_errors.is_empty(),
        "应该检测到对非 mut 参数 x 的 &mut，但结果为空"
    );
}

// ── E2E Drop 语义测试 ──────────────────────────────────

#[test]
fn test_drop_at_scope_exit_via_release_plan() {
    // Arrange: { x = 42; use(x); }
    // x 应在作用域结束时被 Drop
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_expr_stmt(make_var("x")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(errors.is_empty(), "不应有错误，得: {:?}", errors);
    // x 应出现在 ReleasePlan 中
    let dropped_vars: Vec<&String> = plan.drops.values().flatten().collect();
    assert!(
        dropped_vars.iter().any(|v| v.as_str() == "x"),
        "x 应该被释放，plan: {:?}",
        plan
    );
}

#[test]
fn test_drop_in_nested_block() {
    // Arrange: { { let x = 42; use(x); } let y = 1; use(y); }
    // x 在内层作用域 Drop，y 在外层作用域 Drop
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_expr_stmt(Expr::Block(Block {
                stmts: vec![
                    make_var_stmt("x", make_lit(42)),
                    make_expr_stmt(make_var("x")),
                ],
                span: Span::default(),
            })),
            make_var_stmt("y", make_lit(1)),
            make_expr_stmt(make_var("y")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(errors.is_empty(), "不应有错误，得: {:?}", errors);
    let dropped_vars: Vec<&String> = plan.drops.values().flatten().collect();
    assert!(
        dropped_vars.iter().any(|v| v.as_str() == "x"),
        "内层 x 应该被释放，plan: {:?}",
        plan
    );
    assert!(
        dropped_vars.iter().any(|v| v.as_str() == "y"),
        "外层 y 应该被释放，plan: {:?}",
        plan
    );
}

// ── E2E Move + Borrow 交互 ───────────────────────────────

#[test]
fn test_e2e_move_then_borrow_rejected() {
    // Arrange: { x = 42; y = x; z = &x }
    // x 被 move 给 y 后，不能再被借用
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_var_stmt("y", make_var("x")),
            make_var_stmt(
                "z",
                Expr::Borrow {
                    mutable: false,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert: x 已被 move，再借用应报 use after move
    let errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::UseAfterMove))
        })
        .collect();
    assert!(!errors.is_empty(), "应该检测到 move 后 borrow x 的错误");
}

// ── E2E 控制流借用 ──────────────────────────────────────

#[test]
fn test_e2e_borrow_in_if_both_branches() {
    // Arrange: { mut x = 42; if cond { &mut x } else { &mut x }; use(x) }
    // 注：当前单趟遍历不建模分支互斥性，两个 &mut x 会被保守地报冲突。
    // 这是已知限制（NLL without fixpoint），不是 bug。
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_mut_var_stmt("x", make_lit(42)),
            make_expr_stmt(Expr::If {
                condition: Box::new(make_bool_lit(true)),
                then_branch: Box::new(make_block(vec![make_var_stmt(
                    "y",
                    Expr::Borrow {
                        mutable: true,
                        expr: Box::new(make_var("x")),
                        span: Span::default(),
                    },
                )])),
                elif_branches: vec![],
                else_branch: Some(Box::new(make_block(vec![make_var_stmt(
                    "z",
                    Expr::Borrow {
                        mutable: true,
                        expr: Box::new(make_var("x")),
                        span: Span::default(),
                    },
                )]))),
                span: Span::default(),
            }),
            make_expr_stmt(make_var("x")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert: 保守检测到冲突（已知限制）
    let borrow_errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::BorrowConflict))
        })
        .collect();
    assert!(
        !borrow_errors.is_empty(),
        "保守策略应检测到 if/else 双分支 &mut x 的潜在冲突"
    );
}

#[test]
fn test_e2e_borrow_in_while_body() {
    // Arrange: while 体内借用，每次迭代新作用域
    // { mut x = 42; mut i = 0; while i < 3 { let y = &mut x; use(y); i = i + 1 } }
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_mut_var_stmt("x", make_lit(42)),
            make_mut_var_stmt("i", make_lit(0)),
            make_expr_stmt(Expr::While {
                condition: Box::new(Expr::BinOp {
                    op: BinOp::Lt,
                    left: Box::new(make_var("i")),
                    right: Box::new(make_lit(3)),
                    span: Span::default(),
                }),
                body: Box::new(make_block(vec![
                    make_var_stmt(
                        "y",
                        Expr::Borrow {
                            mutable: true,
                            expr: Box::new(make_var("x")),
                            span: Span::default(),
                        },
                    ),
                    make_expr_stmt(make_var("y")),
                    make_expr_stmt(Expr::BinOp {
                        op: BinOp::Assign,
                        left: Box::new(make_var("i")),
                        right: Box::new(Expr::BinOp {
                            op: BinOp::Add,
                            left: Box::new(make_var("i")),
                            right: Box::new(make_lit(1)),
                            span: Span::default(),
                        }),
                        span: Span::default(),
                    }),
                ])),
                label: None,
                span: Span::default(),
            }),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert: 每次迭代新作用域，借用不应冲突
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(
        errors.is_empty(),
        "while 循环体内 &mut x 不应冲突（每次新作用域），但检测到: {:?}",
        errors
    );
}

// ── E2E Drop 排序 ────────────────────────────────────────

#[test]
fn test_e2e_drop_release_plan_multiple_vars() {
    // Arrange: { x = 1; y = 2; use(x); use(y) }
    // x 和 y 都应在同一个 span 释放
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(1)),
            make_var_stmt("y", make_lit(2)),
            make_expr_stmt(make_var("x")),
            make_expr_stmt(make_var("y")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(errors.is_empty(), "不应有错误，得: {:?}", errors);
    let dropped_vars: Vec<&String> = plan.drops.values().flatten().collect();
    assert!(
        dropped_vars.iter().any(|v| v.as_str() == "x"),
        "x 应该被释放"
    );
    assert!(
        dropped_vars.iter().any(|v| v.as_str() == "y"),
        "y 应该被释放"
    );
}

// ── E2E 返回值 Move ──────────────────────────────────────

#[test]
fn test_e2e_return_moved_value() {
    // Arrange: fn f() { x = 42; return x }
    // x 被 return 移出，不应报错
    let module = make_module(vec![make_binding(
        "f",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            Stmt {
                kind: StmtKind::Return(Some(Box::new(make_var("x")))),
                span: Span::default(),
            },
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(
        errors.is_empty(),
        "return x 不应报错（正常所有权转移），但检测到: {:?}",
        errors
    );
}

#[test]
fn test_e2e_use_after_return_rejected() {
    // Arrange: fn f() { x = 42; return x; use(x) }
    // x 被 return 移走后不能再使用
    let module = make_module(vec![make_binding(
        "f",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            Stmt {
                kind: StmtKind::Return(Some(Box::new(make_var("x")))),
                span: Span::default(),
            },
            make_expr_stmt(make_var("x")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::UseAfterMove))
        })
        .collect();
    assert!(!errors.is_empty(), "应该检测到 return 后使用 x 的错误");
}

// ── E2E 多重借用 ──────────────────────────────────────────

#[test]
fn test_e2e_three_read_borrows_no_conflict() {
    // Arrange: { x = 42; a = &x; b = &x; c = &x }
    // 三个 ReadToken 不冲突
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_var_stmt(
                "a",
                Expr::Borrow {
                    mutable: false,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
            make_var_stmt(
                "b",
                Expr::Borrow {
                    mutable: false,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
            make_var_stmt(
                "c",
                Expr::Borrow {
                    mutable: false,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(
        errors.is_empty(),
        "三个 &x 不应冲突，但检测到: {:?}",
        errors
    );
}

#[test]
fn test_e2e_read_then_write_conflict() {
    // Arrange: { mut x = 42; a = &x; b = &mut x }
    // ReadToken 之后 WriteToken → 冲突
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_mut_var_stmt("x", make_lit(42)),
            make_var_stmt(
                "a",
                Expr::Borrow {
                    mutable: false,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
            make_var_stmt(
                "b",
                Expr::Borrow {
                    mutable: true,
                    expr: Box::new(make_var("x")),
                    span: Span::default(),
                },
            ),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let borrow_errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::BorrowConflict))
        })
        .collect();
    assert!(
        !borrow_errors.is_empty(),
        "应该检测到 &x 和 &mut x 的借用冲突"
    );
}

// ── E2E 块表达式 ──────────────────────────────────────────

#[test]
fn test_e2e_block_expression_variable_scope() {
    // Arrange: { { let x = 1; use(x); }; let y = 2; use(y); }
    // x 在内层块释放，y 在外层释放
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_expr_stmt(Expr::Block(make_block(vec![
                make_var_stmt("x", make_lit(1)),
                make_expr_stmt(make_var("x")),
            ]))),
            make_var_stmt("y", make_lit(2)),
            make_expr_stmt(make_var("y")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(errors.is_empty(), "不应有错误，得: {:?}", errors);
    let dropped: Vec<&String> = plan.drops.values().flatten().collect();
    assert!(dropped.iter().any(|v| v.as_str() == "x"), "x 应该被释放");
    assert!(dropped.iter().any(|v| v.as_str() == "y"), "y 应该被释放");
}

// ── E2E 连续 Move ────────────────────────────────────────

#[test]
fn test_e2e_sequential_moves() {
    // Arrange: { x = 42; y = x; z = y }
    // x → y, y → z，连续 Move，最终只有 z 可用
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_var_stmt("y", make_var("x")),
            make_var_stmt("z", make_var("y")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert: 无错误（x→y→z，每次都是正常 Move）
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(
        errors.is_empty(),
        "x→y→z 连续 Move 不应报错，但检测到: {:?}",
        errors
    );
}

#[test]
fn test_e2e_double_move_rejected() {
    // Arrange: { x = 42; y = x; z = x }
    // x 被 move 给 y 后，不能再 move 给 z
    let module = make_module(vec![make_binding(
        "main",
        vec![],
        vec![
            make_var_stmt("x", make_lit(42)),
            make_var_stmt("y", make_var("x")),
            make_var_stmt("z", make_var("x")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::UseAfterMove))
        })
        .collect();
    assert!(
        !errors.is_empty(),
        "应该检测到 x 被 move 两次（use after move）"
    );
}

// ── E2E 参数所有权 ────────────────────────────────────────

#[test]
fn test_e2e_param_move_and_use_rejected() {
    // Arrange: fn f(x: i32) { y = x; use(x) }
    // x 是参数，被 move 给 y 后不能使用
    let module = make_module(vec![make_binding(
        "f",
        vec!["x".into()],
        vec![
            make_var_stmt("y", make_var("x")),
            make_expr_stmt(make_var("x")),
        ],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, _plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| {
            matches!(r, ProofResult::Disproved(model)
                if matches!(model.kind, DisproofKind::UseAfterMove))
        })
        .collect();
    assert!(!errors.is_empty(), "应该检测到参数 x 被 move 后使用");
}

#[test]
fn test_e2e_param_not_in_release_plan() {
    // Arrange: fn f(x: i32) { use(x) }
    // 参数不应出现在 ReleasePlan 中（由调用方负责释放）
    let module = make_module(vec![make_binding(
        "f",
        vec!["x".into()],
        vec![make_expr_stmt(make_var("x"))],
    )]);

    // Act
    let mut checker = OwnershipChecker::new();
    let (results, plan) = checker.check_module(&module, &make_test_env());

    // Assert
    let errors: Vec<_> = results
        .iter()
        .filter(|r| matches!(r, ProofResult::Disproved { .. }))
        .collect();
    assert!(errors.is_empty(), "不应有错误，得: {:?}", errors);
    let dropped: Vec<&String> = plan.drops.values().flatten().collect();
    assert!(
        !dropped.iter().any(|v| v.as_str() == "x"),
        "参数 x 不应出现在 ReleasePlan 中（调用方负责释放），plan: {:?}",
        plan
    );
}
