//! LSP 幽灵提示（Inlay Hints）处理器
//!
//! 实现 `textDocument/inlayHint` 功能，提供：
//! - 类型推断提示
//! - 常量值提示
//! - 可变性提示
//! - 所有权消费提示（部分）

use lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams};
use tracing::debug;

use crate::frontend::core::parser::ast::{BinOp, Expr, Literal, StmtKind};
use crate::lsp::locate::span_to_range;
use crate::lsp::session::Session;

/// 处理 textDocument/inlayHint 请求
pub fn handle_inlay_hint(
    session: &Session,
    params: InlayHintParams,
) -> Option<Vec<InlayHint>> {
    let uri = params.text_document.uri.as_str();
    let doc = session.document_store().get(uri)?;

    // 我们需要解析树
    let ast = doc.ast()?;

    let mut hints = Vec::new();

    for stmt in &ast.items {
        if let StmtKind::Assign {
            target,
            type_annotation,
            value,
            is_mut,
            span: stmt_span,
            ..
        } = &stmt.kind
        {
            use crate::frontend::core::parser::ast::Expr;
            let name_span = match target.as_ref() {
                Expr::Var(_, s) => *s,
                _ => *stmt_span,
            };
            let range = span_to_range(&name_span);
            if type_annotation.is_none() {
                if let Some(init) = value {
                    if let Some(inferred_type) = simple_infer_type(init) {
                        hints.push(InlayHint {
                            position: range.end,
                            label: InlayHintLabel::String(format!(": {}", inferred_type)),
                            kind: Some(InlayHintKind::TYPE),
                            text_edits: None,
                            tooltip: None,
                            padding_left: Some(false),
                            padding_right: Some(false),
                            data: None,
                        });
                    }
                }
            }
            if !*is_mut {
                if let Some(init) = value {
                    if let Some(val) = evaluate_constant(init) {
                        if !is_literal(init) {
                            hints.push(InlayHint {
                                position: span_to_range(&get_expr_span(init)).end,
                                label: InlayHintLabel::String(format!(" => {}", val)),
                                kind: None,
                                text_edits: None,
                                tooltip: Some(lsp_types::InlayHintTooltip::String(
                                    "编译期计算的常量值".to_string(),
                                )),
                                padding_left: Some(true),
                                padding_right: Some(false),
                                data: None,
                            });
                        }
                    }
                }
            }
            if *is_mut {
                hints.push(InlayHint {
                    position: range.start,
                    label: InlayHintLabel::String("mut ".to_string()),
                    kind: Some(InlayHintKind::PARAMETER),
                    text_edits: None,
                    tooltip: Some(lsp_types::InlayHintTooltip::String("可变变量".to_string())),
                    padding_left: Some(false),
                    padding_right: Some(true),
                    data: None,
                });
            }
        }
    }

    // TODO: 可以递归遍历 Expr，添加更多提示

    debug!("生成了 {} 个 inlay hints", hints.len());
    Some(hints)
}

fn get_expr_span(expr: &Expr) -> crate::util::span::Span {
    match expr {
        Expr::Lit(_, span) => *span,
        Expr::Var(_, span) => *span,
        Expr::BinOp { span, .. } => *span,
        Expr::UnOp { span, .. } => *span,
        Expr::Call { span, .. } => *span,
        _ => crate::util::span::Span::default(), // 简化，实际应返回正确的 span
    }
}

fn is_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Lit(_, _))
}

fn simple_infer_type(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(Literal::Int(_), _) => Some("Int".to_string()),
        Expr::Lit(Literal::Float(_), _) => Some("Float".to_string()),
        Expr::Lit(Literal::String(_), _) => Some("String".to_string()),
        Expr::Lit(Literal::Bool(_), _) => Some("Bool".to_string()),
        Expr::Call { func, .. } => {
            if let Expr::Var(name, _) = &**func {
                if name == "vec" || name == "vec!" {
                    return Some("Vec<_>".to_string());
                }
            }
            None
        }
        _ => None,
    }
}

fn evaluate_constant(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Lit(Literal::Int(val), _) => Some(*val as i64),
        Expr::BinOp {
            op, left, right, ..
        } => {
            let l = evaluate_constant(left)?;
            let r = evaluate_constant(right)?;
            match op {
                BinOp::Add => Some(l + r),
                BinOp::Sub => Some(l - r),
                BinOp::Mul => Some(l * r),
                BinOp::Div => {
                    if r != 0 {
                        Some(l / r)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}
