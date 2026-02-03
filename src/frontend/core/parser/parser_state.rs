//! Parser state and error handling

use crate::frontend::core::lexer::tokens::*;
use crate::util::span::Span;

/// Parse error types
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Expected a specific token
    ExpectedToken {
        expected: TokenKind,
        found: TokenKind,
        span: Span,
    },
    /// Unexpected token encountered
    UnexpectedToken { found: TokenKind, span: Span },
    /// Generic parse error with message
    Message(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ParseError::ExpectedToken {
                expected, found, ..
            } => {
                write!(f, "Expected token {:?}, found {:?}", expected, found)
            }
            ParseError::UnexpectedToken { found, .. } => {
                write!(f, "Unexpected token: {:?}", found)
            }
            ParseError::Message(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parser state for Pratt parsing
pub struct ParserState<'a> {
    tokens: &'a [Token],
    pos: usize,
    errors: Vec<ParseError>,
}

impl<'a> ParserState<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
            || matches!(self.current().map(|t| &t.kind), Some(TokenKind::Eof))
    }

    pub fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    pub fn peek_nth(
        &self,
        n: usize,
    ) -> Option<&Token> {
        self.tokens.get(self.pos + n)
    }

    pub fn span(&self) -> Span {
        self.current().map(|t| t.span).unwrap_or(Span::dummy())
    }

    pub fn bump(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned()?;
        self.pos += 1;
        Some(token)
    }

    pub fn at(
        &self,
        kind: &TokenKind,
    ) -> bool {
        if let Some(current) = self.current() {
            &current.kind == kind
        } else {
            false
        }
    }

    pub fn skip(
        &mut self,
        kind: &TokenKind,
    ) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub fn expect(
        &mut self,
        kind: &TokenKind,
    ) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            let found = self
                .current()
                .map(|t| t.kind.clone())
                .unwrap_or(TokenKind::Eof);
            self.error(ParseError::ExpectedToken {
                expected: kind.clone(),
                found,
                span: self.span(),
            });
            false
        }
    }

    pub fn error(
        &mut self,
        error: ParseError,
    ) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn first_error(&self) -> Option<&ParseError> {
        self.errors.first()
    }

    /// Save current position for backtracking
    pub fn save_position(&self) -> usize {
        self.pos
    }

    /// Restore a previously saved position
    pub fn restore_position(
        &mut self,
        pos: usize,
    ) {
        self.pos = pos;
    }

    /// Clear errors (used after backtracking)
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    pub fn can_start_stmt(&self) -> bool {
        if self.at_end() {
            return false;
        }

        match &self.current().unwrap().kind {
            TokenKind::Semicolon => false, // Empty statement
            _ => true,
        }
    }

    /// Synchronize parser after error (skip to next statement boundary)
    pub fn synchronize(&mut self) {
        while !self.at_end() {
            if self.at(&TokenKind::Semicolon) {
                self.bump();
                break;
            }
            if let Some(TokenKind::Identifier(_)) = self.current().map(|t| &t.kind) {
                // Check if this looks like a statement start
                if let Some(next) = self.peek() {
                    match &next.kind {
                        TokenKind::Colon
                        | TokenKind::Eq
                        | TokenKind::LParen
                        | TokenKind::LBrace => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
            self.bump();
        }
    }

    /// Parse a statement
    pub fn parse_statement(&mut self) -> Option<crate::frontend::core::parser::ast::Stmt> {
        use crate::frontend::core::parser::statements::*;
        let start_span = self.span();

        match self.current().map(|t| &t.kind) {
            Some(TokenKind::KwType) => parse_type_stmt(self, start_span),
            Some(TokenKind::KwUse) => parse_use_stmt(self, start_span),
            Some(TokenKind::KwReturn) => parse_return_stmt(self, start_span),
            Some(TokenKind::KwBreak) => parse_break_stmt(self, start_span),
            Some(TokenKind::KwContinue) => parse_continue_stmt(self, start_span),
            Some(TokenKind::KwFor) => parse_for_stmt(self, start_span),
            Some(TokenKind::KwIf) => parse_if_stmt(self, start_span),
            Some(TokenKind::LBrace) => parse_block_stmt(self, start_span),
            Some(TokenKind::KwMut) => parse_var_stmt(self, start_span),
            Some(TokenKind::KwPub) => parse_identifier_stmt(self, start_span), // pub 声明由 parse_identifier_stmt 处理
            Some(TokenKind::Identifier(_)) => parse_identifier_stmt(self, start_span),
            Some(TokenKind::Eof) | None => None,
            Some(_) => parse_expr_stmt(self, start_span),
        }
    }

    /// Parse an expression with minimum binding power
    pub fn parse_expression(
        &mut self,
        min_bp: u8,
    ) -> Option<crate::frontend::core::parser::ast::Expr> {
        // Delegate to the Pratt parser module
        crate::frontend::core::parser::pratt::parse_expression_impl(self, min_bp)
    }
}
