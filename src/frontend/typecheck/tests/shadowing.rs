//! 变量遮蔽检查和作用域管理测试
//!
//! 测试 BodyChecker 的作用域栈、遮蔽检查和变量销毁

use std::collections::HashMap;
use crate::frontend::core::parser::ast;
use crate::frontend::typecheck::inference::ExprInferrer;
use crate::frontend::typecheck::overload;
use crate::frontend::core::type_system::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::typecheck::checking::BodyChecker;
use crate::frontend::typecheck::TypeChecker;
use crate::util::span::{Position, Span};

fn create_dummy_span() -> Span {
    Span::new(Position::dummy(), Position::dummy())
}

// ============================================================
// BodyChecker 作用域栈基础测试
// ============================================================

#[test]
fn test_body_checker_scope_basic() {
    let mut solver = TypeConstraintSolver::new();
    let mut checker = BodyChecker::new(&mut solver);

    // 全局作用域添加变量
    checker.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));
    assert!(checker.var_exists_in_any_scope("x"));
    assert!(checker.var_exists_in_current_scope("x"));

    // 进入新作用域
    checker.enter_scope();
    checker.add_var("y".to_string(), PolyType::mono(MonoType::String));

    // x 可见（外层），y 可见（当前层）
    assert!(checker.var_exists_in_any_scope("x"));
    assert!(checker.var_exists_in_any_scope("y"));
    assert!(!checker.var_exists_in_current_scope("x")); // x 不在当前作用域
    assert!(checker.var_exists_in_current_scope("y"));

    // 退出作用域
    checker.exit_scope();

    // x 存在，y 已销毁
    assert!(checker.var_exists_in_any_scope("x"));
    assert!(!checker.var_exists_in_any_scope("y"));
}

#[test]
fn test_body_checker_nested_scopes() {
    let mut solver = TypeConstraintSolver::new();
    let mut checker = BodyChecker::new(&mut solver);

    checker.add_var("a".to_string(), PolyType::mono(MonoType::Int(64)));

    checker.enter_scope();
    checker.add_var("b".to_string(), PolyType::mono(MonoType::Float(64)));

    checker.enter_scope();
    checker.add_var("c".to_string(), PolyType::mono(MonoType::Bool));

    // 所有变量可见
    assert!(checker.get_var("a").is_some());
    assert!(checker.get_var("b").is_some());
    assert!(checker.get_var("c").is_some());

    checker.exit_scope();
    assert!(checker.get_var("c").is_none()); // c 已销毁
    assert!(checker.get_var("b").is_some());

    checker.exit_scope();
    assert!(checker.get_var("b").is_none()); // b 已销毁
    assert!(checker.get_var("a").is_some());
}

#[test]
fn test_body_checker_get_var_finds_innermost() {
    let mut solver = TypeConstraintSolver::new();
    let mut checker = BodyChecker::new(&mut solver);

    // 全局：x = Int
    checker.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));

    checker.enter_scope();
    // 内层：x = String（覆盖外层）
    checker.add_var("x".to_string(), PolyType::mono(MonoType::String));

    let poly = checker.get_var("x").unwrap();
    assert!(matches!(poly.body, MonoType::String));

    checker.exit_scope();

    let poly = checker.get_var("x").unwrap();
    assert!(matches!(poly.body, MonoType::Int(64)));
}

#[test]
fn test_body_checker_vars_returns_all() {
    let mut solver = TypeConstraintSolver::new();
    let mut checker = BodyChecker::new(&mut solver);

    checker.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));

    checker.enter_scope();
    checker.add_var("y".to_string(), PolyType::mono(MonoType::String));

    let all = checker.vars();
    assert!(all.contains_key("x"));
    assert!(all.contains_key("y"));
    assert_eq!(all.len(), 2);
}

// ============================================================
// mut 声明与重新赋值测试
// ============================================================

