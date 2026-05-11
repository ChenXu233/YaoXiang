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
use crate::frontend::core::parser::ast::{BinOp, Expr, UnOp};
use crate::frontend::core::parser::parse_expression;
use crate::frontend::core::lexer::tokens::TokenKind;
use crate::frontend::core::parser::ParserState;

fn parse_expr(source: &str) -> Expr {
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens);
    state
        .parse_expression(crate::frontend::core::parser::BP_LOWEST)
        .expect("parse failed")
}

// ============================================================================
// infix_info: token → infix parser 路由
// ============================================================================

#[test]
fn test_infix_info_binary_ops() {
    // spec §4.2 优先级 3-4: 算术运算符
    for op in &["+", "-", "*", "/", "%"] {
        let tokens = tokenize(op).unwrap();
        let state = ParserState::new(&tokens);
        assert!(
            state.infix_info().is_some(),
            "infix_info should handle '{op}'"
        );
    }
}

#[test]
fn test_infix_info_equality() {
    for op in &["==", "!="] {
        let tokens = tokenize(op).unwrap();
        let state = ParserState::new(&tokens);
        assert!(state.infix_info().is_some());
    }
}

#[test]
fn test_infix_info_comparison() {
    for op in &["<", ">", "<=", ">="] {
        let tokens = tokenize(op).unwrap();
        let state = ParserState::new(&tokens);
        assert!(state.infix_info().is_some());
    }
}

#[test]
fn test_infix_info_assign() {
    let tokens = tokenize("=").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_range() {
    let tokens = tokenize("..").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_some());
}

#[test]
fn test_infix_info_call_field_index() {
    // spec §4.2 优先级 1: () . []
    for tok in &["(", ".", "["] {
        let tokens = tokenize(tok).unwrap();
        let state = ParserState::new(&tokens);
        assert!(state.infix_info().is_some());
    }
}

#[test]
fn test_infix_info_cast_try_lambda() {
    for tok in &["as", "?", "=>"] {
        let tokens = tokenize(tok).unwrap();
        let state = ParserState::new(&tokens);
        assert!(
            state.infix_info().is_some(),
            "infix_info should handle '{tok}'"
        );
    }
}

#[test]
fn test_infix_info_eof() {
    let tokens = tokenize("").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.infix_info().is_none());
}

// ============================================================================
// 二元算术运算 — spec §4.2 优先级 3-4
// ============================================================================

#[test]
fn test_infix_add() {
    let expr = parse_expr("1 + 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Add, .. }));
}

#[test]
fn test_infix_sub() {
    let expr = parse_expr("1 - 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Sub, .. }));
}

#[test]
fn test_infix_mul() {
    let expr = parse_expr("2 * 3");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Mul, .. }));
}

#[test]
fn test_infix_div() {
    let expr = parse_expr("6 / 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Div, .. }));
}

#[test]
fn test_infix_mod() {
    let expr = parse_expr("10 % 3");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Mod, .. }));
}

// ============================================================================
// 比较运算 — spec §4.2 优先级 7
// ============================================================================

