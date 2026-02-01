//! Type-related statement parsing
//!
//! Type parsing is implemented in declarations::parse_type_annotation.
//! This module provides the TypeStatementParser trait for convenience.

use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::ParserState;
use crate::frontend::core::parser::statements::declarations::parse_type_annotation;

/// Extension trait for type parsing
pub trait TypeStatementParser {
    /// Parse a type annotation
    fn parse_type_annotation(&mut self) -> Option<Type>;
}

impl TypeStatementParser for ParserState<'_> {
    fn parse_type_annotation(&mut self) -> Option<Type> {
        parse_type_annotation(self)
    }
}
