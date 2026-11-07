//! 表达式格式化处理器测试
//!
//! 对应 formatter 规范 §3, §4, §7, §8, §10, §11, §12, §13, §17, §18

use crate::formatter::handlers::expr::{
    format_binop, format_block, format_call, format_dict, format_expr, format_fn_signature,
    format_list, format_literal, format_params,
};
use crate::formatter::context::FormatContext;
use crate::formatter::source_map::SourceMap;
use crate::formatter::FormatOptions;
use crate::ast::*;
use crate::util::span::Span;

fn default_ctx() -> FormatContext {
    FormatContext::new(FormatOptions::default())
}

fn default_source_map() -> SourceMap {
    SourceMap::build("")
}

/// Helper: test_format_fn_signature_curried_grouping 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: curried 签名的 `decl_params = [T: Type, x: Int]`。
fn curried_decl_params() -> Vec<Param> {
    let decl_params = vec![
        Param {
            name: "T".to_string(),
            ty: Some(Type::MetaType {
                name_span: Span::dummy(),
                args: vec![],
            }),
            is_mut: false,
            span: Span::dummy(),
        },
        Param {
            name: "x".to_string(),
            ty: Some(Type::Name {
                name: "Int".to_string(),
                span: Span::dummy(),
            }),
            is_mut: false,
            span: Span::dummy(),
        },
    ];
    decl_params
}

/// Fixture: curried 函数类型 `(Type) -> ((Int) -> Int)`。
fn curried_fn_type() -> Type {
    Type::Fn {
        params: vec![Param::from_type(Type::MetaType {
            name_span: Span::dummy(),
            args: vec![],
        })],
        return_type: Box::new(Type::Fn {
            params: vec![Param::from_type(Type::Name {
                name: "Int".to_string(),
                span: Span::dummy(),
            })],
            return_type: Box::new(Type::Name {
                name: "Int".to_string(),
                span: Span::dummy(),
            }),
        }),
    }
}

#[test]
fn test_format_literal_int() {
    let lit = Literal::Int(42);
    let ctx = default_ctx();
    assert_eq!(format_literal(&lit, &ctx), "42");
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
fn test_format_call_no_args() {
    let ctx = default_ctx();
    let func = Expr::Var("foo".to_string(), Span::dummy());
    let result = format_call(&func, &[], &[], &ctx, &default_source_map());
    assert_eq!(result, "foo()");
}

#[test]
fn test_format_list_empty() {
    let ctx = default_ctx();
    let result = format_list(&[], &ctx, &default_source_map());
    assert_eq!(result, "[]");
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
fn test_format_syntax_error_returns_error() {
    let source = "let x = 1";
    let result = crate::formatter::format_source(source, &FormatOptions::default());
    assert!(
        matches!(result, Err(crate::formatter::FormatError::Semantic(_))),
        "语义错误代码应被拒绝，实际返回 {:?}",
        result
    );
}

// === §8.3 单引号模式 ===

// === §6.3 空代码块 ===

#[test]
fn test_format_empty_block() {
    let ctx = default_ctx();
    let block = Block {
        stmts: vec![],
        span: Span::dummy(),
    };
    let result = format_block(&block, &ctx, &default_source_map());
    assert_eq!(result, "{}");
}

// === §6.2 单行代码块 ===

// === §4.2 参数列表换行 ===

#[test]
fn test_format_params_short() {
    let ctx = default_ctx();
    let params = vec![
        Param {
            name: "x".to_string(),
            ty: Some(Type::Name {
                name: "Int64".to_string(),
                span: Span::dummy(),
            }),
            is_mut: false,
            span: Span::dummy(),
        },
        Param {
            name: "y".to_string(),
            ty: Some(Type::Name {
                name: "Int64".to_string(),
                span: Span::dummy(),
            }),
            is_mut: false,
            span: Span::dummy(),
        },
    ];
    let result = format_params(&params, &ctx, &default_source_map());
    assert_eq!(result, "(x: Int64, y: Int64)", "参数类型应原样输出规范名");
}

// === §11 Match 表达式 ===

// === §13 F-String ===

// === §17 Try 操作符 ===

// === §18 Unsafe 块 ===

// === §3.5 变量引用 ===

// === §5.6 Return 语句 ===

// === §5.7 Break 语句 ===

// === §5.8 Continue 语句 ===

// === §11.4 元组 ===

// === §11.5 索引访问 ===

// === §11.6 字段访问 ===

// === §17 Ref 关键字 ===

// === §E3 错误恢复 ===

// === 签名参数约束语义 / curried 分组 ===

#[test]
fn test_format_fn_signature_curried_grouping() {
    // Arrange — curried 泛型函数签名（decl_params 按嵌套 Fn 切分）
    let ctx = default_ctx();
    let decl_params = curried_decl_params();
    let fn_type = curried_fn_type();

    // Act — 按嵌套 Fn 结构切分 decl_params
    let result = format_fn_signature(&decl_params, &fn_type, &ctx, &default_source_map());

    // Assert — 第一组给外层，第二组给内层，单层括号保证幂等
    assert_eq!(
        result, "(T: Type) -> (x: Int) -> Int",
        "curried 签名应按嵌套 Fn 切分 decl_params：got {:?}",
        result
    );
}