#[test]
fn test_infix_eq() {
    let expr = parse_expr("a == b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Eq, .. }));
}

#[test]
fn test_infix_neq() {
    let expr = parse_expr("a != b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Neq, .. }));
}

#[test]
fn test_infix_lt() {
    let expr = parse_expr("a < b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Lt, .. }));
}

#[test]
fn test_infix_le() {
    let expr = parse_expr("a <= b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Le, .. }));
}

#[test]
fn test_infix_gt() {
    let expr = parse_expr("a > b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Gt, .. }));
}

#[test]
fn test_infix_ge() {
    let expr = parse_expr("a >= b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Ge, .. }));
}

// ============================================================================
// 范围运算符
// ============================================================================

#[test]
fn test_infix_range() {
    let expr = parse_expr("1..5");
    assert!(matches!(
        expr,
        Expr::BinOp {
            op: BinOp::Range,
            ..
        }
    ));
}

// ============================================================================
// 函数调用 — spec §4.2 优先级 1 / §4.3
// ============================================================================

#[test]
fn test_infix_call_no_args() {
    let expr = parse_expr("f()");
    assert!(matches!(expr, Expr::Call { ref args, ref named_args, .. }
        if args.is_empty() && named_args.is_empty()));
}

#[test]
fn test_infix_call_pos_args() {
    let expr = parse_expr("f(a, b, c)");
    if let Expr::Call {
        args, named_args, ..
    } = &expr
    {
        assert_eq!(args.len(), 3);
        assert!(named_args.is_empty());
    } else {
        panic!("Expected Call");
    }
}

#[test]
fn test_infix_call_named_args() {
    // §4.3: 命名参数 f(x=1, y=2)
    let expr = parse_expr("f(x=1, y=2)");
    if let Expr::Call { named_args, .. } = &expr {
        assert_eq!(named_args.len(), 2);
        assert_eq!(named_args[0].0, "x");
        assert_eq!(named_args[1].0, "y");
    } else {
        panic!("Expected Call with named args");
    }
}

#[test]
fn test_infix_call_mixed_args() {
    // §4.3: 位置参数 + 命名参数混合
    let expr = parse_expr("f(a, b, x=1)");
    if let Expr::Call {
        args, named_args, ..
    } = &expr
    {
        assert_eq!(args.len(), 2);
        assert_eq!(named_args.len(), 1);
    }
}

// ============================================================================
// 字段访问 — spec §4.2 优先级 1 / §4.4
// ============================================================================

#[test]
fn test_infix_field_simple() {
    let expr = parse_expr("obj.field");
    assert!(matches!(expr, Expr::FieldAccess { ref field, .. } if field == "field"));
}

#[test]
fn test_infix_field_chain() {
    // §4.4: a.b.c 左结合
    let expr = parse_expr("a.b.c");
    // 应为 FieldAccess(FieldAccess(a, b), c)
    assert!(matches!(expr, Expr::FieldAccess { .. }));
}

// ============================================================================
// 索引访问 — spec §4.2 优先级 1 / §4.5
// ============================================================================

#[test]
fn test_infix_index_simple() {
    let expr = parse_expr("arr[0]");
    assert!(matches!(expr, Expr::Index { .. }));
}

#[test]
fn test_infix_index_nested() {
    // matrix[i][j]
    let expr = parse_expr("m[i][j]");
    assert!(matches!(expr, Expr::Index { .. }));
}

// ============================================================================
// 类型转换 — spec §4.2 优先级 2 / §4.6
// ============================================================================

#[test]
fn test_infix_cast() {
    let expr = parse_expr("42 as Float");
    assert!(matches!(expr, Expr::Cast { .. }));
}

#[test]
fn test_infix_cast_chain() {
    // (x as T) as U
    let expr = parse_expr("x as Int as Float");
    assert!(matches!(expr, Expr::Cast { .. }));
}

// ============================================================================
// 错误传播 — spec §6.9.4
// ============================================================================

#[test]
fn test_infix_try() {
    let expr = parse_expr("x?");
    assert!(matches!(expr, Expr::Try { .. }));
}

#[test]
fn test_infix_try_chain() {
    // x?.y
    let expr = parse_expr("x?.y");
    assert!(matches!(expr, Expr::FieldAccess { .. }));
}

// ============================================================================
// Lambda — spec §4.10
// ============================================================================

#[test]
fn test_infix_lambda_single_param() {
    let expr = parse_expr("x => x + 1");
    assert!(matches!(expr, Expr::Lambda { .. }));
}

#[test]
fn test_infix_lambda_multi_param() {
    let expr = parse_expr("(a, b) => a + b");
    assert!(matches!(expr, Expr::Lambda { .. }));
}

#[test]
fn test_infix_lambda_block_body() {
    let expr = parse_expr("(x) => { return x }");
    assert!(matches!(expr, Expr::Lambda { .. }));
}

#[test]
fn test_infix_lambda_typed_params() {
    let expr = parse_expr("(x: Int) => x + 1");
    assert!(matches!(expr, Expr::Lambda { .. }));
}

// ============================================================================
// 赋值 — spec §4.2 优先级 11
// ============================================================================

#[test]
fn test_infix_assign() {
    let expr = parse_expr("x = 42");
    assert!(matches!(
        expr,
        Expr::BinOp {
            op: BinOp::Assign,
            ..
        }
    ));
}
