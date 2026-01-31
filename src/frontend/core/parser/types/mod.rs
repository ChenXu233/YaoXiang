//! Type parsing module
//!
//! This module contains type parsing logic with support for RFC-010/011 generics.

pub mod parser;
pub mod generics;
pub mod constraints;

pub use parser::*;
pub use generics::*;
pub use constraints::*;

/// Type parser state
pub struct TypeParser<'a> {
    state: &'a mut crate::frontend::core::parser::statements::ParserState<'a>,
}

impl<'a> TypeParser<'a> {
    pub fn new(state: &'a mut crate::frontend::core::parser::statements::ParserState<'a>) -> Self {
        Self { state }
    }

    /// Parse a type expression
    pub fn parse_type(&mut self) -> Option<crate::frontend::core::parser::ast::Type> {
        self.parse_simple_type()
    }

    /// Parse a simple type (identifier or generic type)
    fn parse_simple_type(&mut self) -> Option<crate::frontend::core::parser::ast::Type> {
        let name = match self.state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => {
                let name = name.clone();
                self.state.bump();
                name
            }
            _ => return None,
        };

        Some(crate::frontend::core::parser::ast::Type::Name(name))
    }
}
