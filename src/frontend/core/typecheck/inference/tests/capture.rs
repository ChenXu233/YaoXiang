//! Capture 模块测试 — 基于 RFC-023 闭包捕获模型
//!
//! RFC-023 §2: 捕获模式决策（Copy/Borrow/BorrowMut/Move）
//! RFC-023 §2.4: Dup 类型捕获为 Copy，非 Dup 根据逃逸分析决定
//! RFC-011 §2.4: Dup/Clone 内置 marker trait

use std::collections::HashSet;

use crate::frontend::core::parser::ast::{BinOp, Block, Expr, Param, Stmt, StmtKind};
use crate::frontend::core::typecheck::inference::capture::{
    analyze_captures, analyze_closure_usage, determine_capture_mode, CaptureMode, CaptureUsage,
    ClosureUsage,
};
use crate::frontend::core::types::TraitTable;
use crate::frontend::core::types::mono::{MonoType, StructType};
use crate::util::span::Span;

fn dummy_span() -> Span {
    Span::dummy()
}

/// 构造一个包含 `Fn` 字段的非 Dup 结构体类型 (用于捕获模式测试)。
/// 对应 RFC-009: 不满足 `Dup` trait 的类型在不同场景下的捕获行为。
fn make_non_dup_struct_ty() -> MonoType {
    use std::collections::HashMap;
    let fn_ty = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::Void),
    };
    MonoType::Struct(StructType {
        name: "MyStruct".to_string(),
        fields: vec![("callback".to_string(), fn_ty)],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    })
}

#[test]
fn test_analyze_captures_read_only() {
    // lambda: () => x + y
    // outer scope: {x, y, z}
    let block = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(Expr::BinOp {
                op: BinOp::Add,
                left: Box::new(Expr::Var("x".to_string(), dummy_span())),
                right: Box::new(Expr::Var("y".to_string(), dummy_span())),
                span: dummy_span(),
            })),
            span: dummy_span(),
        }],
        span: dummy_span(),
    };

    let outer: HashSet<String> = ["x", "y", "z"].iter().map(|s| s.to_string()).collect();
    let captures = analyze_captures(&block, &outer);

    assert_eq!(
        captures.len(),
        2,
        "x and y should be captured, z should not"
    );
    let names: HashSet<&str> = captures.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains("x"), "x should be in captures");
    assert!(names.contains("y"), "y should be in captures");
    assert!(
        !names.contains("z"),
        "z should not be captured (not in outer scope)"
    );

    for cap in &captures {
        assert_eq!(
            cap.usage,
            CaptureUsage::Read,
            "all captures should be Read-only"
        );
    }
}

#[test]
fn test_analyze_captures_write() {
    // lambda: () => { x = 42 }
    // outer scope: {x}
    let block = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(Expr::BinOp {
                op: BinOp::Assign,
                left: Box::new(Expr::Var("x".to_string(), dummy_span())),
                right: Box::new(Expr::Lit(
                    crate::frontend::core::lexer::tokens::Literal::Int(42),
                    dummy_span(),
                )),
                span: dummy_span(),
            })),
            span: dummy_span(),
        }],
        span: dummy_span(),
    };

    let outer: HashSet<String> = ["x"].iter().map(|s| s.to_string()).collect();
    let captures = analyze_captures(&block, &outer);

    assert_eq!(captures.len(), 1, "only x should be captured");
    assert_eq!(captures[0].name, "x", "captured variable should be x");
    assert_eq!(
        captures[0].usage,
        CaptureUsage::Write,
        "x is assigned, should be Write"
    );
}

#[test]
fn test_analyze_captures_local_var_not_captured() {
    // lambda: () => { let z = 10; z }
    // outer scope: {x}
    let block = Block {
        stmts: vec![
            Stmt {
                kind: StmtKind::Var {
                    name: "z".to_string(),
                    name_span: dummy_span(),
                    type_annotation: None,
                    initializer: Some(Box::new(Expr::Lit(
                        crate::frontend::core::lexer::tokens::Literal::Int(10),
                        dummy_span(),
                    ))),
                    is_mut: false,
                },
                span: dummy_span(),
            },
            Stmt {
                kind: StmtKind::Expr(Box::new(Expr::Var("z".to_string(), dummy_span()))),
                span: dummy_span(),
            },
        ],
        span: dummy_span(),
    };

    let outer: HashSet<String> = ["x"].iter().map(|s| s.to_string()).collect();
    let captures = analyze_captures(&block, &outer);

    // z 是局部变量，不应被捕获
    assert_eq!(captures.len(), 0, "local variable z should not be captured");
}

