//! 表达式格式化处理器测试
//!
//! 对应 formatter 规范 §3, §7, §8, §10, §11, §12, §13, §17, §18

use crate::formatter::handlers::expr::{
    format_binop, format_block, format_call, format_dict, format_expr, format_list, format_literal,
    format_params,
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
    let ctx = default_ctx();
    assert_eq!(format_literal(&lit, &ctx), "42");
}

#[test]
fn test_format_literal_float() {
    let lit = Literal::Float(3.14);
    let ctx = default_ctx();
    assert_eq!(format_literal(&lit, &ctx), "3.14");
}

#[test]
fn test_format_literal_float_no_decimal() {
    let lit = Literal::Float(42.0);
    let ctx = default_ctx();
    let result = format_literal(&lit, &ctx);
    assert!(
        result.contains('.'),
        "Float should have decimal point: {}",
        result
    );
}

#[test]
fn test_format_literal_bool() {
    let ctx = default_ctx();
    assert_eq!(format_literal(&Literal::Bool(true), &ctx), "true");
    assert_eq!(format_literal(&Literal::Bool(false), &ctx), "false");
}

#[test]
fn test_format_literal_string() {
    let lit = Literal::String("hello".to_string());
    let ctx = default_ctx();
    assert_eq!(format_literal(&lit, &ctx), "\"hello\"");
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
    let ctx = default_ctx();
    assert_eq!(format_literal(&lit, &ctx), r#""say \"hello\"""#);
}

#[test]
fn test_format_literal_string_escapes_backslash() {
    let lit = Literal::String("path\\to\\file".to_string());
    let ctx = default_ctx();
    assert_eq!(format_literal(&lit, &ctx), r#""path\\to\\file""#);
}

#[test]
fn test_format_literal_char_escapes() {
    let lit = Literal::Char('\'');
    let ctx = default_ctx();
    assert_eq!(format_literal(&lit, &ctx), r#"'\''"#);
}

// === §8.3 单引号模式 ===

#[test]
fn test_format_literal_string_single_quote() {
    let lit = Literal::String("hello".to_string());
    let ctx = FormatContext::new(FormatOptions {
        single_quote: true,
        ..Default::default()
    });
    assert_eq!(format_literal(&lit, &ctx), "'hello'");
}

#[test]
fn test_format_literal_string_single_quote_with_escapes() {
    let lit = Literal::String("say 'hello'".to_string());
    let ctx = FormatContext::new(FormatOptions {
        single_quote: true,
        ..Default::default()
    });
    assert_eq!(format_literal(&lit, &ctx), r#"'say \'hello\''"#);
}

// === §6.3 空代码块 ===

#[test]
fn test_format_empty_block() {
    let ctx = default_ctx();
    let block = Block {
        stmts: vec![],
        expr: None,
        span: Span::dummy(),
    };
    let result = format_block(&block, &ctx, &default_source_map());
    assert_eq!(result, "{}");
}

// === §6.2 单行代码块 ===

#[test]
fn test_format_single_stmt_block_inline() {
    let ctx = default_ctx();
    let block = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(Expr::Lit(Literal::Int(42), Span::dummy()))),
            span: Span::dummy(),
        }],
        expr: None,
        span: Span::dummy(),
    };
    let result = format_block(&block, &ctx, &default_source_map());
    assert_eq!(result, "{ 42 }");
}

#[test]
fn test_format_single_stmt_block_multiline_when_long() {
    let ctx = FormatContext::new(FormatOptions {
        line_width: 20, // 很小的行宽
        ..Default::default()
    });
    let long_name = "very_long_variable_name_that_exceeds_line_width";
    let block = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(Expr::Var(long_name.to_string(), Span::dummy()))),
            span: Span::dummy(),
        }],
        expr: None,
        span: Span::dummy(),
    };
    let result = format_block(&block, &ctx, &default_source_map());
    // 应该是多行格式
    assert!(
        result.contains('\n'),
        "Long block should be multiline: {}",
        result
    );
}

// === §4.2 参数列表换行 ===

#[test]
fn test_format_params_short() {
    let ctx = default_ctx();
    let params = vec![
        Param {
            name: "x".to_string(),
            ty: Some(Type::Int(64)),
            is_mut: false,
            span: Span::dummy(),
        },
        Param {
            name: "y".to_string(),
            ty: Some(Type::Int(64)),
            is_mut: false,
            span: Span::dummy(),
        },
    ];
    let result = format_params(&params, &ctx, &default_source_map());
    assert_eq!(result, "(x: i64, y: i64)");
}

