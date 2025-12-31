//! Token types

use crate::util::span::Span;

/// Token kind
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (30 total)
    KwType,
    KwFn,
    KwAsync,
    KwPub,
    KwMod,
    KwUse,
    KwSpawn,
    KwRef,
    KwMut,
    KwLet,
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
    KwVoid,
    KwBool,
    KwChar,
    KwString,
    KwBytes,
    KwInt,
    KwFloat,

    // Identifiers
    Identifier(String),
    Underscore,

    // Literals
    IntLiteral(i128),
    FloatLiteral(f64),
    BoolLiteral(bool),
    CharLiteral(char),
    StringLiteral(String),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
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