#[test]
fn test_analyze_captures_nested_lambda() {
    // outer lambda: () => { (a) => x + a }
    // outer scope: {x}
    let inner_lambda = Expr::Lambda {
        params: vec![Param {
            name: "a".to_string(),
            ty: None,
            is_mut: false,
            span: dummy_span(),
        }],
        body: Box::new(Block {
            stmts: vec![Stmt {
                kind: StmtKind::Expr(Box::new(Expr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(Expr::Var("x".to_string(), dummy_span())),
                    right: Box::new(Expr::Var("a".to_string(), dummy_span())),
                    span: dummy_span(),
                })),
                span: dummy_span(),
            }],
            span: dummy_span(),
        }),
        span: dummy_span(),
    };

    let block = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(inner_lambda)),
            span: dummy_span(),
        }],
        span: dummy_span(),
    };

    let outer: HashSet<String> = ["x"].iter().map(|s| s.to_string()).collect();
    let captures = analyze_captures(&block, &outer);

    assert_eq!(
        captures.len(),
        1,
        "only x from outer scope should be captured"
    );
    assert_eq!(captures[0].name, "x", "captured variable should be x");
    assert_eq!(
        captures[0].usage,
        CaptureUsage::Read,
        "x is only read in nested lambda"
    );
}

#[test]
fn test_determine_capture_mode_dup_inline() {
    let solver = TraitTable::with_std();
    let mode = determine_capture_mode(
        &CaptureUsage::Read,
        &MonoType::Int(64),
        &ClosureUsage::Inline,
        &solver,
    );
    assert_eq!(
        mode,
        CaptureMode::Copy,
        "Int is Dup, inline read should Copy"
    );
}

#[test]
fn test_determine_capture_mode_dup_escaping() {
    let solver = TraitTable::with_std();
    let mode = determine_capture_mode(
        &CaptureUsage::Read,
        &MonoType::Int(64),
        &ClosureUsage::Escaping,
        &solver,
    );
    // Dup 类型即使逃逸也是 Copy
    assert_eq!(
        mode,
        CaptureMode::Copy,
        "Int is Dup, escaping should still Copy"
    );
}

#[test]
fn test_determine_capture_mode_non_dup_read_inline() {
    let solver = TraitTable::with_std();
    let struct_ty = make_non_dup_struct_ty();
    let mode = determine_capture_mode(
        &CaptureUsage::Read,
        &struct_ty,
        &ClosureUsage::Inline,
        &solver,
    );
    assert_eq!(
        mode,
        CaptureMode::Borrow,
        "non-Dup inline read should Borrow"
    );
}

#[test]
fn test_determine_capture_mode_non_dup_write_inline() {
    let solver = TraitTable::with_std();
    let struct_ty = make_non_dup_struct_ty();
    let mode = determine_capture_mode(
        &CaptureUsage::Write,
        &struct_ty,
        &ClosureUsage::Inline,
        &solver,
    );
    assert_eq!(
        mode,
        CaptureMode::BorrowMut,
        "non-Dup inline write should BorrowMut"
    );
}

#[test]
fn test_determine_capture_mode_non_dup_escaping() {
    let solver = TraitTable::with_std();
    let struct_ty = make_non_dup_struct_ty();
    let mode = determine_capture_mode(
        &CaptureUsage::Read,
        &struct_ty,
        &ClosureUsage::Escaping,
        &solver,
    );
    assert_eq!(mode, CaptureMode::Move, "non-Dup escaping should Move");
}

#[test]
fn test_closure_usage_inline() {
    let lambda = Expr::Lambda {
        params: vec![],
        body: Box::new(Block {
            stmts: vec![],
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let usage = analyze_closure_usage(&lambda, None);
    assert_eq!(
        usage,
        ClosureUsage::Inline,
        "lambda with no parent should be Inline"
    );
}

#[test]
fn test_closure_usage_escaping_spawn() {
    let lambda = Expr::Lambda {
        params: vec![],
        body: Box::new(Block {
            stmts: vec![],
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let parent = Expr::Spawn {
        body: Box::new(Block {
            stmts: vec![],
            span: dummy_span(),
        }),
        span: dummy_span(),
    };
    let usage = analyze_closure_usage(&lambda, Some(&parent));
    assert_eq!(
        usage,
        ClosureUsage::Escaping,
        "lambda inside spawn should be Escaping"
    );
}

#[test]
fn test_analyze_captures_no_captures() {
    // lambda: () => 42
    // outer scope: {x, y} — body references nothing
    let block = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(42),
                dummy_span(),
            ))),
            span: dummy_span(),
        }],
        span: dummy_span(),
    };

    let outer: HashSet<String> = ["x", "y"].iter().map(|s| s.to_string()).collect();
    let captures = analyze_captures(&block, &outer);

    assert_eq!(
        captures.len(),
        0,
        "body referencing no outer variables should have no captures"
    );
}

