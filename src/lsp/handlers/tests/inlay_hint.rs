//! LSP 幽灵提示处理器测试
//!
//! 测试覆盖：
//! - 类型推断
//! - 常量计算

use lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams};

use crate::frontend::core::parser::ast::{BinOp, Expr, Literal, StmtKind};
use crate::lsp::handlers::inlay_hint::{
    handle_inlay_hint, simple_infer_type, evaluate_constant,
};
use crate::lsp::session::Session;

#[test]
fn test_simple_infer_type() {
    let span = crate::util::span::Span::default();
    let expr = Expr::Lit(Literal::Int(10), span);
    assert_eq!(simple_infer_type(&expr), Some("Int".to_string()));

    let call_expr = Expr::Call {
        func: Box::new(Expr::Var("vec!".to_string(), span)),
        args: vec![],
        named_args: vec![],
        span,
    };
    assert_eq!(simple_infer_type(&call_expr), Some("Vec<_>".to_string()));
}

#[test]
fn test_evaluate_constant() {
    let span = crate::util::span::Span::default();
    let left = Box::new(Expr::Lit(Literal::Int(100), span));
    let right = Box::new(Expr::Lit(Literal::Int(200), span));
    let bin_op = Expr::BinOp {
        op: BinOp::Add,
        left,
        right,
        span,
    };
    assert_eq!(evaluate_constant(&bin_op), Some(300));
}
