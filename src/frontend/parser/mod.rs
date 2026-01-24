//! Parser module
//!
//! This module implements a Pratt Parser for the YaoXiang language.
//! The parser transforms tokens into an Abstract Syntax Tree (AST).

pub mod ast;
mod expr;
mod led;
mod nud;
mod state;
mod stmt;
mod type_parser;

pub use state::{ParserState, BP_HIGHEST, BP_LOWEST};

use crate::frontend::lexer::tokens::*;
use crate::util::span::Span;
use crate::util::i18n::{t_cur, MSG};
use ast::*;
use tracing::debug;

/// Parse tokens into an AST module
///
/// # Arguments
/// * `tokens` - Token stream from the lexer
///
/// # Returns
/// Parsed module or first parse error
///
/// # Example
/// ```yaoxiang
/// fn main() {
///     print("Hello");
/// }
/// ```
pub fn parse(tokens: &[Token]) -> Result<Module, ParseError> {
    let token_count = tokens.len();
    debug!("{}", t_cur(MSG::ParserStart, Some(&[&token_count])));
    let mut state = ParserState::new(tokens);
    let mut items = Vec::new();

    while !state.at_end() {
        // Skip empty statements (like stray semicolons)
        if !state.can_start_stmt() {
            // Allow semicolons to be skipped without error (empty statements)
            if state.at(&TokenKind::Semicolon) {
                state.bump();
                continue;
            }

            // Report error for unexpected token
            let span = state.current().map(|t| t.span).unwrap_or_else(Span::dummy);
            state.error(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span,
            });
            state.synchronize();
            continue;
        }

        match state.parse_stmt() {
            Some(stmt) => {
                items.push(stmt);
            }
            None => {
                // Skip to next statement or EOF
                state.synchronize();
            }
        }
    }

    if state.has_errors() {
        // Return the first error
        if let Some(error) = state.first_error().cloned() {
            Err(error)
        } else {
            // Should not happen, but return a generic error
            Err(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.current().map(|t| t.span).unwrap_or_else(Span::dummy),
            })
        }
    } else {
        let span = if let Some(first) = items.first() {
            if let Some(last) = items.last() {
                Span::new(first.span.start, last.span.end)
            } else {
                Span::dummy()
            }
        } else {
            Span::dummy()
        };

        let item_count = items.len();
        debug!(
            "{}",
            t_cur(MSG::ParserCompleteWithItems, Some(&[&item_count]))
        );
        Ok(Module { items, span })
    }
}

/// Parse a single expression
///
/// # Arguments
/// * `tokens` - Token stream
///
/// # Returns
/// Parsed expression or error
pub fn parse_expression(tokens: &[Token]) -> Result<Expr, ParseError> {
    let mut state = ParserState::new(tokens);
    let expr = state.parse_expression(BP_LOWEST);

    match expr {
        Some(e) => {
            if state.has_errors() {
                if let Some(error) = state.first_error().cloned() {
                    Err(error)
                } else {
                    Ok(e)
                }
            } else {
                Ok(e)
            }
        }
        None => {
            if let Some(error) = state.first_error().cloned() {
                Err(error)
            } else {
                Err(ParseError::InvalidExpression {
                    span: state.current().map(|t| t.span).unwrap_or_else(Span::dummy),
                })
            }
        }
    }
}

/// Parse error types
#[derive(Debug, thiserror::Error, Clone)]
pub enum ParseError {
    #[error("Unexpected token: {found:?}")]
    UnexpectedToken { found: TokenKind, span: Span },

    #[error("Expected token: {expected:?}, found: {found:?}")]
    ExpectedToken {
        expected: TokenKind,
        found: TokenKind,
        span: Span,
    },

    #[error("Unterminated block")]
    UnterminatedBlock { span: Span },

    #[error("Invalid expression")]
    InvalidExpression { span: Span },

    #[error("Invalid pattern")]
    InvalidPattern { span: Span },

    #[error("Invalid type annotation")]
    InvalidType { span: Span },

    #[error("Missing semicolon after statement")]
    MissingSemicolon { span: Span },

    #[error("Unexpected end of input")]
    UnexpectedEof { span: Span },

    #[error("{message}")]
    Generic { message: String, span: Span },
}

impl ParseError {
    pub fn span(&self) -> Span {
        match self {
            ParseError::UnexpectedToken { span, .. } => *span,
            ParseError::ExpectedToken { span, .. } => *span,
            ParseError::UnterminatedBlock { span } => *span,
            ParseError::InvalidExpression { span } => *span,
            ParseError::InvalidPattern { span } => *span,
            ParseError::InvalidType { span } => *span,
            ParseError::MissingSemicolon { span } => *span,
            ParseError::UnexpectedEof { span } => *span,
            ParseError::Generic { span, .. } => *span,
        }
    }
}

#[cfg(test)]
mod tests;