#[test]
fn test_closure_usage_escaping_return() {
    // Build the full parent expression, then extract a reference to the
    // inner lambda so that std::ptr::eq inside analyze_closure_usage sees
    // the same allocation.
    let parent = Expr::Return(
        Some(Box::new(Expr::Lambda {
            params: vec![],
            body: Box::new(Block {
                stmts: vec![],
                span: dummy_span(),
            }),
            span: dummy_span(),
        })),
        dummy_span(),
    );
    if let Expr::Return(Some(ret_expr), _) = &parent {
        let lambda_ref = ret_expr.as_ref();
        let usage = analyze_closure_usage(lambda_ref, Some(&parent));
        assert_eq!(
            usage,
            ClosureUsage::Escaping,
            "lambda inside return should be Escaping"
        );
    } else {
        panic!("expected Return expression");
    }
}

#[test]
fn test_analyze_captures_mixed_read_write() {
    // lambda: () => { x = x + 1 }
    // x is both read (right side) and written (left side)
    let block = Block {
        stmts: vec![Stmt {
            kind: StmtKind::Expr(Box::new(Expr::BinOp {
                op: BinOp::Assign,
                left: Box::new(Expr::Var("x".to_string(), dummy_span())),
                right: Box::new(Expr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(Expr::Var("x".to_string(), dummy_span())),
                    right: Box::new(Expr::Lit(
                        crate::frontend::core::lexer::tokens::Literal::Int(1),
                        dummy_span(),
                    )),
                    span: dummy_span(),
                }),
                span: dummy_span(),
            })),
            span: dummy_span(),
        }],
        span: dummy_span(),
    };

    let outer: HashSet<String> = ["x"].iter().map(|s| s.to_string()).collect();
    let captures = analyze_captures(&block, &outer);

    assert_eq!(captures.len(), 1, "only x should be captured");
    assert_eq!(captures[0].name, "x", "captured variable should be x");
    // Write 取优先（因为 written_vars 先处理）
    assert_eq!(
        captures[0].usage,
        CaptureUsage::Write,
        "Write should take priority over Read"
    );
}

// ============================================================================
// Ref 类型捕获模式测试
// ============================================================================

#[test]
fn test_capture_mode_for_immutable_ref() {
    // &T 是 Dup → 应该 Copy
    let solver = TraitTable::with_std();
    let ref_ty = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Int(64)),
    };
    let mode = determine_capture_mode(&CaptureUsage::Read, &ref_ty, &ClosureUsage::Inline, &solver);
    assert_eq!(mode, CaptureMode::Copy, "&T is Dup, should Copy");
}

#[test]
fn test_capture_mode_for_immutable_ref_escaping() {
    // &T 是 Dup → 即使逃逸也是 Copy
    let solver = TraitTable::with_std();
    let ref_ty = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Int(64)),
    };
    let mode = determine_capture_mode(
        &CaptureUsage::Read,
        &ref_ty,
        &ClosureUsage::Escaping,
        &solver,
    );
    assert_eq!(
        mode,
        CaptureMode::Copy,
        "&T is Dup, escaping should still Copy"
    );
}

#[test]
fn test_capture_mode_for_mutable_ref_inline_read() {
    // &mut T 不是 Dup，内联读取 → Borrow
    let solver = TraitTable::with_std();
    let ref_ty = MonoType::Ref {
        mutable: true,
        inner: Box::new(MonoType::Int(64)),
    };
    let mode = determine_capture_mode(&CaptureUsage::Read, &ref_ty, &ClosureUsage::Inline, &solver);
    assert_eq!(mode, CaptureMode::Borrow, "&mut T inline read → Borrow");
}

#[test]
fn test_capture_mode_for_mutable_ref_inline_write() {
    // &mut T 不是 Dup，内联写入 → BorrowMut
    let solver = TraitTable::with_std();
    let ref_ty = MonoType::Ref {
        mutable: true,
        inner: Box::new(MonoType::Int(64)),
    };
    let mode = determine_capture_mode(
        &CaptureUsage::Write,
        &ref_ty,
        &ClosureUsage::Inline,
        &solver,
    );
    assert_eq!(
        mode,
        CaptureMode::BorrowMut,
        "&mut T inline write → BorrowMut"
    );
}

#[test]
fn test_capture_mode_for_mutable_ref_escaping() {
    // &mut T 不是 Dup，逃逸 → Move
    let solver = TraitTable::with_std();
    let ref_ty = MonoType::Ref {
        mutable: true,
        inner: Box::new(MonoType::Int(64)),
    };
    let mode = determine_capture_mode(
        &CaptureUsage::Read,
        &ref_ty,
        &ClosureUsage::Escaping,
        &solver,
    );
    assert_eq!(mode, CaptureMode::Move, "&mut T escaping → Move");
}
