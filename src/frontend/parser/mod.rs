//! Parser module

pub mod ast;

use ast::*;
use crate::frontend::lexer::tokens::*;
use crate::util::span::Span;

/// Parse tokens into AST
pub fn parse(tokens: &[Token]) -> Result<Module, ParseError> {
    // TODO: Implement parser
    Ok(Module {
        items: vec![],
        span: Span::dummy(),
    })
}

#[derive(Debug, thiserror::Error)]
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
}
