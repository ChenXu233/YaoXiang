//! Type constraint parsing with RFC-011 support
//!
//! This module implements type constraint parsing for RFC-011 generic constraints.

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::statements::ParserState;

/// Constraint parsing for RFC-011
pub struct ConstraintParser<'a> {
    state: &'a mut ParserState<'a>,
}

impl<'a> ConstraintParser<'a> {
    pub fn new(state: &'a mut ParserState<'a>) -> Self {
        Self { state }
    }

    /// Parse where clauses like `where T: Clone + Add`
    pub fn parse_where_clause(&mut self) -> Option<Vec<TypeConstraint>> {
        if !self.state.at(&TokenKind::KwWhere) {
            return None;
        }

        self.state.bump(); // consume 'where'

        let mut constraints = Vec::new();

        loop {
            let constraint = self.parse_single_constraint()?;
            constraints.push(constraint);

            if !self.state.skip(&TokenKind::Comma) {
                break;
            }
        }

        Some(constraints)
    }

    /// Parse a single type constraint
    fn parse_single_constraint(&mut self) -> Option<TypeConstraint> {
        // Parse the type parameter
        let param = match self.state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => {
                let name = name.clone();
                self.state.bump();
                name
            }
            _ => return None,
        };

        self.state.skip(&TokenKind::Colon); // consume ':'

        // Parse the bounds
        let mut bounds = Vec::new();

        loop {
            let bound = self.parse_type_bound()?;
            bounds.push(bound);

            if !self.state.skip(&TokenKind::Plus) {
                break;
            }
        }

        Some(TypeConstraint { param, bounds })
    }

    /// Parse a type bound
    fn parse_type_bound(&mut self) -> Option<TypeBound> {
        let name = match self.state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => {
                let name = name.clone();
                self.state.bump();
                name
            }
            _ => return None,
        };

        // Parse optional generic arguments
        let args = if self.state.at(&TokenKind::Lt) {
            self.state.bump(); // consume '<'
            let mut args = Vec::new();

            if !self.state.at(&TokenKind::Gt) {
                loop {
                    let arg = self.state.parse_type_annotation()?;
                    args.push(arg);

                    if !self.state.skip(&TokenKind::Comma) {
                        break;
                    }
                }
            }

            self.state.skip(&TokenKind::Gt); // consume '>'
            args
        } else {
            Vec::new()
        };

        Some(TypeBound { name, args })
    }
}

/// Extension trait for constraint parsing
pub trait ConstraintParser {
    /// Parse where clause
    fn parse_where_clause(&mut self) -> Option<Vec<TypeConstraint>>;
}

impl<'a> ConstraintParser for ParserState<'a> {
    fn parse_where_clause(&mut self) -> Option<Vec<TypeConstraint>> {
        let mut parser = ConstraintParser::new(self);
        parser.parse_where_clause()
    }
}
