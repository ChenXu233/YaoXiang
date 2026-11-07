//! Infix 表达式解析测试 — 基于语言规范 §4.2 运算符优先级表
//!
//! 规范 §4.2 定义的运算符:
//!   优先级 1: () . []      左结合（函数调用、成员访问、索引）
//!   优先级 2: as           左结合（类型转换）
//!   优先级 3: * / %        左结合（乘除模）
//!   优先级 4: + -          左结合（加减）
//!   优先级 7: == != < > <= >=  左结合（比较）
//!   优先级 9: and or       左结合（逻辑）
//!   优先级 11: =           右结合（赋值）
//!   Range: ..              （自定义优先级）
//!   ?:                     （错误传播）
//!   =>:                    （Lambda，右结合）

use crate::frontend::core::lexer::tokenize;
use crate::ast::{BinOp, Expr};
use crate::frontend::core::parser::{NullProbe, ParserState};

fn parse_expr(source: &str) -> Expr {
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens, &NullProbe);
    state
        .parse_expression(crate::frontend::core::parser::BP_LOWEST)
        .expect("parse failed")
}

// infix_info: token → infix parser 路由

#[test]
fn test_infix_info_binary_ops() {
    // spec §4.2 优先级 3-4: 算术运算符
    for op in &["+", "-", "*", "/", "%"] {
        let tokens = tokenize(op).unwrap();
        let state = ParserState::new(&tokens, &NullProbe);
        assert!(
            state.infix_info().is_some(),
            "infix_info should handle '{op}'"
        );
    }
}

#[test]
fn test_infix_info_eof() {
    let tokens = tokenize("").unwrap();
    let state = ParserState::new(&tokens, &NullProbe);
    assert!(state.infix_info().is_none());
}

// 二元算术运算 — spec §4.2 优先级 3-4

#[test]
fn test_infix_add() {
    let expr = parse_expr("1 + 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Add, .. }));
}

// 函数调用 — spec §4.2 优先级 1 / §4.3

#[test]
fn test_infix_call_no_args() {
    let expr = parse_expr("f()");
    assert!(matches!(expr, Expr::Call { ref args, ref named_args, .. }
        if args.is_empty() && named_args.is_empty()));
}
