//! 语句检查测试 — 基于语言规范 §5 & RFC-010
//!
//! §5.1-§5.9: 语句分类
//! RFC-010: 统一类型语法

use crate::frontend::core::typecheck::inference::statements::StatementChecker;
use crate::frontend::core::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::core::parser::ast::{self, Stmt, StmtKind, Expr, BinOp, Block, Param};
use crate::frontend::core::lexer::tokens::Literal;
use crate::util::span::Span;

// ===================================================================
// 辅助函数
// ===================================================================

/// 从 StmtKind 构造 Stmt，使用 dummy span
fn make_stmt(kind: StmtKind) -> Stmt {
    Stmt {
        kind,
        span: Span::dummy(),
    }
}

/// 构造 Block
fn make_block(stmts: Vec<Stmt>) -> Block {
    Block {
        stmts,
        span: Span::dummy(),
    }
}

/// 创建默认 StatementChecker
fn make_checker() -> StatementChecker {
    let mut solver = TypeConstraintSolver::default();
    StatementChecker::new(&mut solver)
}

/// 创建带 scope 内已有变量的 StatementChecker
fn make_checker_with_var(
    name: &str,
    ty: MonoType,
) -> StatementChecker {
    let mut checker = make_checker();
    checker.add_var(
        name.to_string(),
        PolyType::mono(ty),
        false,
        crate::util::span::Span::default(),
    );
    checker
}

// ===================================================================
// Happy path 测试
// ===================================================================

/// §5.1: 基础创建 → Ok，StatementChecker::new 不 panic
#[test]
fn test_statement_checker_creation() {
    // Arrange
    let mut solver = TypeConstraintSolver::default();

    // Act
    let _checker = StatementChecker::new(&mut solver);

    // Assert — 应该成功创建，不 panic
}

/// §5.3: 变量声明带类型注解 `x: Int = 42` → Ok，get_var("x") 存在
#[test]
fn test_check_var_stmt_with_type_annotation() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::Var {
        name: "x".to_string(),
        name_span: Span::dummy(),
        type_annotation: Some(ast::Type::Int(64)),
        initializer: Some(Box::new(Expr::Lit(Literal::Int(42), Span::dummy()))),
        is_mut: false,
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(
        result.is_ok(),
        "带类型注解的变量声明应成功: {:?}",
        result.err()
    );
    assert!(
        checker.get_var("x").is_some(),
        "声明后 get_var(\"x\") 应返回 Some"
    );
}

/// §5.3: 变量声明无类型注解 `x = 42` → Ok，get_var("x") 存在
#[test]
fn test_check_var_stmt_type_inference() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::Var {
        name: "x".to_string(),
        name_span: Span::dummy(),
        type_annotation: None,
        initializer: Some(Box::new(Expr::Lit(Literal::Int(42), Span::dummy()))),
        is_mut: false,
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(
        result.is_ok(),
        "无类型注解的变量声明应通过推断成功: {:?}",
        result.err()
    );
    assert!(
        checker.get_var("x").is_some(),
        "声明后 get_var(\"x\") 应返回 Some"
    );
}

/// §5.6: if 语句条件为 Bool → Ok
#[test]
fn test_check_if_stmt_bool_condition() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::If {
        condition: Box::new(Expr::Lit(Literal::Bool(true), Span::dummy())),
        then_branch: Box::new(make_block(vec![])),
        elif_branches: vec![],
        else_branch: None,
        span: Span::dummy(),
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(result.is_ok(), "if true {{}} 应成功");
}

/// §5.6: if 语句带 elif 和 else 分支 → Ok
#[test]
fn test_check_if_stmt_with_elif_and_else() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::If {
        condition: Box::new(Expr::Lit(Literal::Bool(true), Span::dummy())),
        then_branch: Box::new(make_block(vec![])),
        elif_branches: vec![(
            Box::new(Expr::Lit(Literal::Bool(false), Span::dummy())),
            Box::new(make_block(vec![])),
        )],
        else_branch: Some(Box::new(make_block(vec![]))),
        span: Span::dummy(),
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(
        result.is_ok(),
        "if/elif/else 语句中条件均为 Bool 时应成功: {:?}",
        result.err()
    );
}

