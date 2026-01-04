//! Parser state and token stream management

use super::super::lexer::tokens::*;
use crate::util::span::Span;

/// Synchronization points for error recovery
const SYNC_POINTS: &[TokenKind] = &[
    TokenKind::KwMut,
    TokenKind::KwType,
    TokenKind::KwUse,
    TokenKind::KwIf,
    TokenKind::KwWhile,
    TokenKind::KwFor,
    TokenKind::KwMatch,
    TokenKind::LBrace,
    TokenKind::Eof,
];

/// Binding power levels for Pratt parser
pub const BP_LOWEST: u8 = 0;
pub const BP_ASSIGN: u8 = 10;
pub const BP_RANGE: u8 = 15;
pub const BP_OR: u8 = 20;
pub const BP_AND: u8 = 30;
pub const BP_EQ: u8 = 40;
pub const BP_CMP: u8 = 50;
pub const BP_ADD: u8 = 60;
pub const BP_MUL: u8 = 70;
pub const BP_UNARY: u8 = 80;
pub const BP_CALL: u8 = 90;
pub const BP_HIGHEST: u8 = 100;

/// Parser state for tracking position and errors
#[derive(Debug)]
pub struct ParserState<'a> {
    /// Token stream
    tokens: &'a [Token],
    /// Current position in token stream
    pos: usize,
    /// Parsing errors
    errors: Vec<super::ParseError>,
    /// Current span for error reporting
    current_span: Span,
}

impl<'a> ParserState<'a> {
    /// Create a new parser state
    #[inline]
    pub fn new(tokens: &'a [Token]) -> Self {
        let span = tokens.first().map(|t| t.span).unwrap_or_else(Span::dummy);

        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
            current_span: span,
        }
    }

    /// Check if at end of token stream
    #[inline]
    pub fn at_end(&self) -> bool {
        self.pos >= self.tokens.len() || matches!(self.tokens[self.pos].kind, TokenKind::Eof)
    }

    /// Get current token
    #[inline]
    pub fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    /// Get current token kind
    #[inline]
    pub fn at(&self, kind: &TokenKind) -> bool {
        matches!(self.current(), Some(t) if &t.kind == kind)
    }

    /// Peek at next token
    #[inline]
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    /// Peek at nth token ahead
    #[inline]
    pub fn peek_nth(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.pos + n)
    }

    /// Advance to next token
    #[inline]
    pub fn bump(&mut self) {
        if !self.at_end() {
            self.pos += 1;
            if let Some(token) = self.current() {
                self.current_span = token.span;
            }
        }
    }

    /// Skip a specific token
    #[inline]
    pub fn skip(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Expect a specific token, report error if not found
    #[inline]
    pub fn expect(&mut self, kind: &TokenKind) -> bool {
        if let Some(token) = self.current() {
            if &token.kind == kind {
                self.bump();
                return true;
            } else {
                self.error(super::ParseError::ExpectedToken(
                    kind.clone(),
                    token.kind.clone(),
                ));
            }
        } else {
            self.error(super::ParseError::ExpectedToken(
                kind.clone(),
                TokenKind::Eof,
            ));
        }
        false
    }

    /// Start tracking a new span
    #[inline]
    pub fn start_span(&mut self) {
        if let Some(token) = self.current() {
            self.current_span = token.span;
        }
    }

    /// Get current span
    #[inline]
    pub fn span(&self) -> Span {
        self.current_span
    }

    /// Create a span from current position
    #[inline]
    pub fn span_from(&self, start: Span) -> Span {
        Span::new(start.start, self.current_span.end)
    }

    /// Add a parse error
    #[inline]
    pub fn error(&mut self, error: super::ParseError) {
        self.errors.push(error);
    }

    /// Check if there are errors
    #[inline]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get all errors
    #[inline]
    pub fn into_errors(self) -> Vec<super::ParseError> {
        self.errors
    }

    /// Get first error
    #[inline]
    pub fn first_error(&self) -> Option<&super::ParseError> {
        self.errors.first()
    }

    /// Synchronize to next synchronization point
    pub fn synchronize(&mut self) {
        while !self.at_end() {
            if let Some(token) = self.current() {
                if SYNC_POINTS.iter().any(|sp| sp == &token.kind) {
                    break;
                }
            }
            self.bump();
        }
    }

    /// Consume tokens until a synchronization point
    #[inline]
    pub fn skip_to_sync(&mut self) {
        self.synchronize();
    }

    /// Check if current token can start a statement
    #[inline]
    pub fn can_start_stmt(&self) -> bool {
        // Expression statements can start with any expression
        self.can_start_expr()
            || matches!(
                self.current().map(|t| &t.kind),
                Some(TokenKind::KwMut)
                    | Some(TokenKind::KwType)
                    | Some(TokenKind::KwUse)
                    | Some(TokenKind::KwIf)
                    | Some(TokenKind::KwWhile)
                    | Some(TokenKind::KwFor)
                    | Some(TokenKind::KwMatch)
                    | Some(TokenKind::KwReturn)
                    | Some(TokenKind::KwBreak)
                    | Some(TokenKind::KwContinue)
                    | Some(TokenKind::LBrace)
                    // Identifier can start variable/function declaration
                    | Some(TokenKind::Identifier(_))
            )
    }

    /// Check if current token can start an expression
    #[inline]
    pub fn can_start_expr(&self) -> bool {
        matches!(
            self.current().map(|t| &t.kind),
            Some(TokenKind::IntLiteral(_))
                | Some(TokenKind::FloatLiteral(_))
                | Some(TokenKind::StringLiteral(_))
                | Some(TokenKind::CharLiteral(_))
                | Some(TokenKind::BoolLiteral(_))
                | Some(TokenKind::Identifier(_))
                | Some(TokenKind::Minus)
                | Some(TokenKind::Plus)
                | Some(TokenKind::Not)
                | Some(TokenKind::LParen)
                | Some(TokenKind::LBrace)
                | Some(TokenKind::KwIf)
                | Some(TokenKind::KwMatch)
                | Some(TokenKind::KwWhile)
                | Some(TokenKind::KwFor)
                | Some(TokenKind::Pipe)
        )
    }
}
