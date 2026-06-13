//! 终止检查器单元测试
//!
//! RFC-027 Section 7: Termination Checker Tests

use crate::frontend::core::typecheck::layers::termination::TerminationChecker;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::parser::ast::{BinOp, Block, Expr, Literal, Stmt, StmtKind};
use crate::util::span::Span;

// ==================== 测试辅助函数 ====================

fn dummy_span() -> Span {
    Span::default()
}

/// 创建 `i += delta` 表达式 (`i = i + delta`)
fn make_increment(
    var: &str,
    delta: i128,
) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(Expr::Var(var.to_string(), dummy_span())),
            right: Box::new(Expr::BinOp {
                op: BinOp::Add,
                left: Box::new(Expr::Var(var.to_string(), dummy_span())),
                right: Box::new(Expr::Lit(Literal::Int(delta), dummy_span())),
                span: dummy_span(),
            }),
            span: dummy_span(),
        })),
        span: dummy_span(),
    }
}

/// 创建 `i -= delta` 表达式
fn make_decrement(
    var: &str,
    delta: i128,
) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(Expr::Var(var.to_string(), dummy_span())),
            right: Box::new(Expr::BinOp {
                op: BinOp::Sub,
                left: Box::new(Expr::Var(var.to_string(), dummy_span())),
                right: Box::new(Expr::Lit(Literal::Int(delta), dummy_span())),
                span: dummy_span(),
            }),
            span: dummy_span(),
        })),
        span: dummy_span(),
    }
}

/// 创建 `i < n` 条件
fn make_lt_condition(
    var: &str,
    bound: &str,
) -> Box<Expr> {
    Box::new(Expr::BinOp {
        op: BinOp::Lt,
        left: Box::new(Expr::Var(var.to_string(), dummy_span())),
        right: Box::new(Expr::Var(bound.to_string(), dummy_span())),
        span: dummy_span(),
    })
}

/// 创建 `i > 0` 条件
fn make_gt_condition(
    var: &str,
    bound: i128,
) -> Box<Expr> {
    Box::new(Expr::BinOp {
        op: BinOp::Gt,
        left: Box::new(Expr::Var(var.to_string(), dummy_span())),
        right: Box::new(Expr::Lit(Literal::Int(bound), dummy_span())),
        span: dummy_span(),
    })
}

/// 创建 `while cond { body }` 表达式
fn make_while(
    condition: Box<Expr>,
    body_stmts: Vec<Stmt>,
) -> Box<Expr> {
    Box::new(Expr::While {
        condition,
        body: Box::new(Block {
            stmts: body_stmts,
            span: dummy_span(),
        }),
        label: None,
        span: dummy_span(),
    })
}

/// 创建 for 循环表达式
fn make_for(
    var: &str,
    iterable: Box<Expr>,
    body_stmts: Vec<Stmt>,
) -> Box<Expr> {
    Box::new(Expr::For {
        var: var.to_string(),
        var_mut: false,
        iterable,
        body: Box::new(Block {
            stmts: body_stmts,
            span: dummy_span(),
        }),
        label: None,
        span: dummy_span(),
    })
}

/// 运行终止检查器
fn run_check(expr: &Expr) -> Vec<ProofResult> {
    // 将表达式包装为模块语句
    let stmt = Stmt {
        kind: StmtKind::Expr(Box::new(expr.clone())),
        span: dummy_span(),
    };
    let module = crate::frontend::core::parser::ast::Module {
        items: vec![stmt],
        span: dummy_span(),
    };
    let env = crate::frontend::core::typecheck::environment::TypeEnvironment::new();
    let mut checker = TerminationChecker::new();
    checker.check_module(&module, &env)
}

// ==================== 测试：循环终止 ====================

#[test]
fn test_while_increment_to_bound_terminates() {
    // while i < n { i += 1 }
    let while_expr = make_while(make_lt_condition("i", "n"), vec![make_increment("i", 1)]);

    let results = run_check(&while_expr);
    // 所有结果应该都是 Proved（或者空——没有 while 循环外的其他检查）
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "Expected all Proved results, got unproven: {:?}",
        unproven
    );
}

#[test]
fn test_while_decrement_to_lower_bound_terminates() {
    // while i > 0 { i -= 1 }
    let while_expr = make_while(make_gt_condition("i", 0), vec![make_decrement("i", 1)]);

    let results = run_check(&while_expr);
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "Expected all Proved results, got unproven: {:?}",
        unproven
    );
}

