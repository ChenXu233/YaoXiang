//! Statement parsing modules
//! Contains specialized modules for different statement types

pub mod bindings;
pub mod control_flow;
pub mod declarations;
pub mod trait_def;
pub mod types; // Trait 定义/实现解析

// Re-export commonly used items
pub use types::*;
pub use declarations::*;
pub use control_flow::*;
pub use bindings::*;
// Trait 解析函数通过 trait_def 模块访问

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
            // type definition
            Some(TokenKind::KwType) => {
                // 区分类型定义和 Trait 定义
                if trait_def::is_trait_def_stmt(self) {
                    trait_def::parse_trait_def_stmt(self, start_span)
                } else {
                    declarations::parse_type_stmt(self, start_span)
                }
            }
            // Trait 实现 - 通过标识符解析处理 (impl 是标识符，不是关键字)
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
                // 检测是否是 impl 语句: `impl TraitName for Type { ... }`
                if trait_def::is_trait_impl_stmt(self) {
                    trait_def::parse_trait_impl_stmt(self, start_span)
                } else {
                    declarations::parse_identifier_stmt(self, start_span)
                }
            }
            // Eof - no statement to parse
            Some(TokenKind::Eof) | None => None,
            // expression statement
            Some(_) => declarations::parse_expr_stmt(self, start_span),
        }
    }
}