#[test]
fn test_var_reassignment_same_scope_ok() {
    // 同一作用域内 mut x = 1; x = 2 应该正常（重新赋值，不是遮蔽）
    let mut checker = TypeChecker::new("test");

    let stmt1 = ast::Stmt {
        kind: ast::StmtKind::Var {
            name: "x".to_string(),
            type_annotation: None,
            initializer: Some(Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(1),
                create_dummy_span(),
            ))),
            is_mut: true,
        },
        span: create_dummy_span(),
    };

    let stmt2 = ast::Stmt {
        kind: ast::StmtKind::Var {
            name: "x".to_string(),
            type_annotation: None,
            initializer: Some(Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(2),
                create_dummy_span(),
            ))),
            is_mut: false, // a = 2 被解析为 is_mut: false 的 Var
        },
        span: create_dummy_span(),
    };

    // 第一次 mut 声明正常
    assert!(checker.check_stmt(&stmt1).is_ok());
    // 同一作用域内重新赋值应该正常（不是遮蔽）
    assert!(
        checker.check_stmt(&stmt2).is_ok(),
        "同一作用域内对已有变量赋值不是遮蔽"
    );
}

#[test]
fn test_var_shadowing_in_inner_scope() {
    // 外层 x = 1; 内层 x = 2 应该报遮蔽错误
    let mut solver = TypeConstraintSolver::new();
    let mut checker = BodyChecker::new(&mut solver);

    // 外层声明 x
    checker.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));

    // 进入新作用域（模拟 if 块）
    checker.enter_scope();

    // 在新作用域中用 Var 语句声明同名变量 → 应该报遮蔽错误
    let inner_stmt = ast::Stmt {
        kind: ast::StmtKind::Var {
            name: "x".to_string(),
            type_annotation: None,
            initializer: Some(Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(2),
                create_dummy_span(),
            ))),
            is_mut: true,
        },
        span: create_dummy_span(),
    };
    let result = checker.check_stmt(&inner_stmt);
    assert!(result.is_err(), "内层作用域声明同名变量应该报遮蔽错误");

    checker.exit_scope();
}

// ============================================================
// For 循环遮蔽检查测试
// ============================================================

#[test]
fn test_for_loop_shadowing_error() {
    // x = 1; for x in [1, 2, 3] {} 应该报错
    let mut checker = TypeChecker::new("test");

    // 先声明 x
    let var_stmt = ast::Stmt {
        kind: ast::StmtKind::Expr(Box::new(ast::Expr::BinOp {
            op: ast::BinOp::Assign,
            left: Box::new(ast::Expr::Var("x".to_string(), create_dummy_span())),
            right: Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(10),
                create_dummy_span(),
            )),
            span: create_dummy_span(),
        })),
        span: create_dummy_span(),
    };
    assert!(checker.check_stmt(&var_stmt).is_ok());

    // for x in [1, 2, 3] {} 应该报遮蔽错误
    let for_stmt = ast::Stmt {
        kind: ast::StmtKind::For {
            var: "x".to_string(),
            var_mut: false,
            iterable: Box::new(ast::Expr::List(
                vec![ast::Expr::Lit(
                    crate::frontend::core::lexer::tokens::Literal::Int(1),
                    create_dummy_span(),
                )],
                create_dummy_span(),
            )),
            body: Box::new(ast::Block {
                stmts: vec![],
                expr: None,
                span: create_dummy_span(),
            }),
            label: None,
        },
        span: create_dummy_span(),
    };
    let result = checker.check_stmt(&for_stmt);
    assert!(result.is_err(), "for 循环使用已存在的变量名应该报遮蔽错误");
}

#[test]
fn test_for_loop_variable_scoped() {
    // for i in [1, 2, 3] {} 之后 i 不应该存在于外层
    let mut solver = TypeConstraintSolver::new();
    let mut checker = BodyChecker::new(&mut solver);

    // for 循环
    let for_stmt = ast::Stmt {
        kind: ast::StmtKind::For {
            var: "i".to_string(),
            var_mut: false,
            iterable: Box::new(ast::Expr::List(
                vec![ast::Expr::Lit(
                    crate::frontend::core::lexer::tokens::Literal::Int(1),
                    create_dummy_span(),
                )],
                create_dummy_span(),
            )),
            body: Box::new(ast::Block {
                stmts: vec![],
                expr: None,
                span: create_dummy_span(),
            }),
            label: None,
        },
        span: create_dummy_span(),
    };
    assert!(checker.check_stmt(&for_stmt).is_ok());

    // for 循环结束后，循环变量应该被销毁
    assert!(
        !checker.var_exists_in_any_scope("i"),
        "for 循环变量应该在循环结束后被销毁"
    );
}

