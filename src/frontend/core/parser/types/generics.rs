//! Generic type parsing with RFC-010/011 support
//!
//! This module implements generic type parsing including constraints and trait bounds.

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::statements::ParserState;

/// Generic parameter parsing
pub struct GenericParamParser<'a> {
    state: &'a mut ParserState<'a>,
}

impl<'a> GenericParamParser<'a> {
    pub fn new(state: &'a mut ParserState<'a>) -> Self {
        Self { state }
    }

    /// Parse generic parameters like `[T]`, `[T: Clone]`, `[T, U: Add]`
    pub fn parse_generic_params(&mut self) -> Option<Vec<GenericParam>> {
        if !self.state.at(&TokenKind::LBracket) {
            return None;
        }

        self.state.bump(); // consume '['

        let mut params = Vec::new();

        if !self.state.at(&TokenKind::RBracket) {
            loop {
                let param = self.parse_single_generic_param()?;
                params.push(param);

                if !self.state.skip(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.state.skip(&TokenKind::RBracket); // consume ']'

        Some(params)
    }

    /// Parse a single generic parameter
    fn parse_single_generic_param(&mut self) -> Option<GenericParam> {
        // Parse parameter name
        let name = match self.state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => {
                let name = name.clone();
                self.state.bump();
                name
            }
            _ => return None,
        };

        // Parse optional bounds/constraints
        let bounds = if self.state.skip(&TokenKind::Colon) {
            self.parse_type_bounds()?
        } else {
            Vec::new()
        };

        Some(GenericParam {
            name,
            bounds,
        })
    }

    /// Parse type bounds like `Clone + Add`
    fn parse_type_bounds(&mut self) -> Option<Vec<TypeBound>> {
        let mut bounds = Vec::new();

        loop {
            let bound = self.parse_type_bound()?;
            bounds.push(bound);

            if !self.state.skip(&TokenKind::Plus) {
                break;
            }
        }

        Some(bounds)
    }

    /// Parse a single type bound
    fn parse_type_bound(&mut self) -> Option<TypeBound> {
        let name = match self.state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => {
                let name = name.clone();
                self.state.bump();
                name
            }
            _ => return None,
        };

        // Parse optional generic arguments for the bound
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

/// Extension trait for generic parsing
pub trait GenericParser {
    /// Parse generic parameters
    fn parse_generic_params(&mut self) -> Option<Vec<GenericParam>>;
}

impl<'a> GenericParser for ParserState<'a> {
    fn parse_generic_params(&mut self) -> Option<Vec<GenericParam>> {
        let mut parser = GenericParamParser::new(self);
        parser.parse_generic_params()
    }
}
