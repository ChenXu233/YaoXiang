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
use ast::*;

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
    let mut state = ParserState::new(tokens);
    let mut items = Vec::new();
    let mut stmt_count = 0;

    while !state.at_end() {
        // Skip empty statements (like stray semicolons)
        if !state.can_start_stmt() {
            // Allow semicolons to be skipped without error (empty statements)
            if state.at(&TokenKind::Semicolon) {
                state.bump();
                continue;
            }

            eprintln!(
                "[DEBUG] Skipping token that can't start statement: {:?}",
                state.current().map(|t| &t.kind)
            );
            // Report error for unexpected token
            state.error(ParseError::UnexpectedToken(
                state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
            ));
            state.synchronize();
            continue;
        }

        stmt_count += 1;
        eprintln!("[DEBUG] Parsing statement {}", stmt_count);
        match state.parse_stmt() {
            Some(stmt) => {
                eprintln!(
                    "[DEBUG] Statement {} parsed successfully: {:?}",
                    stmt_count,
                    std::mem::discriminant(&stmt.kind)
                );
                items.push(stmt);
            }
            None => {
                eprintln!(
                    "[DEBUG] Statement {} failed to parse, current token: {:?}",
                    stmt_count,
                    state.current().map(|t| &t.kind)
                );
                if state.has_errors() {
                    eprintln!("[DEBUG] Error set: {:?}", state.first_error());
                }
                // Skip to next statement or EOF
                state.synchronize();
            }
        }
    }

    eprintln!("[DEBUG] Total statements parsed: {}", items.len());

    if state.has_errors() {
        // Return the first error
        if let Some(error) = state.first_error().cloned() {
            eprintln!("[DEBUG] Parse error detected: {:?}", error);
            Err(error)
        } else {
            // Should not happen, but return a generic error
            eprintln!("[DEBUG] No specific error but has_errors() is true");
            Err(ParseError::UnexpectedToken(
                state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
            ))
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
                Err(ParseError::InvalidExpression)
            }
        }
    }
}

/// Parse error types
#[derive(Debug, thiserror::Error, Clone)]
pub enum ParseError {
    #[error("Unexpected token: {0:?}")]
    UnexpectedToken(TokenKind),

    #[error("Expected token: {0:?}, found: {1:?}")]
    ExpectedToken(TokenKind, TokenKind),

    #[error("Unterminated block")]
    UnterminatedBlock,

    #[error("Invalid expression")]
    InvalidExpression,

    #[error("Invalid pattern")]
    InvalidPattern,

    #[error("Invalid type annotation")]
    InvalidType,

    #[error("Missing semicolon after statement")]
    MissingSemicolon,

    #[error("Unexpected end of input")]
    UnexpectedEof,

    #[error("{0}")]
    Generic(String),
}

#[cfg(test)]
mod tests;
