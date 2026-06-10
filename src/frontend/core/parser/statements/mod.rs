//! Statement parsing modules
//! Contains specialized modules for different statement types

pub mod bindings;
pub mod control_flow;
pub mod declarations;
pub mod functions;
pub mod imports;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export commonly used items
pub use types::*;
pub use declarations::*;
pub use functions::*;
pub use imports::*;
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
use crate::util::diagnostic::ErrorCodeDefinition;
use crate::frontend::core::parser::parse_msg;

impl StatementParser for ParserState<'_> {
    fn parse_statement(&mut self) -> Option<Stmt> {
        let start_span = self.span();

        match self.current().map(|t| &t.kind) {
            // RFC-010: 'type' keyword removed
            // Type definitions use `Name: Type = { ... }` syntax (handled by parse_identifier_stmt)
            // use import
            Some(TokenKind::KwUse) => imports::parse_use_stmt(self, start_span),
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
            // tuple destructuring with parens: (a, b) = expr
            Some(TokenKind::LParen) => declarations::parse_paren_destructure_stmt(self, start_span),
            // Eof - no statement to parse
            Some(TokenKind::Eof) | None => None,
            // Phase 1: @ 不再是有效的语句起始（eval block 已移除）
            Some(TokenKind::At) => {
                self.error(
                    ErrorCodeDefinition::unexpected_token("@")
                        .at(start_span)
                        .build(),
                );
                None
            }
            // 关键字不能用作变量名或表达式的语句开头
            Some(kw @ TokenKind::KwRef)
            | Some(kw @ TokenKind::KwUnsafe)
            | Some(kw @ TokenKind::KwElif)
            | Some(kw @ TokenKind::KwElse)
            | Some(kw @ TokenKind::KwIn)
            | Some(kw @ TokenKind::KwAs) => {
                let keyword = match kw {
                    TokenKind::KwRef => "ref",
                    TokenKind::KwUnsafe => "unsafe",
                    TokenKind::KwElif => "elif",
                    TokenKind::KwElse => "else",
                    TokenKind::KwIn => "in",
                    TokenKind::KwAs => "as",
                    _ => "keyword",
                };
                self.error(parse_msg(format!(
                    "'{}' 是关键字，不能用作变量名或表达式",
                    keyword
                )));
                self.bump();
                None
            }
            // expression statement
            Some(_) => declarations::parse_expr_stmt(self, start_span),
        }
    }
}
