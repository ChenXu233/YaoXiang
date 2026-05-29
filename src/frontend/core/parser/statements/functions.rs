//! Function definition parsing
//!
//! Implements parsing for:
//! - `name = (params) => body`
//! - `name = param => body` (single param without parentheses)

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::{ParserState, BP_LOWEST};
use crate::util::span::Span;

use super::types::parse_type_annotation;

/// Parse function definition with already parsed name
/// Handles: `[pub] name = (params) => body`
pub fn parse_fn_stmt_with_name(
    state: &mut ParserState<'_>,
    name: String,
    span: Span,
    is_pub: bool,
) -> Option<Stmt> {
    if !state.expect(&TokenKind::LParen) {
        return None;
    }
    let params = parse_fn_params(state)?;
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    if !state.expect(&TokenKind::FatArrow) {
        return None;
    }

    let (stmts, expr) = parse_fn_body(state)?;

    Some(Stmt {
        kind: StmtKind::Binding {
            name,
            type_name: None,
            method_type: None,
            generic_params: Vec::new(),
            type_annotation: None,
            eval: None,
            params,
            body: (stmts, expr),
            is_pub,
        },
        span,
    })
}

/// Parse function definition with already parsed name (simple form)
/// Handles: `[pub] name = param => body` (single param without parentheses)
pub fn parse_fn_stmt_with_name_simple(
    state: &mut ParserState<'_>,
    name: String,
    span: Span,
    is_pub: bool,
) -> Option<Stmt> {
    let param_span = state.span();
    let param_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    if !state.expect(&TokenKind::FatArrow) {
        return None;
    }

    let (stmts, expr) = parse_fn_body(state)?;

    Some(Stmt {
        kind: StmtKind::Binding {
            name,
            type_name: None,
            method_type: None,
            generic_params: Vec::new(),
            type_annotation: None,
            eval: None,
            params: vec![Param {
                name: param_name,
                ty: None,
                is_mut: false,
                span: param_span,
            }],
            body: (stmts, expr),
            is_pub,
        },
        span,
    })
}

/// Parse function body (expression or block)
pub(crate) fn parse_fn_body(state: &mut ParserState<'_>) -> Option<(Vec<Stmt>, Option<Box<Expr>>)> {
    if state.at(&TokenKind::LBrace) {
        if !state.expect(&TokenKind::LBrace) {
            return None;
        }
        let body = parse_block_body_impl(state)?;
        if !state.expect(&TokenKind::RBrace) {
            return None;
        }
        Some(body)
    } else {
        let expr = state.parse_expression(BP_LOWEST)?;
        Some((Vec::new(), Some(Box::new(expr))))
    }
}

/// Parse block body implementation (shared helper)
fn parse_block_body_impl(state: &mut ParserState<'_>) -> Option<(Vec<Stmt>, Option<Box<Expr>>)> {
    let mut stmts = Vec::new();

    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(stmt) = state.parse_statement() {
            stmts.push(stmt);
        } else {
            state.synchronize();
        }
    }

    let expr = if !state.at(&TokenKind::RBrace) {
        state.parse_expression(BP_LOWEST)
    } else {
        None
    };

    Some((stmts, expr.map(Box::new)))
}

/// Parse function parameters: `(param1: Type, param2: Type)`
pub fn parse_fn_params(state: &mut ParserState<'_>) -> Option<Vec<Param>> {
    let mut params = Vec::new();

    while !state.at(&TokenKind::RParen) && !state.at_end() {
        if !params.is_empty() && !state.expect(&TokenKind::Comma) {
            return None;
        }

        if state.at(&TokenKind::RParen) {
            break;
        }

        let param_span = state.span();

        // Handle '...' for variadic parameters
        let _is_variadic = state.skip(&TokenKind::DotDotDot);

        // Check for mut keyword
        let is_mut = state.skip(&TokenKind::KwMut);

        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => break,
        };
        state.bump();

        let ty = if state.skip(&TokenKind::Colon) {
            parse_type_annotation(state)
        } else {
            None
        };

        params.push(Param {
            name,
            ty,
            is_mut,
            span: param_span,
        });
    }

    Some(params)
}
