//! spawn 位置检查测试 — 基于 RFC-024 §2.1
//!
//! RFC-024 §2.1: spawn 块可以出现在任何表达式位置

use crate::frontend::core::parser::ast::{Block, Expr, Module, Stmt, StmtKind};
use crate::frontend::core::spawn::placement::check_spawn_placement;
use crate::util::span::Span;

// ============================================================================
// 辅助函数
// ============================================================================

fn empty_module() -> Module {
    Module {
        items: vec![],
        span: Span::dummy(),
    }
}

fn make_block(stmts: Vec<Stmt>) -> Block {
    Block {
        stmts,
        span: Span::dummy(),
    }
}

fn spawn_stmt(body_stmts: Vec<Stmt>) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Spawn {
            body: Box::new(make_block(body_stmts)),
            span: Span::dummy(),
        })),
        span: Span::dummy(),
    }
}

fn var_stmt(name: &str) -> Stmt {
    Stmt {
        kind: StmtKind::Assign {
            target: Box::new(Expr::Var(name.to_string(), Span::dummy())),
            type_annotation: None,
            signature_params: vec![],
            value: None,
            is_pub: false,
            is_mut: false,
            span: Span::dummy(),
        },
        span: Span::dummy(),
    }
}

// ============================================================================
// Happy path
// ============================================================================

#[test]
fn test_empty_module_produces_no_diagnostics() {
    // Arrange
    let module = empty_module();

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert!(
        diagnostics.is_empty(),
        "空模块不应产生任何诊断, 实际数量: {}",
        diagnostics.len()
    );
}

#[test]
fn test_module_without_spawn_produces_no_diagnostics() {
    // Arrange
    let module = Module {
        items: vec![var_stmt("x")],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert!(
        diagnostics.is_empty(),
        "不含 spawn 的模块不应产生诊断, 实际数量: {}",
        diagnostics.len()
    );
}

#[test]
fn test_spawn_at_module_level_allowed() {
    // RFC-024 §2.1: spawn 可以出现在任何位置
    // Arrange
    let module = Module {
        items: vec![spawn_stmt(vec![])],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert!(
        diagnostics.is_empty(),
        "spawn 应允许出现在模块顶层, 实际诊断数: {}",
        diagnostics.len()
    );
}

#[test]
fn test_spawn_in_for_body_allowed() {
    // RFC-024 §2.1: spawn 可以出现在任何位置
    // Arrange
    let module = Module {
        items: vec![Stmt {
            kind: StmtKind::For {
                var: "i".to_string(),
                var_span: Span::dummy(),
                var_mut: false,
                iterable: Box::new(Expr::Var("items".to_string(), Span::dummy())),
                body: Box::new(make_block(vec![spawn_stmt(vec![])])),
                label: None,
            },
            span: Span::dummy(),
        }],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert!(
        diagnostics.is_empty(),
        "spawn 应允许出现在 for 循环体内, 实际诊断数: {}",
        diagnostics.len()
    );
}
