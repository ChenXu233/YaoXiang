//! `spawn` 放置检查测试 — 基于语言规范 §6.9
//!
//! §6.9: 并作函数与注解
//!
//! 测试目标：`check_spawn_placement` 公共 API
//! RFC-001/008: `spawn { ... }` 仅在 `@block` 作用域内有效。

use crate::frontend::core::parser::ast::{Block, EvalMode, Expr, Module, Stmt, StmtKind};
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

/// 构造 eval 表达式语句（设置作用域模式）
fn eval_stmt(
    mode: EvalMode,
    body_stmts: Vec<Stmt>,
) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Eval {
            mode,
            body: Box::new(make_block(body_stmts, None)),
            span: Span::dummy(),
        })),
        span: Span::dummy(),
    }
}

/// 构造表达式语句（包装任意 Expr）
fn expr_stmt(expr: Expr) -> Stmt {
    Stmt {
        kind: StmtKind::Expr(Box::new(expr)),
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

#[test]
fn test_spawn_inside_block_eval_is_valid() {
    // Arrange: spawn 出现在 @block 作用域内，应通过
    let module = Module {
        items: vec![eval_stmt(EvalMode::Block, vec![spawn_stmt(vec![])])],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert!(
        diagnostics.is_empty(),
        "spawn 在 @block 作用域内应合法, 但产生了 {} 个诊断",
        diagnostics.len()
    );
}

#[test]
fn test_spawn_in_block_binding_body_is_valid() {
    // Arrange: spawn 出现在带 @block eval 的 binding 函数体内
    let module = Module {
        items: vec![Stmt {
            kind: StmtKind::Binding {
                name: "worker".to_string(),
                type_name: None,
                method_type: None,
                is_pub: false,
                params: vec![],
                body: (vec![spawn_stmt(vec![])], None),
                generic_params: vec![],
                type_annotation: None,
                eval: Some(EvalMode::Block),
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
        "spawn 在 @block binding 函数体内应合法, 但产生了 {} 个诊断",
        diagnostics.len()
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_spawn_at_module_level_reports_error() {
    // Arrange: spawn 直接出现在模块顶层（默认 eval mode 为 Auto），应报错
    let module = Module {
        items: vec![spawn_stmt(vec![])],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        1,
        "模块顶层 spawn 应产生恰好 1 个诊断, 实际: {}",
        diagnostics.len()
    );
    assert!(
        diagnostics[0].code.contains("E1080"),
        "诊断错误码应包含 E1080, 实际: {}",
        diagnostics[0].code
    );
}

#[test]
fn test_spawn_inside_auto_eval_reports_error() {
    // Arrange: spawn 出现在 @auto 作用域内，应报错
    let module = Module {
        items: vec![eval_stmt(EvalMode::Auto, vec![spawn_stmt(vec![])])],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        1,
        "@auto 作用域内 spawn 应产生恰好 1 个诊断, 实际: {}",
        diagnostics.len()
    );
    assert!(
        diagnostics[0].code.contains("E1080"),
        "诊断错误码应包含 E1080, 实际: {}",
        diagnostics[0].code
    );
}

#[test]
fn test_spawn_inside_eager_eval_reports_error() {
    // Arrange: spawn 出现在 @eager 作用域内，应报错
    let module = Module {
        items: vec![eval_stmt(EvalMode::Eager, vec![spawn_stmt(vec![])])],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        1,
        "@eager 作用域内 spawn 应产生恰好 1 个诊断, 实际: {}",
        diagnostics.len()
    );
    assert!(
        diagnostics[0].code.contains("E1080"),
        "诊断错误码应包含 E1080, 实际: {}",
        diagnostics[0].code
    );
}

#[test]
fn test_spawn_in_auto_binding_body_reports_error() {
    // Arrange: spawn 出现在默认（无 eval）binding 函数体内
    // 函数边界重置 eval stack，无 eval 时默认为 Auto
    let module = Module {
        items: vec![Stmt {
            kind: StmtKind::Binding {
                name: "fn_default".to_string(),
                type_name: None,
                method_type: None,
                is_pub: false,
                params: vec![],
                body: (vec![spawn_stmt(vec![])], None),
                generic_params: vec![],
                type_annotation: None,
                eval: None,
            },
            span: Span::dummy(),
        }],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        1,
        "默认 eval binding 内 spawn 应报错, 实际诊断数: {}",
        diagnostics.len()
    );
    assert!(
        diagnostics[0].code.contains("E1080"),
        "诊断错误码应包含 E1080, 实际: {}",
        diagnostics[0].code
    );
}

#[test]
fn test_multiple_spawns_in_wrong_scope_report_multiple_errors() {
    // Arrange: 同一模块中两个 spawn 都在错误作用域
    let module = Module {
        items: vec![
            eval_stmt(EvalMode::Auto, vec![spawn_stmt(vec![])]),
            spawn_stmt(vec![]),
        ],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        2,
        "两个错误位置的 spawn 应产生 2 个诊断, 实际: {}",
        diagnostics.len()
    );
    assert!(
        diagnostics.iter().all(|d| d.code.contains("E1080")),
        "所有诊断错误码均应为 E1080"
    );
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_spawn_inside_fn_def_resets_eval_to_auto() {
    // Arrange: @block 作用域内包含函数定义，函数定义内 spawn 应报错
    // 因为 FnDef 创建新的 function boundary，重置 eval stack
    let module = Module {
        items: vec![eval_stmt(
            EvalMode::Block,
            vec![expr_stmt(Expr::FnDef {
                name: "inner".to_string(),
                params: vec![],
                return_type: None,
                body: Box::new(make_block(vec![spawn_stmt(vec![])], None)),
                is_async: false,
                span: Span::dummy(),
            })],
        )],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        1,
        "FnDef 内部重置 eval，spawn 应报错, 实际诊断数: {}",
        diagnostics.len()
    );
}

#[test]
fn test_spawn_inside_lambda_resets_eval_to_auto() {
    // Arrange: @block 作用域内包含 lambda，lambda 内 spawn 应报错
    // Lambda 同样创建 function boundary
    let module = Module {
        items: vec![eval_stmt(
            EvalMode::Block,
            vec![expr_stmt(Expr::Lambda {
                params: vec![],
                body: Box::new(make_block(vec![spawn_stmt(vec![])], None)),
                span: Span::dummy(),
            })],
        )],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        1,
        "Lambda 内部重置 eval，spawn 应报错, 实际诊断数: {}",
        diagnostics.len()
    );
}

#[test]
fn test_nested_spawn_in_block_scope_reports_no_error() {
    // Arrange: @block 作用域内的 spawn 块内再嵌套 spawn
    // 内层 spawn 仍处于 @block eval，应合法
    let module = Module {
        items: vec![eval_stmt(
            EvalMode::Block,
            vec![spawn_stmt(vec![spawn_stmt(vec![])])],
        )],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert!(
        diagnostics.is_empty(),
        "嵌套 spawn 在 @block 作用域内均应合法, 但产生了 {} 个诊断",
        diagnostics.len()
    );
}

#[test]
fn test_nested_spawn_in_auto_scope_reports_errors_for_both() {
    // Arrange: @auto 作用域内 spawn 又嵌套 spawn，两个都应报错
    let module = Module {
        items: vec![eval_stmt(
            EvalMode::Auto,
            vec![spawn_stmt(vec![spawn_stmt(vec![])])],
        )],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        2,
        "两层嵌套 spawn 在 @auto 作用域内均应报错, 实际诊断数: {}",
        diagnostics.len()
    );
}

#[test]
fn test_spawn_in_for_body_reports_error_when_not_in_block() {
    // Arrange: for 循环体内的 spawn，默认 eval 为 Auto，应报错
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

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        1,
        "for 循环体内默认 eval 为 Auto，spawn 应报错, 实际诊断数: {}",
        diagnostics.len()
    );
}

#[test]
fn test_spawn_in_if_body_under_block_eval_is_valid() {
    // Arrange: @block 作用域内 if 分支中的 spawn 应合法
    let module = Module {
        items: vec![eval_stmt(
            EvalMode::Block,
            vec![Stmt {
                kind: StmtKind::If {
                    condition: Box::new(Expr::Lit(
                        crate::frontend::core::lexer::tokens::Literal::Bool(true),
                        Span::dummy(),
                    )),
                    then_branch: Box::new(make_block(vec![spawn_stmt(vec![])], None)),
                    elif_branches: vec![],
                    else_branch: None,
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            }],
        )],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert!(
        diagnostics.is_empty(),
        "@block 作用域内 if 分支中的 spawn 应合法, 但产生了 {} 个诊断",
        diagnostics.len()
    );
}

#[test]
fn test_eval_mode_switch_affects_spawn_correctly() {
    // Arrange: @block 包含 spawn（合法），紧接着 @auto 包含 spawn（非法）
    let module = Module {
        items: vec![
            eval_stmt(EvalMode::Block, vec![spawn_stmt(vec![])]),
            eval_stmt(EvalMode::Auto, vec![spawn_stmt(vec![])]),
        ],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert: 只有 @auto 内的 spawn 应报错
    assert_eq!(
        diagnostics.len(),
        1,
        "@block 内 spawn 合法、@auto 内 spawn 应报错, 实际诊断数: {}",
        diagnostics.len()
    );
}

#[test]
fn test_spawn_with_nested_eval_override_in_auto_reports_error() {
    // Arrange: 在默认作用域（Auto）中用 @eager 包裹，eager 内 spawn 应报错
    // @eager 不是 @block，spawn 仍非法
    let module = Module {
        items: vec![eval_stmt(EvalMode::Eager, vec![spawn_stmt(vec![])])],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        1,
        "@eager 内 spawn 应报错, 实际诊断数: {}",
        diagnostics.len()
    );
}

#[test]
fn test_spawn_body_contents_are_still_checked() {
    // Arrange: @auto 内的 spawn，其 body 内又有一个 spawn（在 @auto 子作用域下）
    // 两个 spawn 都应报错，说明 checker 递归检查了 spawn 的 body
    let inner = spawn_stmt(vec![]);
    let outer = eval_stmt(EvalMode::Auto, vec![spawn_stmt(vec![inner])]);
    let module = Module {
        items: vec![outer],
        span: Span::dummy(),
    };

    // Act
    let diagnostics = check_spawn_placement(&module);

    // Assert
    assert_eq!(
        diagnostics.len(),
        2,
        "spawn body 内的 spawn 也应被检查并报错, 实际诊断数: {}",
        diagnostics.len()
    );
}