#[test]
fn test_while_constant_condition_fails() {
    // while true { x = 1 } — 没有递减的循环变量
    let body_stmt = Box::new(Stmt {
        kind: StmtKind::Expr(Box::new(Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(Expr::Var("x".to_string(), dummy_span())),
            right: Box::new(Expr::Lit(Literal::Int(1), dummy_span())),
            span: dummy_span(),
        })),
        span: dummy_span(),
    });
    let while_expr = Box::new(Expr::While {
        condition: Box::new(Expr::Lit(Literal::Bool(true), dummy_span())),
        body: Box::new(Block {
            stmts: vec![*body_stmt],
            span: dummy_span(),
        }),
        label: None,
        span: dummy_span(),
    });

    let results = run_check(&while_expr);
    assert!(
        !results.is_empty(),
        "Expected unproven result for non-terminating loop"
    );
    assert!(
        matches!(&results[0], ProofResult::Unproven { .. }),
        "Expected Unproven, got: {:?}",
        results[0]
    );
}

#[test]
fn test_while_no_assignment_fails() {
    // while i < n { print(i) } — 循环体内没有修改 i
    let body_stmt = Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Var("i".to_string(), dummy_span()))),
        span: dummy_span(),
    };
    let while_expr = make_while(make_lt_condition("i", "n"), vec![body_stmt]);

    let results = run_check(&while_expr);
    assert!(
        !results.is_empty(),
        "Expected unproven result for loop with no progress"
    );
    assert!(
        matches!(&results[0], ProofResult::Unproven { .. }),
        "Expected Unproven, got: {:?}",
        results[0]
    );
}

#[test]
fn test_while_decrement_wrong_direction_fails() {
    // while i < n { i -= 1 } — i 递减但上界是 n，方向错误
    let while_expr = make_while(make_lt_condition("i", "n"), vec![make_decrement("i", 1)]);

    let results = run_check(&while_expr);
    assert!(
        !results.is_empty(),
        "Expected unproven result: i decreases when it should increase toward bound"
    );
    assert!(
        matches!(&results[0], ProofResult::Unproven { .. }),
        "Expected Unproven, got: {:?}",
        results[0]
    );
}

#[test]
fn test_while_increment_by_two_terminates() {
    // while i < n { i += 2 }
    let while_expr = make_while(make_lt_condition("i", "n"), vec![make_increment("i", 2)]);

    let results = run_check(&while_expr);
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "Expected all Proved results for i += 2, got unproven: {:?}",
        unproven
    );
}

// ==================== 测试：for 循环 ====================

#[test]
fn test_for_loop_trivially_terminates() {
    // for x in range { print(x) }
    let body_stmt = Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Var("x".to_string(), dummy_span()))),
        span: dummy_span(),
    };
    let for_expr = make_for(
        "x",
        Box::new(Expr::Var("range".to_string(), dummy_span())),
        vec![body_stmt],
    );

    let results = run_check(&for_expr);
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "Expected all Proved results for for-loop, got unproven: {:?}",
        unproven
    );
}

// ==================== 测试：嵌套循环 ====================

#[test]
fn test_nested_while_both_terminating() {
    // 外层 while i < n { i += 1 }
    let inner_while = make_while(make_lt_condition("j", "m"), vec![make_increment("j", 1)]);
    let inner_stmt = Stmt {
        kind: StmtKind::Expr(inner_while),
        span: dummy_span(),
    };
    let body_stmts = vec![make_increment("i", 1), inner_stmt];
    let outer_while = make_while(make_lt_condition("i", "n"), body_stmts);

    let results = run_check(&outer_while);
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        unproven.is_empty(),
        "Expected all Proved results for nested terminating loops, got unproven: {:?}",
        unproven
    );
}

#[test]
fn test_nested_while_inner_fails() {
    // 外层终止，内层不终止
    let inner_while = Box::new(Expr::While {
        condition: Box::new(Expr::Lit(Literal::Bool(true), dummy_span())),
        body: Box::new(Block {
            stmts: vec![],
            span: dummy_span(),
        }),
        label: None,
        span: dummy_span(),
    });
    let inner_stmt = Stmt {
        kind: StmtKind::Expr(inner_while),
        span: dummy_span(),
    };
    let outer_while = make_while(
        make_lt_condition("i", "n"),
        vec![make_increment("i", 1), inner_stmt],
    );

    let results = run_check(&outer_while);
    // 外层终止(Proved)但内层不终止(Unproven) → 至少 1 个 Unproven
    let unproven: Vec<_> = results.iter().filter(|r| !r.is_proved()).collect();
    assert!(
        !unproven.is_empty(),
        "Expected at least one Unproven result for inner non-terminating loop"
    );
}