#[test]
fn test_for_loop_no_conflict_with_unique_var() {
    // for i in [1, 2, 3] {} 没有冲突时应该正常工作
    let mut checker = TypeChecker::new("test");

    let for_stmt = ast::Stmt {
        kind: ast::StmtKind::For {
            var: "i".to_string(),
            var_mut: false,
            iterable: Box::new(ast::Expr::List(
                vec![
                    ast::Expr::Lit(
                        crate::frontend::core::lexer::tokens::Literal::Int(1),
                        create_dummy_span(),
                    ),
                    ast::Expr::Lit(
                        crate::frontend::core::lexer::tokens::Literal::Int(2),
                        create_dummy_span(),
                    ),
                ],
                create_dummy_span(),
            )),
            body: Box::new(ast::Block {
                stmts: vec![],
                expr: Some(Box::new(ast::Expr::Var(
                    "i".to_string(),
                    create_dummy_span(),
                ))),
                span: create_dummy_span(),
            }),
            label: None,
        },
        span: create_dummy_span(),
    };

    assert!(
        checker.check_stmt(&for_stmt).is_ok(),
        "无冲突的 for 循环应该正常工作"
    );
}

// ============================================================
// If 块作用域测试
// ============================================================

#[test]
fn test_if_block_creates_scope() {
    // if 块内声明的变量不应该泄漏到外层
    let mut solver = TypeConstraintSolver::new();
    let mut checker = BodyChecker::new(&mut solver);

    let if_stmt = ast::Stmt {
        kind: ast::StmtKind::If {
            condition: Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Bool(true),
                create_dummy_span(),
            )),
            then_branch: Box::new(ast::Block {
                stmts: vec![ast::Stmt {
                    kind: ast::StmtKind::Var {
                        name: "inner_var".to_string(),
                        type_annotation: None,
                        initializer: Some(Box::new(ast::Expr::Lit(
                            crate::frontend::core::lexer::tokens::Literal::Int(42),
                            create_dummy_span(),
                        ))),
                        is_mut: true,
                    },
                    span: create_dummy_span(),
                }],
                expr: None,
                span: create_dummy_span(),
            }),
            elif_branches: vec![],
            else_branch: None,
            span: create_dummy_span(),
        },
        span: create_dummy_span(),
    };

    assert!(checker.check_stmt(&if_stmt).is_ok());

    // if 块内声明的变量不应该泄漏到外层
    assert!(
        !checker.var_exists_in_any_scope("inner_var"),
        "if 块内的变量应该在块结束后被销毁"
    );
}

// ============================================================
// 赋值表达式遮蔽检查测试
// ============================================================

#[test]
fn test_assignment_shadowing_in_block() {
    // x = 1; if ... { x = 2 } 应该报遮蔽错误
    // 因为 x = 2 在 if 块内，x 只存在于外层作用域
    let mut solver = TypeConstraintSolver::new();
    let mut checker = BodyChecker::new(&mut solver);

    // 全局作用域：x = 1
    let assign_stmt = ast::Stmt {
        kind: ast::StmtKind::Expr(Box::new(ast::Expr::BinOp {
            op: ast::BinOp::Assign,
            left: Box::new(ast::Expr::Var("x".to_string(), create_dummy_span())),
            right: Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(1),
                create_dummy_span(),
            )),
            span: create_dummy_span(),
        })),
        span: create_dummy_span(),
    };
    assert!(checker.check_stmt(&assign_stmt).is_ok());

    // if 块内：x = 2 应该报遮蔽错误
    let if_stmt = ast::Stmt {
        kind: ast::StmtKind::If {
            condition: Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Bool(true),
                create_dummy_span(),
            )),
            then_branch: Box::new(ast::Block {
                stmts: vec![ast::Stmt {
                    kind: ast::StmtKind::Expr(Box::new(ast::Expr::BinOp {
                        op: ast::BinOp::Assign,
                        left: Box::new(ast::Expr::Var("x".to_string(), create_dummy_span())),
                        right: Box::new(ast::Expr::Lit(
                            crate::frontend::core::lexer::tokens::Literal::Int(2),
                            create_dummy_span(),
                        )),
                        span: create_dummy_span(),
                    })),
                    span: create_dummy_span(),
                }],
                expr: None,
                span: create_dummy_span(),
            }),
            elif_branches: vec![],
            else_branch: None,
            span: create_dummy_span(),
        },
        span: create_dummy_span(),
    };
    let result = checker.check_stmt(&if_stmt);
    assert!(result.is_err(), "在 if 块中对外层变量赋值应该报遮蔽错误");
}

