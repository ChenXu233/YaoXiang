//! Lexer module

pub mod tokens;

use tokens::*;

pub use tokenizer::tokenize;

/// Tokenize source code
mod tokenizer {
    use super::*;

    pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
        // TODO: Implement lexer
        Ok(vec![])
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LexError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Unterminated string")]
    UnterminatedString,
    #[error("Invalid escape sequence: {0}")]
    InvalidEscape(String),
    #[error("Invalid number: {0}")]
    InvalidNumber(String),
}