#[test]
fn test_format_params_long_wraps() {
    let ctx = FormatContext::new(FormatOptions {
        line_width: 30, // 很小的行宽
        ..Default::default()
    });
    let params = vec![
        Param {
            name: "very_long_param_name".to_string(),
            ty: Some(Type::Int(64)),
            is_mut: false,
            span: Span::dummy(),
        },
        Param {
            name: "another_long_param".to_string(),
            ty: Some(Type::Int(64)),
            is_mut: false,
            span: Span::dummy(),
        },
    ];
    let result = format_params(&params, &ctx, &default_source_map());
    // 应该是多行格式，带尾随逗号
    assert!(result.contains('\n'), "Long params should wrap: {}", result);
    assert!(
        result.contains(",\n"),
        "Should have trailing comma: {}",
        result
    );
}

// === §11 Match 表达式 ===

#[test]
fn test_format_match_basic() {
    let match_expr = Expr::Match {
        expr: Box::new(Expr::Var("x".to_string(), Span::dummy())),
        arms: vec![
            MatchArm {
                pattern: Pattern::Literal(Literal::Int(1)),
                body: Block {
                    stmts: vec![],
                    expr: Some(Box::new(Expr::Lit(
                        Literal::String("one".to_string()),
                        Span::dummy(),
                    ))),
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            },
            MatchArm {
                pattern: Pattern::Wildcard,
                body: Block {
                    stmts: vec![],
                    expr: Some(Box::new(Expr::Lit(
                        Literal::String("other".to_string()),
                        Span::dummy(),
                    ))),
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            },
        ],
        span: Span::dummy(),
    };
    let result = format_expr(&match_expr, &default_ctx(), &default_source_map());
    assert!(
        result.contains("match x {"),
        "Expected match format: {}",
        result
    );
    assert!(result.contains("1 =>"), "Expected arm pattern: {}", result);
    assert!(result.contains("_ =>"), "Expected wildcard arm: {}", result);
}

// === §13 F-String ===

#[test]
fn test_format_fstring_simple() {
    let expr = Expr::FString {
        segments: vec![
            FStringSegment::Text("Hello, ".to_string()),
            FStringSegment::Interpolation {
                expr: Box::new(Expr::Var("name".to_string(), Span::dummy())),
                format_spec: None,
            },
        ],
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &default_ctx(), &default_source_map());
    assert_eq!(result, r#"f"Hello, {name}""#);
}

#[test]
fn test_format_fstring_with_spec() {
    let expr = Expr::FString {
        segments: vec![FStringSegment::Interpolation {
            expr: Box::new(Expr::Var("value".to_string(), Span::dummy())),
            format_spec: Some(".2f".to_string()),
        }],
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &default_ctx(), &default_source_map());
    assert_eq!(result, r#"f"{value:.2f}""#);
}

// === §17 Try 操作符 ===

#[test]
fn test_format_try_operator() {
    let expr = Expr::Try {
        expr: Box::new(Expr::Call {
            func: Box::new(Expr::Var("foo".to_string(), Span::dummy())),
            args: vec![],
            named_args: vec![],
            span: Span::dummy(),
        }),
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &default_ctx(), &default_source_map());
    assert_eq!(result, "foo()?");
}

// === §18 Unsafe 块 ===

#[test]
fn test_format_unsafe_block() {
    let expr = Expr::Unsafe {
        body: Box::new(Block {
            stmts: vec![],
            expr: Some(Box::new(Expr::Var("x".to_string(), Span::dummy()))),
            span: Span::dummy(),
        }),
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &default_ctx(), &default_source_map());
    assert!(
        result.starts_with("unsafe {"),
        "Expected unsafe block: {}",
        result
    );
}

// === §3.5 变量引用 ===

#[test]
fn test_format_var_simple() {
    let ctx = default_ctx();
    let expr = Expr::Var("my_variable".to_string(), Span::dummy());
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "my_variable");
}

#[test]
fn test_format_var_camel_case() {
    let ctx = default_ctx();
    let expr = Expr::Var("camelCaseName".to_string(), Span::dummy());
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "camelCaseName");
}

// === §5.6 Return 语句 ===

#[test]
fn test_format_return_with_value() {
    let ctx = default_ctx();
    let expr = Expr::Return(
        Some(Box::new(Expr::Lit(Literal::Int(42), Span::dummy()))),
        Span::dummy(),
    );
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "return 42");
}

#[test]
fn test_format_return_empty() {
    let ctx = default_ctx();
    let expr = Expr::Return(None, Span::dummy());
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "return");
}

