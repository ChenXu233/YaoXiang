//! Lexer state management
//! Handles lexer state and keyword recognition
//! Supports RFC-010 generic syntax keywords

use crate::frontend::core::lexer::tokens::TokenKind;

/// Lexer state management
pub struct LexerState {
    // RFC-010: Support for generic syntax keywords
    // Additional state tracking for RFC features can be added here
}

impl LexerState {
    /// Create new lexer state
    pub fn new() -> Self {
        Self {}
    }

    /// Convert string to keyword token
    /// Supports RFC-010 unified syntax keywords
    pub fn keyword_from_str(
        &self,
        s: &str,
    ) -> Option<TokenKind> {
        match s {
            // Basic type and declaration keywords
            "type" => Some(TokenKind::KwType),
            "pub" => Some(TokenKind::KwPub),
            "use" => Some(TokenKind::KwUse),

            // Concurrency keywords
            "spawn" => Some(TokenKind::KwSpawn),
            "ref" => Some(TokenKind::KwRef),
            "mut" => Some(TokenKind::KwMut),

            // Control flow keywords
            "if" => Some(TokenKind::KwIf),
            "elif" => Some(TokenKind::KwElif),
            "else" => Some(TokenKind::KwElse),
            "match" => Some(TokenKind::KwMatch),
            "while" => Some(TokenKind::KwWhile),
            "for" => Some(TokenKind::KwFor),
            "in" => Some(TokenKind::KwIn),
            "return" => Some(TokenKind::KwReturn),
            "break" => Some(TokenKind::KwBreak),
            "continue" => Some(TokenKind::KwContinue),

            // Type casting and conversion
            "as" => Some(TokenKind::KwAs),

            // Boolean literals
            "true" => Some(TokenKind::BoolLiteral(true)),
            "false" => Some(TokenKind::BoolLiteral(false)),

            // Void type
            "void" => Some(TokenKind::VoidLiteral),

            _ => None,
        }
    }
}

impl Default for LexerState {
    fn default() -> Self {
        Self::new()
    }
}
