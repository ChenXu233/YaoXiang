//! Token types

use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;

/// Lexer error
#[derive(Debug, thiserror::Error)]
pub enum LexError {
    #[error("Invalid token at {position}: {message}")]
    InvalidToken {
        position: String,
        message: String,
        span: Span,
    },
    #[error("Unterminated string starting at {position}")]
    UnterminatedString { position: String, span: Span },
    #[error("Invalid escape sequence: {sequence}")]
    InvalidEscape { sequence: String, span: Span },
    #[error("Invalid number literal: {0}")]
    InvalidNumber(String, Span),
    #[error("Unexpected character: '{ch}'")]
    UnexpectedChar { ch: char, span: Span },
    #[error("Unterminated f-string interpolation starting at {position}")]
    UnterminatedFStringInterpolation { position: String, span: Span },
}

impl LexError {
    /// 错误发生点的源码 span（#324：诊断位置由构造点提供）
    pub fn span(&self) -> Span {
        match self {
            LexError::InvalidNumber(_, s) => *s,
            LexError::InvalidToken { span, .. }
            | LexError::UnterminatedString { span, .. }
            | LexError::InvalidEscape { span, .. }
            | LexError::UnexpectedChar { span, .. }
            | LexError::UnterminatedFStringInterpolation { span, .. } => *span,
        }
    }

    /// Convert to Diagnostic for unified error system
    pub fn to_diagnostic(&self) -> Diagnostic {
        let span = self.span();
        match self {
            LexError::InvalidToken { message, .. } => ErrorCodeDefinition::invalid_syntax(message)
                .at(span)
                .build(),
            LexError::UnterminatedString { position, .. } => {
                ErrorCodeDefinition::invalid_syntax(&format!("unterminated string at {}", position))
                    .at(span)
                    .build()
            }
            LexError::InvalidEscape { sequence, .. } => {
                ErrorCodeDefinition::invalid_syntax(&format!("invalid escape: {}", sequence))
                    .at(span)
                    .build()
            }
            LexError::InvalidNumber(literal, _) => {
                ErrorCodeDefinition::invalid_number_literal(literal)
                    .at(span)
                    .build()
            }
            LexError::UnexpectedChar { ch, .. } => {
                ErrorCodeDefinition::invalid_character(&ch.to_string())
                    .at(span)
                    .build()
            }
            LexError::UnterminatedFStringInterpolation { position, .. } => {
                ErrorCodeDefinition::invalid_syntax(&format!(
                    "unterminated f-string interpolation at {}",
                    position
                ))
                .at(span)
                .build()
            }
        }
    }
}

/// Token kind
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (16 total - RFC-010: 'type' keyword removed, use `Name: Type = ...` syntax)
    KwPub,
    KwUse,
    KwSpawn,
    KwRef,
    KwMut,
    KwIf,
    KwElse,
    KwMatch,
    KwWhile,
    KwFor,
    KwIn,
    KwReturn,
    KwBreak,
    KwContinue,
    KwAs,
    KwUnsafe,

    // Identifiers
    Identifier(String),
    Underscore,

    // Literals
    IntLiteral(i128),
    FloatLiteral(f64),
    BoolLiteral(bool),
    CharLiteral(char),
    StringLiteral(String),
    /// RFC-012: F-string template literal
    /// Stores the raw content of f"..." including interpolation markers
    FStringLiteral(String),
    VoidLiteral,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    Ampersand,
    MutRef,
    // #285: 位异或（SPEC §2.2 级 8）
    Caret,
    ColonColon,
    DotDotDot,
    DotDot,

    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    At,
    Comma,
    Colon,
    Semicolon,
    Pipe,
    Dot,
    Arrow,
    FatArrow,
    Question,

    // Special
    Eof,
    Error(String),
}

/// Token
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub literal: Option<Literal>,
}

/// Literal value
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i128),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Void,
}

impl From<TokenKind> for Token {
    fn from(kind: TokenKind) -> Self {
        Token {
            kind,
            span: Span::dummy(),
            literal: None,
        }
    }
}
