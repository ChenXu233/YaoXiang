//! Parser state and error handling

use crate::frontend::core::lexer::tokens::*;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;

/// Parse error types (legacy - migrating to Diagnostic)
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    ExpectedToken { expected: TokenKind, found: TokenKind, span: Span },
    UnexpectedToken { found: TokenKind, span: Span },
    Message(String),
}

impl ParseError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            ParseError::ExpectedToken { expected, found, span } => {
                ErrorCodeDefinition::expected_token(&format!("{:?}", expected), &format!("{:?}", found)).at(*span).build()
            }
            ParseError::UnexpectedToken { found, span } => {
                ErrorCodeDefinition::unexpected_token(&format!("{:?}", found)).at(*span).build()
            }
            ParseError::Message(msg) => {
                ErrorCodeDefinition::invalid_syntax(msg).build()
            }
        }
    }
}

/// Convenience: wrap a string msg as an E0012 diagnostic with span
pub fn parse_msg(msg: impl Into<String>, span: Span) -> Diagnostic {
    ErrorCodeDefinition::invalid_syntax(&msg.into()).at(span).build()
}

pub struct ParserState<'a> {
    tokens: &'a [Token],
    pos: usize,
    errors: Vec<Diagnostic>,
}

impl<'a> ParserState<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0, errors: Vec::new() }
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
            || matches!(self.current().map(|t| &t.kind), Some(TokenKind::Eof))
    }
    pub fn current(&self) -> Option<&Token> { self.tokens.get(self.pos) }
    pub fn peek(&self) -> Option<&Token> { self.tokens.get(self.pos + 1) }
    pub fn peek_nth(&self, n: usize) -> Option<&Token> { self.tokens.get(self.pos + n) }
    pub fn span(&self) -> Span { self.current().map(|t| t.span).unwrap_or(Span::dummy()) }
    pub fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned()?; self.pos += 1; Some(t)
    }
    pub fn at(&self, kind: &TokenKind) -> bool {
        self.current().map_or(false, |t| &t.kind == kind)
    }
    pub fn skip(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) { self.bump(); true } else { false }
    }
    pub fn expect(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) { self.bump(); true }
        else {
            let found = self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof);
            self.errors.push(
                ErrorCodeDefinition::expected_token(&format!("{:?}", kind), &format!("{:?}", found))
                    .at(self.span()).build()
            );
            false
        }
    }

    /// Push a legacy ParseError (auto-converts to Diagnostic)
    pub fn error(&mut self, error: ParseError) {
        self.errors.push(error.to_diagnostic());
    }
    /// Push a Diagnostic directly
    pub fn diag_error(&mut self, error: Diagnostic) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool { !self.errors.is_empty() }
    pub fn first_error(&self) -> Option<&Diagnostic> { self.errors.first() }
    pub fn save_position(&self) -> usize { self.pos }
    pub fn restore_position(&mut self, pos: usize) { self.pos = pos; }
    pub fn clear_errors(&mut self) { self.errors.clear(); }
    pub fn error_count(&self) -> usize { self.errors.len() }
    pub fn errors(&self) -> &[Diagnostic] { &self.errors }
    pub fn take_errors(&mut self) -> Vec<Diagnostic> { std::mem::take(&mut self.errors) }
    pub fn truncate_errors(&mut self, len: usize) { self.errors.truncate(len); }

    pub fn can_start_stmt(&self) -> bool {
        !self.at_end() && !self.at(&TokenKind::Semicolon)
    }
    pub fn synchronize(&mut self) {
        while !self.at_end() {
            if self.at(&TokenKind::Semicolon) { self.bump(); break; }
            if let Some(TokenKind::Identifier(_)) = self.current().map(|t| &t.kind) {
                if matches!(self.peek().map(|t| &t.kind),
                    Some(TokenKind::Colon | TokenKind::Eq | TokenKind::LParen | TokenKind::LBrace)
                ) { break; }
            }
            self.bump();
        }
    }
    pub fn parse_statement(&mut self) -> Option<crate::frontend::core::parser::ast::Stmt> {
        use crate::frontend::core::parser::statements::*;
        let ss = self.span();
        match self.current().map(|t| &t.kind) {
            Some(TokenKind::KwUse) => parse_use_stmt(self, ss),
            Some(TokenKind::KwReturn) => parse_return_stmt(self, ss),
            Some(TokenKind::KwBreak) => parse_break_stmt(self, ss),
            Some(TokenKind::KwContinue) => parse_continue_stmt(self, ss),
            Some(TokenKind::KwFor) => parse_for_stmt(self, ss),
            Some(TokenKind::KwIf) => parse_if_stmt(self, ss),
            Some(TokenKind::LBrace) => parse_block_stmt(self, ss),
            Some(TokenKind::KwMut) => parse_var_stmt(self, ss),
            Some(TokenKind::KwPub) => parse_identifier_stmt(self, ss),
            Some(TokenKind::Identifier(_)) => parse_identifier_stmt(self, ss),
            Some(TokenKind::LParen) => parse_paren_destructure_stmt(self, ss),
            Some(TokenKind::Eof) | None => None,
            Some(TokenKind::At) => {
                self.diag_error(ErrorCodeDefinition::unexpected_token("@").at(ss).build());
                None
            }
            Some(kw @ TokenKind::KwRef) | Some(kw @ TokenKind::KwUnsafe)
            | Some(kw @ TokenKind::KwElif) | Some(kw @ TokenKind::KwElse)
            | Some(kw @ TokenKind::KwIn) | Some(kw @ TokenKind::KwAs) => {
                let keyword = match kw {
                    TokenKind::KwRef => "ref", TokenKind::KwUnsafe => "unsafe",
                    TokenKind::KwElif => "elif", TokenKind::KwElse => "else",
                    TokenKind::KwIn => "in", TokenKind::KwAs => "as", _ => "keyword",
                };
                self.diag_error(ErrorCodeDefinition::keyword_as_name(keyword).at(ss).build());
                self.bump(); None
            }
            Some(_) => parse_expr_stmt(self, ss),
        }
    }
    pub fn parse_expression(
        &mut self, min_bp: u8,
    ) -> Option<crate::frontend::core::parser::ast::Expr> {
        crate::frontend::core::parser::pratt::parse_expression_impl(self, min_bp)
    }
}