#[test]
fn test_format_return_expression() {
    let ctx = default_ctx();
    let expr = Expr::Return(
        Some(Box::new(Expr::BinOp {
            op: BinOp::Add,
            left: Box::new(Expr::Var("x".to_string(), Span::dummy())),
            right: Box::new(Expr::Var("y".to_string(), Span::dummy())),
            span: Span::dummy(),
        })),
        Span::dummy(),
    );
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "return x + y");
}

// === §5.7 Break 语句 ===

#[test]
fn test_format_break_simple() {
    let ctx = default_ctx();
    let expr = Expr::Break(None, Span::dummy());
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "break");
}

#[test]
fn test_format_break_with_label() {
    let ctx = default_ctx();
    let expr = Expr::Break(Some("outer".to_string()), Span::dummy());
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "break outer");
}

// === §5.8 Continue 语句 ===

#[test]
fn test_format_continue_simple() {
    let ctx = default_ctx();
    let expr = Expr::Continue(None, Span::dummy());
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "continue");
}

#[test]
fn test_format_continue_with_label() {
    let ctx = default_ctx();
    let expr = Expr::Continue(Some("outer".to_string()), Span::dummy());
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "continue outer");
}

// === §11.4 元组 ===

#[test]
fn test_format_tuple_simple() {
    let ctx = default_ctx();
    let expr = Expr::Tuple(
        vec![
            Expr::Lit(Literal::Int(1), Span::dummy()),
            Expr::Lit(Literal::String("hello".to_string()), Span::dummy()),
            Expr::Lit(Literal::Bool(true), Span::dummy()),
        ],
        Span::dummy(),
    );
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, r#"(1, "hello", true)"#);
}

#[test]
fn test_format_tuple_single() {
    let ctx = default_ctx();
    let expr = Expr::Tuple(
        vec![Expr::Lit(Literal::Int(42), Span::dummy())],
        Span::dummy(),
    );
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "(42)");
}

#[test]
fn test_format_tuple_empty() {
    let ctx = default_ctx();
    let expr = Expr::Tuple(vec![], Span::dummy());
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "()");
}

// === §11.5 索引访问 ===

#[test]
fn test_format_index_simple() {
    let ctx = default_ctx();
    let expr = Expr::Index {
        expr: Box::new(Expr::Var("arr".to_string(), Span::dummy())),
        index: Box::new(Expr::Lit(Literal::Int(0), Span::dummy())),
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "arr[0]");
}

#[test]
fn test_format_index_nested() {
    let ctx = default_ctx();
    let expr = Expr::Index {
        expr: Box::new(Expr::Index {
            expr: Box::new(Expr::Var("matrix".to_string(), Span::dummy())),
            index: Box::new(Expr::Var("i".to_string(), Span::dummy())),
            span: Span::dummy(),
        }),
        index: Box::new(Expr::Var("j".to_string(), Span::dummy())),
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "matrix[i][j]");
}

// === §11.6 字段访问 ===

#[test]
fn test_format_field_access_simple() {
    let ctx = default_ctx();
    let expr = Expr::FieldAccess {
        expr: Box::new(Expr::Var("obj".to_string(), Span::dummy())),
        field: "field".to_string(),
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "obj.field");
}

#[test]
fn test_format_field_access_chain() {
    let ctx = default_ctx();
    let expr = Expr::FieldAccess {
        expr: Box::new(Expr::FieldAccess {
            expr: Box::new(Expr::Var("obj".to_string(), Span::dummy())),
            field: "method1".to_string(),
            span: Span::dummy(),
        }),
        field: "method2".to_string(),
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "obj.method1.method2");
}

// === §17 Ref 关键字 ===

#[test]
fn test_format_ref_simple() {
    let ctx = default_ctx();
    let expr = Expr::Ref {
        expr: Box::new(Expr::Var("value".to_string(), Span::dummy())),
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "ref value");
}

#[test]
fn test_format_ref_expression() {
    let ctx = default_ctx();
    let expr = Expr::Ref {
        expr: Box::new(Expr::Call {
            func: Box::new(Expr::Var("foo".to_string(), Span::dummy())),
            args: vec![],
            named_args: vec![],
            span: Span::dummy(),
        }),
        span: Span::dummy(),
    };
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "ref foo()");
}

// === §E3 错误恢复 ===

#[test]
fn test_format_error_placeholder() {
    let ctx = default_ctx();
    let expr = Expr::Error(Span::dummy());
    let result = format_expr(&expr, &ctx, &default_source_map());
    assert_eq!(result, "/* error */");
}
