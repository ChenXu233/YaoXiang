//! unsafe 和裸指针语法解析测试
//!
//! 验证 unsafe 块和裸指针语法的解析功能

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_expression;

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助函数：解析代码并返回 AST
    fn parse_expr(code: &str) -> crate::frontend::core::parser::ast::Expr {
        let tokens = tokenize(code).unwrap_or_else(|e| {
            panic!("词法分析失败: {:?}", e);
        });
        parse_expression(&tokens).unwrap_or_else(|e| {
            panic!("解析失败: {:?}", e);
        })
    }

    // ========== unsafe 块解析测试 ==========

    #[test]
    fn test_unsafe_block_basic() {
        // 基本 unsafe 块语法应该能解析
        let code = "unsafe { 42 }";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::Unsafe { body, .. } => {
                assert!(body.stmts.is_empty());
                assert!(body.expr.is_some());
            }
            _ => panic!("期望 unsafe 表达式"),
        }
    }

    #[test]
    fn test_unsafe_block_nested() {
        // 嵌套 unsafe 块
        let code = "unsafe { unsafe { 1 } }";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::Unsafe { body, .. } => {
                // 外部块包含内部 unsafe 表达式
                match &body.expr {
                    Some(inner) => match inner.as_ref() {
                        crate::frontend::core::parser::ast::Expr::Unsafe { .. } => {
                            // 内部 unsafe 块应该被正确解析
                        }
                        _ => panic!("期望内部 unsafe 表达式"),
                    },
                    None => panic!("期望表达式"),
                }
            }
            _ => panic!("期望 unsafe 表达式"),
        }
    }

    // ========== 解引用操作测试 ==========

    #[test]
    fn test_deref_expression() {
        // *ptr 解引用表达式
        let code = "*ptr";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::UnOp { op, expr, .. } => {
                if let crate::frontend::core::parser::ast::UnOp::Deref = op {
                    match expr.as_ref() {
                        crate::frontend::core::parser::ast::Expr::Var(name, _) => {
                            assert_eq!(name, "ptr");
                        }
                        _ => panic!("期望变量"),
                    }
                } else {
                    panic!("期望 Deref 操作符");
                }
            }
            _ => panic!("期望一元操作表达式"),
        }
    }

    #[test]
    fn test_deref_chain() {
        // **ptr 链式解引用
        let code = "**ptr";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::UnOp { op, expr, .. } => {
                if let crate::frontend::core::parser::ast::UnOp::Deref = op {
                    match expr.as_ref() {
                        crate::frontend::core::parser::ast::Expr::UnOp {
                            op: op2,
                            expr: expr2,
                            ..
                        } => {
                            if let crate::frontend::core::parser::ast::UnOp::Deref = op2 {
                                match expr2.as_ref() {
                                    crate::frontend::core::parser::ast::Expr::Var(name, _) => {
                                        assert_eq!(name, "ptr");
                                    }
                                    _ => panic!("期望变量"),
                                }
                            } else {
                                panic!("期望内部 Deref 操作符");
                            }
                        }
                        _ => panic!("期望嵌套解引用"),
                    }
                } else {
                    panic!("期望 Deref 操作符");
                }
            }
            _ => panic!("期望一元操作表达式"),
        }
    }

    #[test]
    fn test_deref_in_unsafe() {
        // unsafe 块内的解引用
        let code = "unsafe { *ptr }";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::Unsafe { body, .. } => {
                match &body.expr {
                    Some(expr) => match expr.as_ref() {
                        crate::frontend::core::parser::ast::Expr::UnOp { op, .. } => {
                            if let crate::frontend::core::parser::ast::UnOp::Deref = op {
                                // 正确识别为解引用
                            } else {
                                panic!("期望 Deref 操作符");
                            }
                        }
                        _ => panic!("期望解引用表达式"),
                    },
                    None => panic!("期望表达式"),
                }
            }
            _ => panic!("期望 unsafe 表达式"),
        }
    }

    // ========== 完整场景测试 ==========

    #[test]
    fn test_unsafe_with_deref_assignment() {
        // unsafe 块内指针赋值
        let code = "unsafe { *ptr = 42 }";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::Unsafe { body, .. } => {
                // 块内应该有解引用赋值
                assert!(!body.stmts.is_empty() || body.expr.is_some());
            }
            _ => panic!("期望 unsafe 表达式"),
        }
    }

    #[test]
    fn test_field_access_through_deref() {
        // 通过解引用访问字段
        let code = "(*ptr).x";
        let result = parse_expr(code);
        // 验证解析成功（不panic即为成功）
        match result {
            crate::frontend::core::parser::ast::Expr::FieldAccess { expr, .. } => {
                match expr.as_ref() {
                    crate::frontend::core::parser::ast::Expr::UnOp { op, .. } => {
                        if let crate::frontend::core::parser::ast::UnOp::Deref = op {
                            // 正确识别为解引用
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // ========== 边界测试 ==========

    #[test]
    fn test_unary_minus_not_deref() {
        // 负号不是解引用
        let code = "-x";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::UnOp { op, .. } => {
                if let crate::frontend::core::parser::ast::UnOp::Neg = op {
                    // 正确识别为负号
                } else {
                    panic!("期望 Neg 操作符");
                }
            }
            _ => panic!("期望一元操作表达式"),
        }
    }

    #[test]
    fn test_plus_not_deref() {
        // 正号不是解引用
        let code = "+x";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::UnOp { op, .. } => {
                if let crate::frontend::core::parser::ast::UnOp::Pos = op {
                    // 正确识别为正号
                } else {
                    panic!("期望 Pos 操作符");
                }
            }
            _ => panic!("期望一元操作表达式"),
        }
    }

    #[test]
    fn test_logical_not_not_deref() {
        // 逻辑非不是解引用
        let code = "!x";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::UnOp { op, .. } => {
                if let crate::frontend::core::parser::ast::UnOp::Not = op {
                    // 正确识别为逻辑非
                } else {
                    panic!("期望 Not 操作符");
                }
            }
            _ => panic!("期望一元操作表达式"),
        }
    }

    #[test]
    fn test_deref_with_literal() {
        // 解引用常量（虽然类型检查会失败，但解析应该成功）
        let code = "*42";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::UnOp { op, .. } => {
                if let crate::frontend::core::parser::ast::UnOp::Deref = op {
                    // 正确识别为解引用
                } else {
                    panic!("期望 Deref 操作符");
                }
            }
            _ => panic!("期望一元操作表达式"),
        }
    }

    #[test]
    fn test_unsafe_with_multiple_statements() {
        // 多个语句的 unsafe 块
        let code = "unsafe { x = 1; y = 2; x + y }";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::Unsafe { body, .. } => {
                // 应该有语句和表达式
                assert!(body.stmts.len() >= 2);
            }
            _ => panic!("期望 unsafe 表达式"),
        }
    }

    #[test]
    fn test_unsafe_in_binary_op() {
        // unsafe 块在二元操作中
        let code = "unsafe { 1 } + unsafe { 2 }";
        let result = parse_expr(code);
        match result {
            crate::frontend::core::parser::ast::Expr::BinOp { .. } => {
                // 二元加法成功
            }
            _ => panic!("期望二元操作表达式"),
        }
    }
}
