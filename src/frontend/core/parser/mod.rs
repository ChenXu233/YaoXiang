//! Parser module
//!
//! Implements a Pratt Parser for the YaoXiang language with RFC-004/010/011 support.
//! This module provides the main entry points for parsing tokens into AST.

pub mod ast;
pub mod parser_state;
pub mod pratt;
pub mod statements;
#[cfg(test)]
pub mod tests;

// Re-export commonly used items
pub use parser_state::{ParserState, ParseError};
pub use statements::StatementParser;
pub use pratt::*;
pub use ast::*;

// Re-export lexer tokens
pub use crate::frontend::core::lexer::tokens::*;
pub use crate::util::span::Span;

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

    while !state.at_end() {
        // Skip empty statements (like stray semicolons)
        if !state.can_start_stmt() {
            if state.at(&TokenKind::Semicolon) {
                state.bump();
                continue;
            }

            state.error(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.span(),
            });
            state.bump();
            continue;
        }

        if let Some(stmt) = state.parse_statement() {
            items.push(stmt);
        } else {
            state.bump(); // Skip problematic tokens
        }
    }

    if state.has_errors() {
        if let Some(error) = state.first_error().cloned() {
            Err(error)
        } else {
            Err(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.span(),
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
                Err(ParseError::UnexpectedToken {
                    found: state
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                    span: state.span(),
                })
            }
        }
    }
}
