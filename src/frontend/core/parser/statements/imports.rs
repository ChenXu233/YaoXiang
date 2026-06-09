//! Use/import statement parsing
//!
//! Implements parsing for:
//! - `use path;`
//! - `use path.{item1, item2};`
//! - `use path as alias;`

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::{ParserState, ParseError};
use crate::util::span::Span;

/// Parse use import statement: `use path;` or `use path.{item1, item2};`
pub fn parse_use_stmt(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    state.bump(); // consume 'use'

    let (path, path_span, path_parts) = parse_use_path(state)?;

    // Parse import items: use path.{item1, item2};
    let items = if state.skip(&TokenKind::LBrace) {
        let mut items = Vec::new();
        while !state.at(&TokenKind::RBrace) && !state.at_end() {
            match state.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => {
                    items.push(n.clone());
                    state.bump();
                    state.skip(&TokenKind::Comma);
                }
                Some(TokenKind::KwPub) => {
                    // Skip 'pub' in import items
                    state.bump();
                }
                _ => break,
            }
        }
        state.expect(&TokenKind::RBrace);
        Some(items)
    } else {
        None
    };

    // Parse alias: use path as alias; or use path.{a, b} as alias1, alias2;
    let alias = if state.skip(&TokenKind::KwAs) {
        let mut aliases = Vec::new();
        while let Some(TokenKind::Identifier(n)) = state.current().map(|t| &t.kind) {
            aliases.push(n.clone());
            state.bump();
            // 继续读取逗号分隔的下一个别名
            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
        if aliases.is_empty() {
            None
        } else {
            Some(aliases)
        }
    } else {
        None
    };

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::Use {
            path,
            path_span,
            path_parts,
            items,
            alias,
        },
        span,
    })
}

/// Parse use path (dot-separated identifiers)
fn parse_use_path(state: &mut ParserState<'_>) -> Option<(String, Span, Vec<SpannedIdent>)> {
    let mut parts = Vec::new();
    let mut part_spans = Vec::new();
    let mut start = None;
    let mut end = None;

    while let Some(TokenKind::Identifier(n)) = state.current().map(|t| &t.kind) {
        let token_span = state.span();
        if start.is_none() {
            start = Some(token_span.start);
        }
        end = Some(token_span.end);
        parts.push(n.clone());
        part_spans.push(SpannedIdent {
            name: n.clone(),
            span: token_span,
        });
        state.bump();
        if !state.skip(&TokenKind::Dot) {
            break;
        }
    }

    if parts.is_empty() {
        state.error(ParseError::UnexpectedToken {
            found: state
                .current()
                .map(|t| t.kind.clone())
                .unwrap_or(TokenKind::Eof),
            span: state.span(),
        });
        None
    } else {
        let start = start.unwrap_or_else(|| Span::dummy().start);
        let end = end.unwrap_or_else(|| Span::dummy().end);
        Some((parts.join("."), Span::new(start, end), part_spans))
    }
}
