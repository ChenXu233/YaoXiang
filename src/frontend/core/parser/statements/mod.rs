//! Statement parsing modules
//! Contains specialized modules for different statement types

pub mod bindings;
pub mod control_flow;
pub mod declarations;
pub mod types;

// Re-export commonly used items
pub use types::*;
pub use declarations::*;
pub use control_flow::*;
pub use bindings::*;

/// Statement parsing trait for RFC support
pub trait StatementParser {
    /// Parse a statement with RFC support
    fn parse_statement(&mut self) -> Option<crate::frontend::core::parser::ast::Stmt>;
}

/// Bridge implementation to connect ParserState with statement parsing methods
use crate::frontend::core::parser::ParserState;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::lexer::tokens::*;

impl StatementParser for ParserState<'_> {
    fn parse_statement(&mut self) -> Option<Stmt> {
        let start_span = self.span();

        match self.current().map(|t| &t.kind) {
            // RFC-010: 'type' keyword removed
            // Type definitions use `Name: Type = { ... }` syntax (handled by parse_identifier_stmt)
            // use import
            Some(TokenKind::KwUse) => declarations::parse_use_stmt(self, start_span),
            // return statement
            Some(TokenKind::KwReturn) => control_flow::parse_return_stmt(self, start_span),
            // break statement
            Some(TokenKind::KwBreak) => control_flow::parse_break_stmt(self, start_span),
            // continue statement
            Some(TokenKind::KwContinue) => control_flow::parse_continue_stmt(self, start_span),
            // for loop
            Some(TokenKind::KwFor) => control_flow::parse_for_stmt(self, start_span),
            // while loop
            Some(TokenKind::KwWhile) => control_flow::parse_while_stmt(self, start_span),
            // if statement
            Some(TokenKind::KwIf) => control_flow::parse_if_stmt(self, start_span),
            // block as statement
            Some(TokenKind::LBrace) => control_flow::parse_block_stmt(self, start_span),
            // variable declaration: [mut] identifier [: type] [= expr]
            Some(TokenKind::KwMut) => declarations::parse_var_stmt(self, start_span),
            Some(TokenKind::Identifier(_)) => {
                // Identifier 语句解析
                declarations::parse_identifier_stmt(self, start_span)
            }
            // Eof - no statement to parse
            Some(TokenKind::Eof) | None => None,
            // expression statement
            Some(_) => declarations::parse_expr_stmt(self, start_span),
        }
    }
}
