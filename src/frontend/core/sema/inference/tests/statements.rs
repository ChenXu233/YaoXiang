//! 语句检查测试 — 基于语言规范 §5 & RFC-010
//!
//! §5.1-§5.9: 语句分类
//! RFC-010: 统一类型语法

use crate::frontend::core::sema::inference::statements::StatementChecker;
use crate::frontend::core::types::{MonoType, PolyType, TraitTable, TypeConstraintSolver};
use crate::ast::{self, Stmt, StmtKind, Expr, BinOp, Block, PlanId};
use crate::frontend::core::lexer::tokens::Literal;
use crate::util::span::Span;

// 辅助函数

/// 内建类型在 AST 里的唯一写法：Name（WBS 6.3 删除了 Int/Float/… 专用变体）。
fn builtin_ty(name: &str) -> ast::Type {
    ast::Type::Name {
        name: name.to_string(),
        span: Span::dummy(),
    }
}

/// 从 StmtKind 构造 Stmt，使用 dummy span
fn make_stmt(kind: StmtKind) -> Stmt {
    Stmt {
        kind,
        span: Span::dummy(),
        plan_id: PlanId::UNPLANNED,
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
    StatementChecker::new(
        &mut solver,
        None,
        crate::frontend::core::types::eval::dependent_types::DependentTypeEnv::new(),
        TraitTable::default(),
    )
}

// Happy path 测试

/// §5.1: 基础创建 → Ok，StatementChecker::new 不 panic
#[test]
fn test_statement_checker_creation() {
    // Arrange
    let mut solver = TypeConstraintSolver::default();
    // Act
    let _checker = StatementChecker::new(
        &mut solver,
        None,
        crate::frontend::core::types::eval::dependent_types::DependentTypeEnv::new(),
        TraitTable::default(),
    );

    // Assert — 应该成功创建，不 panic
}

/// §5.3: 变量声明带类型注解 `x: Int = 42` → Ok，get_var("x") 存在
#[test]
fn test_check_var_stmt_with_type_annotation() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::Assign {
        target: Box::new(Expr::Var("x".to_string(), Span::dummy())),
        type_annotation: Some(builtin_ty("Int64")),
        value: Some(Box::new(Expr::Lit(Literal::Int(42), Span::dummy()))),
        is_mut: false,
        span: Span::dummy(),
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
    let stmt = make_stmt(StmtKind::Assign {
        target: Box::new(Expr::Var("x".to_string(), Span::dummy())),
        type_annotation: None,
        value: Some(Box::new(Expr::Lit(Literal::Int(42), Span::dummy()))),
        is_mut: false,
        span: Span::dummy(),
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
        else_if_branches: vec![],
        else_branch: None,
        span: Span::dummy(),
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(result.is_ok(), "if true {{}} 应成功");
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

/// §5.3: 赋值给已有局部变量（同类型）→ Ok，类型保持声明类型
/// （赋值类型统一：右值必须与变量当前类型统一，见 assign_var 的 enforce_unify。
///   场景是 main 块体内的局部变量，故先 enter_scope 再注册——globals 层是
///   std/模块导出，撞名赋值走旧覆写行为，不在此测试范围）
#[test]
fn test_check_expr_stmt_reassignment_same_type_succeeds() {
    // Arrange
    let mut checker = make_checker();
    checker.enter_scope();
    checker.add_var(
        "x".to_string(),
        PolyType::mono(MonoType::Int(64)),
        true,
        Span::dummy(),
    );
    let stmt = make_stmt(StmtKind::Expr(Box::new(Expr::BinOp {
        op: BinOp::Assign,
        left: Box::new(Expr::Var("x".to_string(), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::Int(100), Span::dummy())),
        span: Span::dummy(),
    })));

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(result.is_ok(), "同类型重赋值应成功: {:?}", result.err());
    assert!(
        matches!(checker.get_var("x"), Some(poly) if poly.body == MonoType::Int(64)),
        "重赋值后变量类型应保持声明的 Int(64)"
    );
}

/// §5.3: 赋值给已有局部变量（类型可变）→ Err E1002
/// （工作流验证 Bug 1：此前赋值直接覆写类型，Int 变量可赋 String，
/// 漏检到运行时 E6007 才爆——赋值必须与声明类型统一）
#[test]
fn test_check_expr_stmt_reassignment_type_change_rejected() {
    // Arrange
    let mut checker = make_checker();
    checker.enter_scope();
    checker.add_var(
        "x".to_string(),
        PolyType::mono(MonoType::Int(64)),
        true,
        Span::dummy(),
    );
    let stmt = make_stmt(StmtKind::Expr(Box::new(Expr::BinOp {
        op: BinOp::Assign,
        left: Box::new(Expr::Var("x".to_string(), Span::dummy())),
        right: Box::new(Expr::Lit(Literal::String("str".to_string()), Span::dummy())),
        span: Span::dummy(),
    })));

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    let err = result.expect_err("类型可变重赋值应被拒绝（E1002）");
    assert!(
        format!("{:?}", err).contains("E1002"),
        "应报 E1002 type mismatch，实际: {:?}",
        err
    );
}

// Error path 测试

/// §5.6: if 条件非 Bool 类型 → Err（§5.6 条件必须是 Bool）
#[test]
fn test_check_if_stmt_non_bool_condition() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::If {
        condition: Box::new(Expr::Lit(Literal::Int(42), Span::dummy())),
        then_branch: Box::new(make_block(vec![])),
        else_if_branches: vec![],
        else_branch: None,
        span: Span::dummy(),
    });

    // Act
    let result = checker.check_stmt(&stmt);

    // Assert
    assert!(result.is_err(), "if 42 {{}} 应返回 Err（条件非 Bool）");
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

// Boundary 测试

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

/// §5.1: 大量语句不 panic
#[test]
fn test_check_statement_checker_with_many_statements() {
    // Arrange

    let mut stmts = Vec::new();
    for i in 0..100 {
        stmts.push(make_stmt(StmtKind::Assign {
            target: Box::new(Expr::Var(format!("var_{}", i), Span::dummy())),
            type_annotation: Some(builtin_ty("Int64")),
            value: Some(Box::new(Expr::Lit(Literal::Int(i as i128), Span::dummy()))),
            is_mut: false,
            span: Span::dummy(),
        }));
    }
    let block = make_block(stmts);

    // Act
    let mut solver = TypeConstraintSolver::default();
    let mut checker = StatementChecker::new(
        &mut solver,
        None,
        crate::frontend::core::types::eval::dependent_types::DependentTypeEnv::new(),
        TraitTable::default(),
    );
    let result = checker.check_fn_def("test_fn", &[], &block);

    // Assert
    assert!(
        result.is_ok(),
        "大量变量声明语句应全部处理成功: {:?}",
        result.err()
    );
}

/// §5.3: 变量声明：只有类型注解，无初始化器
#[test]
fn test_check_var_stmt_only_annotation() {
    // Arrange
    let mut checker = make_checker();
    let stmt = make_stmt(StmtKind::Assign {
        target: Box::new(Expr::Var("y".to_string(), Span::dummy())),
        type_annotation: Some(builtin_ty("Float64")),
        value: None,
        is_mut: false,
        span: Span::dummy(),
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
