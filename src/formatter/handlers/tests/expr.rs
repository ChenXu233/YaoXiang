//! 表达式格式化处理器测试
//!
//! 对应 formatter 规范 §3, §7, §8, §10, §12

use crate::formatter::handlers::expr::{
    format_binop, format_call, format_dict, format_expr, format_list, format_literal,
};
use crate::formatter::context::FormatContext;
use crate::formatter::source_map::SourceMap;
use crate::formatter::FormatOptions;
use crate::frontend::core::parser::ast::*;
use crate::util::span::Span;

fn default_ctx() -> FormatContext {
    FormatContext::new(FormatOptions::default())
}

fn default_source_map() -> SourceMap {
    SourceMap::build("")
}

#[test]
fn test_format_literal_int() {
    let lit = Literal::Int(42);
    assert_eq!(format_literal(&lit), "42");
}

#[test]
fn test_format_literal_float() {
    let lit = Literal::Float(3.14);
    assert_eq!(format_literal(&lit), "3.14");
}

#[test]
fn test_format_literal_float_no_decimal() {
    let lit = Literal::Float(42.0);
    let result = format_literal(&lit);
    assert!(
        result.contains('.'),
        "Float should have decimal point: {}",
        result
    );
}

#[test]
fn test_format_literal_bool() {
    assert_eq!(format_literal(&Literal::Bool(true)), "true");
    assert_eq!(format_literal(&Literal::Bool(false)), "false");
}

#[test]
fn test_format_literal_string() {
    let lit = Literal::String("hello".to_string());
    assert_eq!(format_literal(&lit), "\"hello\"");
}

#[test]
fn test_format_binop_add() {
    let ctx = default_ctx();
    let left = Expr::Lit(Literal::Int(1), Span::dummy());
    let right = Expr::Lit(Literal::Int(2), Span::dummy());
    let result = format_binop(&BinOp::Add, &left, &right, &ctx, &default_source_map());
    assert_eq!(result, "1 + 2");
}

#[test]
fn test_format_binop_eq() {
    let ctx = default_ctx();
    let left = Expr::Var("x".to_string(), Span::dummy());
    let right = Expr::Lit(Literal::Int(0), Span::dummy());
    let result = format_binop(&BinOp::Eq, &left, &right, &ctx, &default_source_map());
    assert_eq!(result, "x == 0");
}

#[test]
fn test_format_call_no_args() {
    let ctx = default_ctx();
    let func = Expr::Var("foo".to_string(), Span::dummy());
    let result = format_call(&func, &[], &[], &ctx, &default_source_map());
    assert_eq!(result, "foo()");
}

#[test]
fn test_format_call_with_args() {
    let ctx = default_ctx();
    let func = Expr::Var("add".to_string(), Span::dummy());
    let arg1 = Expr::Lit(Literal::Int(1), Span::dummy());
    let arg2 = Expr::Lit(Literal::Int(2), Span::dummy());
    let result = format_call(&func, &[arg1, arg2], &[], &ctx, &default_source_map());
    assert_eq!(result, "add(1, 2)");
}

#[test]
fn test_format_list_empty() {
    let ctx = default_ctx();
    let result = format_list(&[], &ctx, &default_source_map());
    assert_eq!(result, "[]");
}

#[test]
fn test_format_list_single() {
    let ctx = default_ctx();
    let items = vec![Expr::Lit(Literal::Int(1), Span::dummy())];
    let result = format_list(&items, &ctx, &default_source_map());
    assert_eq!(result, "[1]");
}

#[test]
fn test_format_dict_empty() {
    let ctx = default_ctx();
    let result = format_dict(&[], &ctx, &default_source_map());
    assert_eq!(result, "{}");
}

#[test]
fn test_format_return() {
    let ctx = default_ctx();
    let expr = Expr::Return(
        Some(Box::new(Expr::Lit(Literal::Int(42), Span::dummy()))),
        Span::dummy(),
    );
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "return 42");
}

#[test]
fn test_format_return_none() {
    let ctx = default_ctx();
    let expr = Expr::Return(None, Span::dummy());
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "return");
}

#[test]
fn test_format_cast() {
    let ctx = default_ctx();
    let inner = Expr::Var("x".to_string(), Span::dummy());
    let expr = Expr::Cast {
        expr: Box::new(inner),
        target_type: Type::Int(64),
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "x as i64");
}

#[test]
fn test_format_syntax_error_preserves_content() {
    let source = "let x = ;";
    let result = crate::formatter::format_source(source, &FormatOptions::default());
    assert!(result.is_ok(), "Should not panic on syntax error");
}

#[test]
fn test_format_literal_string_escapes_quotes() {
    let lit = Literal::String("say \"hello\"".to_string());
    assert_eq!(format_literal(&lit), r#""say \"hello\"""#);
}

#[test]
fn test_format_literal_string_escapes_backslash() {
    let lit = Literal::String("path\\to\\file".to_string());
    assert_eq!(format_literal(&lit), r#""path\\to\\file""#);
}

#[test]
fn test_format_literal_char_escapes() {
    let lit = Literal::Char('\'');
    assert_eq!(format_literal(&lit), r#"'\''"#);
}
