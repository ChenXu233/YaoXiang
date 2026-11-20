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
    // Keywords (15 total - RFC-010: 'type' keyword removed, use `Name: Type = ...` syntax;
    // RFC-029g: 'pub' keyword removed, no visibility mechanism)
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
    /// raw 保留 f"..." / f"""...""" 的既有内容（转义已解码、花括号序列原样）；
    /// segments 是词法期切好的段，parser 直接消费，不再自己扫 raw 切分（#402）。
    FStringLiteral(FStringToken),
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
/// RFC-012 f-string 词法产物。
///
/// raw 是既有 token 内容（转义已解码、花括号序列原样保留），Display 与既有消费者
/// 逐字不变；segments 是词法期切好的段（唯一事实源），parser 不再自己切分 raw。
#[derive(Debug, Clone, PartialEq)]
pub struct FStringToken {
    pub raw: String,
    pub segments: Vec<FStringTokenSegment>,
}

/// f-string 的段：Text 与 Interpolation 交替，由词法期切分产生。
#[derive(Debug, Clone, PartialEq)]
pub enum FStringTokenSegment {
    /// 文本段：{{ / }} 已解成单个花括号（#402 的既有切分口径）
    Text(String),
    /// 插值段：expr 是去掉格式说明符并 trim 后的表达式源文本；
    /// raw 是插值原内容（含格式说明符），解析失败时回退成字面文本。
    Interpolation {
        expr: String,
        format_spec: Option<String>,
        raw: String,
    },
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
