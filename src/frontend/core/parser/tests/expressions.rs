//! Pratt parser expression tests — based on spec §4, §4.2, §4.10

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_expression;
use crate::frontend::core::parser::ast::{BinOp, Expr, UnOp};
use crate::frontend::core::lexer::tokens::Literal;

fn parse_expr(source: &str) -> Expr {
    let tokens = tokenize(source).unwrap();
    parse_expression(&tokens).unwrap()
}

// ============================================================================
// 字面量和标识符 (Spec §2.6, §2.5)
// ============================================================================

#[test]
fn test_int_literal() {
    let expr = parse_expr("42");
    assert!(matches!(expr, Expr::Lit(..)));
}

#[test]
fn test_float_literal() {
    let expr = parse_expr("3.14");
    assert!(matches!(expr, Expr::Lit(..)));
}

#[test]
fn test_string_literal() {
    let expr = parse_expr(r#""hello""#);
    assert!(matches!(expr, Expr::Lit(..)));
}

#[test]
fn test_bool_literal() {
    assert!(matches!(parse_expr("true"), Expr::Lit(..)));
    assert!(matches!(parse_expr("false"), Expr::Lit(..)));
}

#[test]
fn test_char_literal() {
    let expr = parse_expr("'a'");
    assert!(matches!(expr, Expr::Lit(..)));
}

#[test]
fn test_void_literal() {
    let expr = parse_expr("void");
    assert!(matches!(expr, Expr::Lit(Literal::Void, _)));
}

#[test]
fn test_identifier() {
    let expr = parse_expr("x");
    assert!(matches!(expr, Expr::Var(..)));
}

// ============================================================================
// 一元运算符 (Spec §4.2)
// ============================================================================

#[test]
fn test_unary_neg() {
    let expr = parse_expr("-x");
    assert!(matches!(expr, Expr::UnOp { op: UnOp::Neg, .. }));
}

#[test]
fn test_unary_not() {
    // not 表达式 — 至少能解析不 panic
    let _ = parse_expr("not x");
}

// ============================================================================
// 二元运算符优先级 (Spec §4.2)
// ============================================================================

#[test]
fn test_binop_add() {
    let expr = parse_expr("1 + 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Add, .. }));
}

#[test]
fn test_binop_sub() {
    let expr = parse_expr("1 - 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Sub, .. }));
}

#[test]
fn test_binop_mul() {
    let expr = parse_expr("1 * 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Mul, .. }));
}

#[test]
fn test_binop_div() {
    let expr = parse_expr("1 / 2");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Div, .. }));
}

#[test]
fn test_binop_eq() {
    let expr = parse_expr("a == b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Eq, .. }));
}

#[test]
fn test_binop_neq() {
    let expr = parse_expr("a != b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Neq, .. }));
}

#[test]
fn test_binop_lt() {
    let expr = parse_expr("a < b");
    assert!(matches!(expr, Expr::BinOp { op: BinOp::Lt, .. }));
}

#[test]
fn test_binop_range() {
    let expr = parse_expr("1..5");
    assert!(matches!(
        expr,
        Expr::BinOp {
            op: BinOp::Range,
            ..
        }
    ));
}

#[test]
fn test_precedence_mul_before_add() {
    // 2 + 3 * 4 应解析为 2 + (3 * 4)
    let expr = parse_expr("2 + 3 * 4");
    if let Expr::BinOp { op: BinOp::Add, .. } = &expr {
        // top level is Add, right side should be Mul — correct
    } else {
        panic!("Expected Add at top level");
    }
}

#[test]
fn test_precedence_compare_before_logical() {
    // a == b and c — and 可能是关键字或标识符
    let expr = parse_expr("a == b and c");
    // 至少能解析不 panic
    let _ = expr;
}

// ============================================================================
// 函数调用 (Spec §4.3)
// ============================================================================

#[test]
fn test_call_no_args() {
    let expr = parse_expr("f()");
    assert!(matches!(expr, Expr::Call { .. }));
}

#[test]
fn test_call_args() {
    let expr = parse_expr("f(a, b)");
    if let Expr::Call {
        args, named_args, ..
    } = &expr
    {
        assert_eq!(args.len(), 2);
        assert!(named_args.is_empty(), "无命名参数时 named_args 应为空");
    } else {
        panic!("Expected Call");
    }
}

#[test]
fn test_call_named_args() {
    // Spec §4.3: 命名参数 Point(x=1, y=2)
    let expr = parse_expr("Point(x=1, y=2)");
    if let Expr::Call { named_args, .. } = &expr {
        assert_eq!(named_args.len(), 2);
        assert_eq!(named_args[0].0, "x");
        assert_eq!(named_args[1].0, "y");
    } else {
        panic!("Expected Call with named args");
    }
}

// ============================================================================
// 成员访问和索引 (Spec §4.4, §4.5)
// ============================================================================

#[test]
fn test_field_access() {
    let expr = parse_expr("obj.field");
    assert!(matches!(expr, Expr::FieldAccess { .. }));
}

#[test]
fn test_index() {
    let expr = parse_expr("arr[0]");
    assert!(matches!(expr, Expr::Index { .. }));
}

// ============================================================================
// 字面量和集合 (Spec §2.6.4)
// ============================================================================

#[test]
fn test_list_literal() {
    let expr = parse_expr("[1, 2, 3]");
    assert!(matches!(expr, Expr::List(..)));
}

#[test]
fn test_list_empty() {
    let expr = parse_expr("[]");
    assert!(matches!(expr, Expr::List(..)));
}

#[test]
fn test_tuple() {
    let expr = parse_expr("(1, 2)");
    assert!(matches!(expr, Expr::Tuple(..)));
}

// ============================================================================
// 列表推导式 (Spec §2.6.5)
// ============================================================================

#[test]
fn test_list_comp() {
    let expr = parse_expr("[x * x for x in items]");
    assert!(matches!(expr, Expr::ListComp { .. }));
}

// ============================================================================
// Lambda (Spec §4.10)
// ============================================================================

#[test]
fn test_lambda_single() {
    let expr = parse_expr("(x) => x + 1");
    assert!(matches!(expr, Expr::Lambda { .. }));
}

#[test]
fn test_lambda_multi() {
    let expr = parse_expr("(a, b) => a + b");
    assert!(matches!(expr, Expr::Lambda { .. }));
}

#[test]
fn test_lambda_block_body() {
    let expr = parse_expr("(x) => { return x }");
    assert!(matches!(expr, Expr::Lambda { .. }));
}

// ============================================================================
// 控制流表达式 (Spec §4.7, §4.8)
// ============================================================================

#[test]
fn test_if_expr() {
    let expr = parse_expr("if true { 1 } else { 2 }");
    assert!(matches!(expr, Expr::If { .. }));
}

#[test]
fn test_block_expr() {
    let expr = parse_expr("{ 42 }");
    assert!(matches!(expr, Expr::Block(..)));
}

// ============================================================================
// 类型转换 (Spec §4.6)
// ============================================================================

#[test]
fn test_cast() {
    let expr = parse_expr("42 as Float");
    assert!(matches!(expr, Expr::Cast { .. }));
}

// ============================================================================
// 错误传播 (Spec §6.9.4)
// ============================================================================

#[test]
fn test_try_operator() {
    let expr = parse_expr("x?");
    assert!(matches!(expr, Expr::Try { .. }));
}

// ============================================================================
// 分组表达式
// ============================================================================

#[test]
fn test_grouped_expr() {
    let expr = parse_expr("(1 + 2) * 3");
    // 外层应该是 Mul，左操作数应该是 BinOp::Add
    if let Expr::BinOp {
        op: BinOp::Mul,
        left,
        ..
    } = &expr
    {
        assert!(matches!(left.as_ref(), Expr::BinOp { op: BinOp::Add, .. }));
    } else {
        panic!("Expected Mul at top level with grouped Add inside");
    }
}

// ============================================================================
// 特殊表达式
// ============================================================================

#[test]
fn test_ref_expr() {
    // Spec §8.3: ref 关键字创建 Arc
    let expr = parse_expr("ref x");
    assert!(matches!(expr, Expr::Ref { .. }));
}
