//! Tokenizer implementation
//! Main lexer structure and token generation logic
//! Supports RFC-004 binding syntax and RFC-010/011 generic syntax

use super::state::LexerState;
use super::literals::{
    scan_number, scan_string, scan_char, scan_leading_dot, is_identifier_start, is_identifier_char,
    is_digit,
};
use crate::frontend::core::lexer::tokens::*;
use crate::util::span::{Position, Span};
use std::iter::Peekable;
use std::str::Chars;

/// Main lexer structure
pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    offset: usize,
    line: usize,
    column: usize,
    start_offset: usize,
    start_line: usize,
    start_column: usize,
    pub error: Option<crate::frontend::core::lexer::LexError>,
    state: LexerState,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given source
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().peekable(),
            offset: 0,
            line: 1,
            column: 1,
            start_offset: 0,
            start_line: 1,
            start_column: 1,
            error: None,
            state: LexerState::new(),
        }
    }

    /// Get current position
    pub fn position(&self) -> Position {
        Position::with_offset(self.line, self.column, self.offset)
    }

    /// Get start position of current token
    pub fn start_position(&self) -> Position {
        Position::with_offset(self.start_line, self.start_column, self.start_offset)
    }

    /// Get span of current token
    pub fn span(&self) -> Span {
        Span::new(self.start_position(), self.position())
    }

    /// Advance to next character
    pub fn advance(&mut self) -> Option<char> {
        match self.chars.next() {
            Some('\n') => {
                self.offset += 1;
                self.line += 1;
                self.column = 1;
                Some('\n')
            }
            Some(c) => {
                self.offset += c.len_utf8();
                self.column += 1;
                Some(c)
            }
            None => None,
        }
    }

    /// Peek at next character
    pub fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    /// Peek at character after next
    pub fn peek_next(&mut self) -> Option<char> {
        self.chars.clone().nth(1)
    }

    /// Get start line for error reporting
    pub fn start_line(&self) -> usize {
        self.start_line
    }

    /// Get start column for error reporting
    pub fn start_column(&self) -> usize {
        self.start_column
    }

    /// Get start offset for error reporting
    pub fn start_offset(&self) -> usize {
        self.start_offset
    }

    /// Get a clone of chars for lookahead operations
    pub fn chars_clone(&self) -> Peekable<Chars<'a>> {
        self.chars.clone()
    }

    /// Peek at next character (public for literals module)
    pub fn peek_public(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    /// Skip whitespace and comments
    fn skip_whitespace_and_comments(&mut self) {
        while let Some(&c) = self.peek() {
            match c {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' => {
                    // Check for comments
                    if self.peek_next() == Some('/') {
                        // Single line comment
                        self.advance();
                        self.advance();
                        while let Some(&c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    } else if self.peek_next() == Some('*') {
                        // Multi-line comment
                        self.advance();
                        self.advance();
                        let mut depth = 1;
                        while depth > 0 {
                            if let Some(c) = self.advance() {
                                if c == '/' && self.peek() == Some(&'*') {
                                    self.advance();
                                    depth += 1;
                                } else if c == '*' && self.peek() == Some(&'/') {
                                    self.advance();
                                    depth -= 1;
                                }
                            } else {
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    /// Generate next token
    pub fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace_and_comments();

        // Check if at end of file
        self.peek()?;

        self.start_offset = self.offset;
        self.start_line = self.line;
        self.start_column = self.column;

        let c = self.advance().unwrap();

        match c {
            '_' => {
                // Check if next char is part of identifier (e.g., _foo)
                // Only treat standalone _ as Underscore token
                if self.peek().map(|&c| is_identifier_char(c)).unwrap_or(false) {
                    self.scan_identifier(c)
                } else {
                    Some(self.make_token(TokenKind::Underscore))
                }
            }
            c if is_identifier_start(c) => self.scan_identifier(c),
            c if is_digit(c) => scan_number(self, c),
            '"' => scan_string(self),
            '\'' => scan_char(self),
            '+' => Some(self.make_token(TokenKind::Plus)),
            '-' => {
                if self.peek() == Some(&'>') {
                    self.advance();
                    Some(self.make_token(TokenKind::Arrow))
                } else {
                    Some(self.make_token(TokenKind::Minus))
                }
            }
            '*' => Some(self.make_token(TokenKind::Star)),
            '%' => Some(self.make_token(TokenKind::Percent)),
            ',' => Some(self.make_token(TokenKind::Comma)),
            ';' => Some(self.make_token(TokenKind::Semicolon)),
            '(' => Some(self.make_token(TokenKind::LParen)),
            ')' => Some(self.make_token(TokenKind::RParen)),
            '[' => {
                // RFC-004: Binding syntax support
                // Left bracket [ for binding positions
                Some(self.make_token(TokenKind::LBracket))
            }
            ']' => {
                // RFC-004: Binding syntax support
                // Right bracket ] for binding positions
                Some(self.make_token(TokenKind::RBracket))
            }
            '{' => Some(self.make_token(TokenKind::LBrace)),
            '}' => Some(self.make_token(TokenKind::RBrace)),
            '=' => {
                if self.peek() == Some(&'>') {
                    self.advance();
                    Some(self.make_token(TokenKind::FatArrow))
                } else if self.peek() == Some(&'=') {
                    self.advance();
                    Some(self.make_token(TokenKind::EqEq))
                } else {
                    Some(self.make_token(TokenKind::Eq))
                }
            }
            '!' => {
                if self.peek() == Some(&'=') {
                    self.advance();
                    Some(self.make_token(TokenKind::Neq))
                } else {
                    Some(self.make_token(TokenKind::Not))
                }
            }
            '<' => {
                // RFC-010/011: Generic syntax support
                if self.peek() == Some(&'=') {
                    self.advance();
                    Some(self.make_token(TokenKind::Le))
                } else {
                    Some(self.make_token(TokenKind::Lt))
                }
            }
            '>' => {
                if self.peek() == Some(&'=') {
                    self.advance();
                    Some(self.make_token(TokenKind::Ge))
                } else {
                    Some(self.make_token(TokenKind::Gt))
                }
            }
            '&' => {
                if self.peek() == Some(&'&') {
                    self.advance();
                    Some(self.make_token(TokenKind::And))
                } else {
                    self.error =
                        Some(crate::frontend::core::lexer::LexError::UnexpectedChar { ch: '&' });
                    Some(self.make_token(TokenKind::Error("Unexpected character: &".to_string())))
                }
            }
            '|' => {
                if self.peek() == Some(&'|') {
                    self.advance();
                    Some(self.make_token(TokenKind::Or))
                } else {
                    Some(self.make_token(TokenKind::Pipe))
                }
            }
            ':' => {
                if self.peek() == Some(&':') {
                    self.advance();
                    Some(self.make_token(TokenKind::ColonColon))
                } else {
                    Some(self.make_token(TokenKind::Colon))
                }
            }
            '.' => {
                if self.peek() == Some(&'.') {
                    self.advance();
                    if self.peek() == Some(&'.') {
                        self.advance();
                        Some(self.make_token(TokenKind::DotDotDot))
                    } else {
                        Some(self.make_token(TokenKind::DotDot))
                    }
                } else if self.peek().map(|c| is_digit(*c)).unwrap_or(false) {
                    // Leading decimal point: .5
                    scan_leading_dot(self)
                } else {
                    Some(self.make_token(TokenKind::Dot))
                }
            }
            '/' => Some(self.make_token(TokenKind::Slash)),
            '?' => Some(self.make_token(TokenKind::Question)),
            c => {
                self.error = Some(crate::frontend::core::lexer::LexError::UnexpectedChar { ch: c });
                Some(self.make_token(TokenKind::Error(format!("Unexpected character: {}", c))))
            }
        }
    }

    /// Scan identifier token
    fn scan_identifier(
        &mut self,
        first_char: char,
    ) -> Option<Token> {
        let mut value = String::new();
        value.push(first_char);

        while let Some(&c) = self.peek() {
            if is_identifier_char(c) {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if let Some(kind) = self.state.keyword_from_str(&value) {
            Some(Token {
                kind,
                span: self.span(),
                literal: None,
            })
        } else {
            Some(Token {
                kind: TokenKind::Identifier(value.clone()),
                span: self.span(),
                literal: None,
            })
        }
    }

    /// Create token with current span
    pub fn make_token(
        &self,
        kind: TokenKind,
    ) -> Token {
        Token {
            kind,
            span: self.span(),
            literal: None,
        }
    }
}