#[test]
fn test_assignment_in_same_scope_ok() {
    // x = 1; x = 2 在同一作用域应该正常（赋值操作）
    let mut solver = TypeConstraintSolver::new();
    let mut checker = BodyChecker::new(&mut solver);

    let stmt1 = ast::Stmt {
        kind: ast::StmtKind::Expr(Box::new(ast::Expr::BinOp {
            op: ast::BinOp::Assign,
            left: Box::new(ast::Expr::Var("x".to_string(), create_dummy_span())),
            right: Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(1),
                create_dummy_span(),
            )),
            span: create_dummy_span(),
        })),
        span: create_dummy_span(),
    };

    let stmt2 = ast::Stmt {
        kind: ast::StmtKind::Expr(Box::new(ast::Expr::BinOp {
            op: ast::BinOp::Assign,
            left: Box::new(ast::Expr::Var("x".to_string(), create_dummy_span())),
            right: Box::new(ast::Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(2),
                create_dummy_span(),
            )),
            span: create_dummy_span(),
        })),
        span: create_dummy_span(),
    };

    assert!(checker.check_stmt(&stmt1).is_ok());
    assert!(
        checker.check_stmt(&stmt2).is_ok(),
        "同一作用域内的重复赋值应该正常工作"
    );
}

// ============================================================
// ExprInferrer 遮蔽检查测试
// ============================================================

#[test]
fn test_inferrer_try_add_var_shadowing() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    // 全局：x = Int
    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));

    // 进入新作用域
    inferrer.enter_scope();

    // 尝试在新作用域声明同名变量 → 应该报遮蔽错误
    let result = inferrer.try_add_var(
        "x".to_string(),
        PolyType::mono(MonoType::String),
        create_dummy_span(),
    );
    assert!(result.is_err(), "try_add_var 应检测到遮蔽");

    inferrer.exit_scope();
}

#[test]
fn test_inferrer_scope_destroyed_on_exit() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut inferrer = ExprInferrer::new(&mut solver, &overload_candidates);

    inferrer.enter_scope();
    inferrer.add_var("temp".to_string(), PolyType::mono(MonoType::Bool));
    assert!(inferrer.get_var("temp").is_some());

    inferrer.exit_scope();
    assert!(
        inferrer.get_var("temp").is_none(),
        "退出作用域后变量应该被销毁"
    );
}

// ============================================================
// 函数作用域测试
// ============================================================

#[test]
fn test_fn_def_creates_scope() {
    // 函数内参数和局部变量不应泄漏到外层
    let mut solver = TypeConstraintSolver::new();
    let mut checker = BodyChecker::new(&mut solver);

    let fn_stmt = ast::Stmt {
        kind: ast::StmtKind::Expr(Box::new(ast::Expr::FnDef {
            name: "my_fn".to_string(),
            params: vec![ast::Param {
                name: "param".to_string(),
                ty: Some(ast::Type::Name("Int".to_string())),
                span: create_dummy_span(),
            }],
            return_type: Some(ast::Type::Name("Int".to_string())),
            body: Box::new(ast::Block {
                stmts: vec![],
                expr: Some(Box::new(ast::Expr::Var(
                    "param".to_string(),
                    create_dummy_span(),
                ))),
                span: create_dummy_span(),
            }),
            is_async: false,
            span: create_dummy_span(),
        })),
        span: create_dummy_span(),
    };

    assert!(checker.check_stmt(&fn_stmt).is_ok());

    // 函数参数不应泄漏到外层
    assert!(
        !checker.var_exists_in_any_scope("param"),
        "函数参数应该在函数作用域结束后被销毁"
    );
}
