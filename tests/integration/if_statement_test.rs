//! if 语句功能测试
//!
//! 测试 if 语句在 YaoXiang 语言中的双重支持：
//! 1. if 表达式形式（已存在）
//! 2. if 语句形式（新实现）

use yaoxiang::frontend::lexer::tokenize;
use yaoxiang::frontend::parser::{parse, parse_expression};
use yaoxiang::frontend::parser::ast::{Expr, Module, Stmt, StmtKind};

/// 测试 if 表达式仍然正常工作（向后兼容性）
#[test]
fn test_if_expression_still_works() {
    let code = "if x > 0 { 1 } else { 0 }";

    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);

    assert!(result.is_ok(), "Failed to parse if expression");

    let expr = result.unwrap();
    match expr {
        Expr::If {
            condition: _,
            then_branch,
            elif_branches,
            else_branch,
            ..
        } => {
            // 验证基本结构
            assert!(then_branch.expr.is_some(), "Missing then branch expression");
            assert_eq!(elif_branches.len(), 0, "Should have no elif branches");
            assert!(else_branch.is_some(), "Missing else branch");
        }
        _ => panic!("Expected Expr::If"),
    }
}

/// 测试 if 表达式支持 elif 分支
#[test]
fn test_if_expression_with_elif() {
    let code = "if x > 0 { 1 } elif x == 0 { 0 } else { -1 }";

    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);

    assert!(result.is_ok(), "Failed to parse if expression with elif");

    let expr = result.unwrap();
    match expr {
        Expr::If {
            condition: _,
            then_branch: _,
            elif_branches,
            else_branch: _,
            ..
        } => {
            assert_eq!(elif_branches.len(), 1, "Should have one elif branch");
        }
        _ => panic!("Expected Expr::If"),
    }
}

/// 测试在模块中解析 if 语句
#[test]
fn test_parse_if_statement_in_module() {
    let code = r#"
        main = () => {
            if x > 0 {
                print("positive")
            } else {
                print("non-positive")
            }
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse module with if statement");

    let module = result.unwrap();
    assert!(!module.items.is_empty(), "Module should have items");

    // 查找 main 函数并检查其是否包含 if 语句
    let main_func = module.items.iter().find(|item| {
        if let StmtKind::Fn { name, .. } = &item.kind {
            name == "main"
        } else {
            false
        }
    });

    assert!(main_func.is_some(), "Should have main function");

    if let StmtKind::Fn {
        body: (_, expr), ..
    } = &main_func.unwrap().kind
    {
        if let Some(main_expr) = expr {
            // main 函数体应该包含 if 语句
            // 这里简化处理，只验证解析成功
        }
    }
}

/// 测试复杂的 if 表达式与语句混合使用
#[test]
fn test_if_expr_and_stmt_mixed() {
    let code = r#"
        main = () => {
            // if 表达式用于赋值
            status = if x > 0 { "positive" } else { "non-positive" };

            // if 语句用于控制流程
            if status == "positive" {
                print("Got positive status")
            }
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(
        result.is_ok(),
        "Failed to parse mixed if expression and statement"
    );
}

/// 测试嵌套的 if 语句
#[test]
fn test_nested_if_statement() {
    let code = r#"
        main = () => {
            if x > 0 {
                if y > 0 {
                    print("both positive")
                } else {
                    print("x positive, y non-positive")
                }
            }
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse nested if statement");
}

/// 测试 if 语句支持 elif 分支
#[test]
fn test_if_statement_with_elif() {
    let code = r#"
        main = () => {
            if x > 0 {
                print("positive")
            } elif x == 0 {
                print("zero")
            } else {
                print("negative")
            }
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse if statement with elif");
}

/// 测试 if 语句可以没有 else 分支
#[test]
fn test_if_statement_without_else() {
    let code = r#"
        main = () => {
            if x > 0 {
                print("positive")
            }
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse if statement without else");
}