/// §5.3: 表达式语句（赋值）→ Ok，赋值后变量存在
#[test]
fn test_check_expr_stmt_assignment_creates_var() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::Expr(Box::new(Expr::BinOp {
        op: BinOp::Assign,
        left: Box::new(Expr::Var("x".to_string(), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(99), Span::dummy())),
        span: Span::dummy(),
    })));

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(result.is_ok(), "赋值语句应成功: {:?}", result.err());
    assert!(
        checker.get_var("x").is_some(),
        "赋值后 get_var(\"x\") 应返回 Some"
    );
}

/// §5.3: 赋值给已有变量 → Ok，类型被更新
#[test]
fn test_check_expr_stmt_reassignment_updates_type() {
    // Arrange
    let mut checker = make_checker_with_var("x", MonoType::Int(32));
    let stmt = make_stmt(StmtKind::Expr(Box::new(Expr::BinOp {
        op: BinOp::Assign,
        left: Box::new(Expr::Var("x".to_string(), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(100), Span::dummy())),
        span: Span::dummy(),
    })));

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(result.is_ok(), "重新赋值应成功: {:?}", result.err());
}

/// §5.9.2: for 循环遍历 Range → Ok
#[test]
fn test_check_for_stmt_range() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::For {
        var: "i".to_string(),
        var_span: Span::dummy(),
        var_mut: false,
        iterable: Box::new(Expr::BinOp {
            op: BinOp::Range,
            left: Box::new(Expr::Lit(Literal::Int(0), Span::dummy())),
            right: Box::new(Expr::Lit(Literal::Int(10), Span::dummy())),
            span: Span::dummy(),
        }),
        body: Box::new(make_block(vec![])),
        label: None,
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(
        result.is_ok(),
        "for 循环遍历 Range 应成功: {:?}",
        result.err()
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

/// §5.6: if 条件非 Bool 类型 → Err（§5.6 条件必须是 Bool）
#[test]
fn test_check_if_stmt_non_bool_condition() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::If {
        condition: Box::new(Expr::Lit(Literal::Int(42), Span::dummy())),
        then_branch: Box::new(make_block(vec![])),
        elif_branches: vec![],
        else_branch: None,
        span: Span::dummy(),
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(result.is_err(), "if 42 {{}} 应返回 Err（条件非 Bool）");
}

/// §5.6: elif 条件非 Bool 类型 → Err
#[test]
fn test_check_if_stmt_non_bool_elif_condition() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::If {
        condition: Box::new(Expr::Lit(Literal::Bool(true), Span::dummy())),
        then_branch: Box::new(make_block(vec![])),
        elif_branches: vec![(
            Box::new(Expr::Lit(Literal::Int(1), Span::dummy())),
            Box::new(make_block(vec![])),
        )],
        else_branch: None,
        span: Span::dummy(),
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(result.is_err(), "elif 条件为 Int 时应返回 Err");
}

/// §5.3: 使用未定义变量 → Err
#[test]
fn test_check_undefined_variable() {
    // Arrange
    let mut checker = make_checker();
    let expr = Expr::Var("undefined_var".to_string(), Span::dummy());

    // Act
    let result = checker.check_expr(&expr);

    // Assert
    assert!(result.is_err(), "使用未定义变量应返回 Err");
}

/// §5.3.1: 变量声明类型注解与初始化器不匹配 → Err
#[test]
fn test_check_var_stmt_type_mismatch() {
    // Arrange
    let mut checker = make_checker();
    // x: Bool = 42 — Int 字面量与 Bool 注解不匹配
    let stmt = make_stmt(StmtKind::Var {
        name: "x".to_string(),
        name_span: Span::dummy(),
        type_annotation: Some(ast::Type::Bool),
        initializer: Some(Box::new(Expr::Lit(Literal::Int(42), Span::dummy()))),
        is_mut: false,
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(result.is_err(), "x: Bool = 42 类型不匹配应返回 Err");
}

/// §5.9.3: for 循环变量遮蔽外层变量 → Err
#[test]
fn test_check_for_stmt_shadowing() {
    // Arrange
    let mut checker = make_checker_with_var("i", MonoType::Int(32));
    let stmt = make_stmt(StmtKind::For {
        var: "i".to_string(),
        var_span: Span::dummy(),
        var_mut: false,
        iterable: Box::new(Expr::BinOp {
            op: BinOp::Range,
            left: Box::new(Expr::Lit(Literal::Int(0), Span::dummy())),
            right: Box::new(Expr::Lit(Literal::Int(5), Span::dummy())),
            span: Span::dummy(),
        }),
        body: Box::new(make_block(vec![])),
        label: None,
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(
        result.is_err(),
        "for i in 1..5 {{}} 当外层已存在 i 时应返回 Err（变量遮蔽）"
    );
}

// ===================================================================
// Boundary 测试
// ===================================================================

/// §5.2: 嵌套作用域：内层变量对外层不可见
#[test]
fn test_check_nested_scopes() {
    // Arrange
    let mut checker = make_checker();

    // Act — 进入内层作用域，添加变量
    checker.enter_scope();
    checker.add_var(
        "inner_var".to_string(),
        PolyType::mono(MonoType::Int(32)),
        false,
        crate::util::span::Span::default(),
    );

    // Assert — 内层变量存在
    assert!(
        checker.get_var("inner_var").is_some(),
        "内层作用域中 get_var(\"inner_var\") 应返回 Some"
    );

    // Act — 退出内层作用域
    checker.exit_scope();

    // Assert — 外层不可见
    assert!(
        checker.get_var("inner_var").is_none(),
        "退出作用域后 get_var(\"inner_var\") 应返回 None"
    );
}

/// §5.2: 嵌套作用域：外层变量对内层可见
#[test]
fn test_check_nested_scopes_outer_visible_in_inner() {
    // Arrange
    let mut checker = make_checker();
    checker.add_var(
        "outer_var".to_string(),
        PolyType::mono(MonoType::Bool),
        false,
        crate::util::span::Span::default(),
    );

    // Act — 进入内层作用域
    checker.enter_scope();

    // Assert — 内层可以访问外层变量
    assert!(
        checker.get_var("outer_var").is_some(),
        "内层作用域应能访问外层变量 outer_var"
    );

    // Act — 退出内层
    checker.exit_scope();

    // Assert — 外层变量仍然存在
    assert!(
        checker.get_var("outer_var").is_some(),
        "退出内层后外层变量 outer_var 应仍然存在"
    );
}

/// §5.2: var_exists_in_any_scope 和 var_exists_in_current_scope 区分
#[test]
fn test_check_scope_level_queries() {
    // Arrange
    let mut checker = make_checker();
    checker.add_var(
        "global".to_string(),
        PolyType::mono(MonoType::String),
        false,
        crate::util::span::Span::default(),
    );

    // Act — 进入内层
    checker.enter_scope();

    // Assert — global 在任何作用域中存在，但不在当前作用域
    assert!(
        checker.var_exists_in_any_scope("global"),
        "var_exists_in_any_scope(\"global\") 应为 true"
    );
    assert!(
        !checker.var_exists_in_current_scope("global"),
        "var_exists_in_current_scope(\"global\") 在内层应为 false"
    );

    // Act — 添加到当前作用域
    checker.add_var(
        "local".to_string(),
        PolyType::mono(MonoType::Float(64)),
        false,
        crate::util::span::Span::default(),
    );

    // Assert
    assert!(
        checker.var_exists_in_current_scope("local"),
        "var_exists_in_current_scope(\"local\") 应为 true"
    );

    // Cleanup
    checker.exit_scope();
}

/// §5.1: 大量语句不 panic
#[test]
fn test_check_statement_checker_with_many_statements() {
    // Arrange

    let mut stmts = Vec::new();
    for i in 0..100 {
        stmts.push(make_stmt(StmtKind::Var {
            name: format!("var_{}", i),
            name_span: Span::dummy(),
            type_annotation: Some(ast::Type::Int(64)),
            initializer: Some(Box::new(Expr::Lit(Literal::Int(i as i128), Span::dummy()))),
            is_mut: false,
        }));
    }
    let block = make_block(stmts);

    // Act
    let mut solver = TypeConstraintSolver::default();
    let mut checker = StatementChecker::new(&mut solver);
    let result = checker.check_fn_def("test_fn", &[], &block);

    // Assert
    assert!(
        result.is_ok(),
        "大量变量声明语句应全部处理成功: {:?}",
        result.err()
    );
}

/// §5.8: check_fn_def 处理带参数的函数
#[test]
fn test_check_fn_def_with_params() {
    // Arrange
    let mut checker = make_checker();
    let params = vec![
        Param {
            name: "a".to_string(),
            ty: Some(ast::Type::Int(32)),
            is_mut: false,
            span: Span::dummy(),
        },
        Param {
            name: "b".to_string(),
            ty: Some(ast::Type::Bool),
            is_mut: false,
            span: Span::dummy(),
        },
    ];
    let body = make_block(vec![Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Var("a".to_string(), Span::dummy()))),
        span: Span::dummy(),
    }]);

    // Act
    let result = checker.check_fn_def("my_fn", &params, &body);

    // Assert
    assert!(result.is_ok(), "带参数的函数定义应成功: {:?}", result.err());
}

/// §5.8: check_fn_def 参数在函数体内可见
#[test]
fn test_check_fn_def_params_visible_in_body() {
    // Arrange
    let mut checker = make_checker();
    let params = vec![Param {
        name: "x".to_string(),
        ty: Some(ast::Type::Int(32)),
        is_mut: false,
        span: Span::dummy(),
    }];
    // 函数体中使用参数 x + 1
    let body = make_block(vec![Stmt {
        kind: StmtKind::Expr(Box::new(Expr::BinOp {
            op: BinOp::Add,
            left: Box::new(Expr::Var("x".to_string(), Span::dummy())),
            right: Box::new(Expr::Lit(Literal::Int(1), Span::dummy())),
            span: Span::dummy(),
        })),
        span: Span::dummy(),
    }]);

    // Act
    let result = checker.check_fn_def("inc", &params, &body);

    // Assert
    assert!(result.is_ok(), "函数体内使用参数应成功: {:?}", result.err());
}

/// §5.3: 变量声明：只有类型注解，无初始化器
#[test]
fn test_check_var_stmt_only_annotation() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::Var {
        name: "y".to_string(),
        name_span: Span::dummy(),
        type_annotation: Some(ast::Type::Float(64)),
        initializer: None,
        is_mut: false,
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(
        result.is_ok(),
        "只有类型注解的变量声明应成功: {:?}",
        result.err()
    );
    assert!(
        checker.get_var("y").is_some(),
        "声明后 get_var(\"y\") 应返回 Some"
    );
}

/// §5.3: 表达式语句（非赋值）→ Ok
#[test]
fn test_check_expr_stmt_non_assignment() {
    // Arrange
    let mut checker = make_checker();
    // 纯表达式语句：1 + 2
    let stmt = make_stmt(StmtKind::Expr(Box::new(Expr::BinOp {
        op: BinOp::Add,
        left: Box::new(Expr::Lit(Literal::Int(1), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(2), Span::dummy())),
        span: Span::dummy(),
    })));

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(result.is_ok(), "纯表达式语句应成功: {:?}", result.err());
}
