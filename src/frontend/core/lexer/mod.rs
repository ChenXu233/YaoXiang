//! Lexer module - refactored for RFC support
//! Split into specialized modules for better maintainability and RFC feature support

pub mod literals;
pub mod state;
pub mod symbols;
pub mod tokenizer;
pub mod tokens;

// Re-export types
pub use tokens::{Token, TokenKind, Literal, LexError};
pub use tokenizer::Lexer;

/// Tokenize source code with RFC support
/// Supports:
/// - RFC-004: Binding syntax (e.g., function[0, 1, 2])
/// - RFC-010: Generic syntax (e.g., List(T), Vec(T: Clone))
/// - RFC-011: Advanced type system features
pub fn tokenize(source: &str) -> Result<Vec<Token>, crate::frontend::core::lexer::LexError> {
    use crate::util::i18n::{t_cur, MSG};

    let source_len = source.len();
    tracing::debug!("{}", t_cur(MSG::LexStart, Some(&[&source_len])));

    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next_token() {
        // Log each token (except EOF which is added later)
        if !matches!(token.kind, TokenKind::Eof) {
            log_token(&token);
        }
        tokens.push(token);
    }

    if let Some(err) = lexer.error {
        Err(err)
    } else {
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: crate::util::span::Span::new(
                lexer.position(),
                lexer.position(), // Use current position for EOF
            ),
            literal: None,
        });
        let token_count = tokens.len();
        tracing::debug!(
            "{}",
            t_cur(MSG::LexCompleteWithTokens, Some(&[&token_count]))
        );
        Ok(tokens)
    }
}

/// Log a token for debugging
fn log_token(token: &Token) {
    use crate::util::i18n::{t_cur, MSG};

    let (msg, arg) = match &token.kind {
        TokenKind::Identifier(name) => (MSG::LexTokenIdentifier, name.clone()),
        TokenKind::KwPub
        | TokenKind::KwUse
        | TokenKind::KwSpawn
        | TokenKind::KwRef
        | TokenKind::KwMut
        | TokenKind::KwIf
        | TokenKind::KwElif
        | TokenKind::KwElse
        | TokenKind::KwMatch
        | TokenKind::KwWhile
        | TokenKind::KwFor
        | TokenKind::KwIn
        | TokenKind::KwReturn
        | TokenKind::KwBreak
        | TokenKind::KwContinue
        | TokenKind::KwAs => (MSG::LexTokenKeyword, format!("{:?}", token.kind)),
        TokenKind::IntLiteral(n) => (MSG::LexTokenNumber, n.to_string()),
        TokenKind::FloatLiteral(f) => (MSG::LexTokenNumber, f.to_string()),
        TokenKind::StringLiteral(s) => (MSG::LexTokenString, s.clone()),
        TokenKind::FStringLiteral(s) => (MSG::LexTokenString, format!("f\"{}\"", s)),
        TokenKind::CharLiteral(c) => (MSG::LexTokenChar, c.to_string()),
        TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash
        | TokenKind::Percent
        | TokenKind::Arrow
        | TokenKind::FatArrow
        | TokenKind::EqEq
        | TokenKind::Neq
        | TokenKind::Le
        | TokenKind::Lt
        | TokenKind::Ge
        | TokenKind::Gt
        | TokenKind::And
        | TokenKind::Or
        | TokenKind::Not
        | TokenKind::ColonColon => (MSG::LexTokenOperator, format!("{:?}", token.kind)),
        _ => (MSG::LexTokenPunctuation, format!("{:?}", token.kind)),
    };
    tracing::debug!("{}", t_cur(msg, Some(&[&arg])));
}

#[cfg(test)]
#[path = "tests/fstring.rs"]
mod fstring_tests;
