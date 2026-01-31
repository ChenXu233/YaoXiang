//! Token types

use crate::util::span::Span;

/// Lexer error
#[derive(Debug, thiserror::Error)]
pub enum LexError {
    #[error("Invalid token at {position}: {message}")]
    InvalidToken { position: String, message: String },
    #[error("Unterminated string starting at {position}")]
    UnterminatedString { position: String },
    #[error("Invalid escape sequence: {sequence}")]
    InvalidEscape { sequence: String },
    #[error("Invalid number literal: {0}")]
    InvalidNumber(String),
    #[error("Unexpected character: '{ch}'")]
    UnexpectedChar { ch: char },
}

/// Token kind
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (17 total)
    KwType,
    KwPub,
    KwUse,
    KwSpawn,
    KwRef,
    KwMut,
    KwIf,
    KwElif,
    KwElse,
    KwMatch,
    KwWhile,
    KwFor,
    KwIn,
    KwReturn,
    KwBreak,
    KwContinue,
    KwAs,

    // Identifiers
    Identifier(String),
    Underscore,

    // Literals
    IntLiteral(i128),
    FloatLiteral(f64),
    BoolLiteral(bool),
    CharLiteral(char),
    StringLiteral(String),
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
