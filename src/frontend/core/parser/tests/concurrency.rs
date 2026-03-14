//! Parser tests for RFC-001/008 concurrency syntax.

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::{parse, parse_expression};
use crate::frontend::core::parser::ast::{EvalMode, Expr, StmtKind};

#[test]
fn test_parse_spawn_expr() {
    let tokens = tokenize("spawn { }").unwrap();
    let expr = parse_expression(&tokens).unwrap();
    assert!(matches!(expr, Expr::Spawn { .. }));
}

#[test]
fn test_parse_eval_block_expr_with_spawn() {
    let tokens = tokenize("@block { spawn { } }").unwrap();
    let expr = parse_expression(&tokens).unwrap();
    match expr {
        Expr::Eval { mode, body, .. } => {
            assert_eq!(mode, EvalMode::Block);
            let has_spawn_stmt = body.stmts.iter().any(|s| {
                matches!(
                    &s.kind,
                    StmtKind::Expr(inner) if matches!(inner.as_ref(), Expr::Spawn { .. })
                )
            });
            let has_spawn_expr = body
                .expr
                .as_ref()
                .is_some_and(|e| matches!(e.as_ref(), Expr::Spawn { .. }));
            assert!(
                has_spawn_stmt || has_spawn_expr,
                "expected spawn inside @block body"
            );
        }
        other => panic!("expected Expr::Eval, got {other:?}"),
    }
}

#[test]
fn test_parse_fn_eval_annotation() {
    let source = "main: () -> Void @block = () => { }";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        crate::frontend::core::parser::ast::StmtKind::Fn { eval, .. } => {
            assert_eq!(*eval, Some(EvalMode::Block));
        }
        other => panic!("expected StmtKind::Fn, got {other:?}"),
    }
}
