//! `spawn` 放置检查测试 — 基于语言规范 §6.9
//!
//! §6.9: 并作函数与注解
//!
//! 测试目标：`check_spawn_placement` 公共 API
//! Phase 1: `@block` 限制已移除，spawn 现在可以出现在任何位置。

use crate::frontend::core::parser::ast::{Block, Expr, Module, Stmt, StmtKind};
use crate::frontend::core::typecheck::spawn_placement::check_spawn_placement;
use crate::util::span::Span;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// 构造空模块
fn empty_module() -> Module {
    Module {
        items: vec![],
        span: Span::dummy(),
    }
}

/// 构造一个 Block
fn make_block(
    stmts: Vec<Stmt>,
    expr: Option<Box<Expr>>,
) -> Block {
    Block {
        stmts,
        expr,
        span: Span::dummy(),
    }
}

/// 构造 spawn 表达式语句
fn spawn_stmt(body_stmts: Vec<Stmt>) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Spawn {
            body: Box::new(make_block(body_stmts, None)),
            span: Span::dummy(),
        })),
        span: Span::dummy(),
    }
}

// ===================================================================
// Happy path 测试
// ===================================================================

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
    // Arrange: 模块仅包含变量声明，无 spawn
    let module = Module {
        items: vec![Stmt {
            kind: StmtKind::Var {
                name: "x".to_string(),
                name_span: Span::dummy(),
                type_annotation: None,
                initializer: None,
                is_mut: false,
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
        "不含 spawn 的模块不应产生诊断, 实际数量: {}",
        diagnostics.len()
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_spawn_at_module_level_allowed() {
    // Phase 1: spawn 现在可以出现在任何位置（@block 限制已移除）
    let module = Module {
        items: vec![spawn_stmt(vec![])],
        span: Span::dummy(),
    };

    let diagnostics = check_spawn_placement(&module);

    assert!(
        diagnostics.is_empty(),
        "Phase 1: spawn 现在可以出现在模块顶层, 实际诊断数: {}",
        diagnostics.len()
    );
}

#[test]
fn test_spawn_in_for_body_allowed() {
    // Phase 1: spawn 现在可以出现在任何位置（@block 限制已移除）
    let module = Module {
        items: vec![Stmt {
            kind: StmtKind::For {
                var: "i".to_string(),
                var_span: Span::dummy(),
                var_mut: false,
                iterable: Box::new(Expr::Var("items".to_string(), Span::dummy())),
                body: Box::new(make_block(vec![spawn_stmt(vec![])], None)),
                label: None,
            },
            span: Span::dummy(),
        }],
        span: Span::dummy(),
    };

    let diagnostics = check_spawn_placement(&module);

    assert!(
        diagnostics.is_empty(),
        "Phase 1: spawn 现在可以出现在 for 循环体内, 实际诊断数: {}",
        diagnostics.len()
    );
}
