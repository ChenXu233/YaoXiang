//! Type parsing implementation
//!
//! This module implements type parsing with support for RFC-010/011 generics.

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::statements::ParserState;
use crate::util::span::Span;

/// Extension trait for type parsing
pub trait TypeParser {
    /// Parse a type expression
    fn parse_type(&mut self) -> Option<Type>;
}

impl<'a> TypeParser for ParserState<'a> {
    fn parse_type(&mut self) -> Option<Type> {
        self.parse_type_annotation()
    }
}

impl<'a> ParserState<'a> {
    /// Parse a type annotation
    pub fn parse_type_annotation(&mut self) -> Option<Type> {
        match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(_)) => self.parse_simple_type(),
            Some(TokenKind::LParen) => self.parse_tuple_type(),
            Some(TokenKind::LBrace) => self.parse_struct_type(),
            _ => None,
        }
    }

    /// Parse a simple type (identifier)
    fn parse_simple_type(&mut self) -> Option<Type> {
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => {
                let name = name.clone();
                self.bump();
                name
            }
            _ => return None,
        };

        // Check for generic parameters
        if self.at(&TokenKind::Lt) {
            return self.parse_generic_type(name);
        }

        Some(Type::Name(name))
    }

    /// Parse a generic type like `Vec<T>` or `HashMap<K, V>`
    fn parse_generic_type(&mut self, name: String) -> Option<Type> {
        self.skip(&TokenKind::Lt); // consume '<'

        let mut args = Vec::new();

        if !self.at(&TokenKind::Gt) {
            loop {
                if let Some(arg) = self.parse_type_annotation() {
                    args.push(arg);
                } else {
                    break;
                }

                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.skip(&TokenKind::Gt); // consume '>'

        Some(Type::Generic {
            name,
            args,
        })
    }

    /// Parse a tuple type like `(T, U, V)`
    fn parse_tuple_type(&mut self) -> Option<Type> {
        self.skip(&TokenKind::LParen); // consume '('

        let mut types = Vec::new();

        if !self.at(&TokenKind::RParen) {
            loop {
                if let Some(ty) = self.parse_type_annotation() {
                    types.push(ty);
                } else {
                    break;
                }

                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.skip(&TokenKind::RParen); // consume ')'

        Some(Type::Tuple(types))
    }

    /// Parse a struct type like `{ field: Type }`
    fn parse_struct_type(&mut self) -> Option<Type> {
        self.skip(&TokenKind::LBrace); // consume '{'

        let mut fields = Vec::new();

        if !self.at(&TokenKind::RBrace) {
            loop {
                let field_name = match self.current().map(|t| &t.kind) {
                    Some(TokenKind::Identifier(name)) => {
                        let name = name.clone();
                        self.bump();
                        name
                    }
                    _ => break,
                };

                self.skip(&TokenKind::Colon);

                let field_type = self.parse_type_annotation()?;

                fields.push((field_name, field_type));

                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.skip(&TokenKind::RBrace); // consume '}'

        Some(Type::Struct(fields))
    }

    /// Parse a function type like `fn(T, U) -> V`
    pub fn parse_function_type(&mut self) -> Option<Type> {
        if !self.at(&TokenKind::KwFn) {
            return None;
        }

        self.bump(); // consume 'fn'

        self.skip(&TokenKind::LParen);

        let mut params = Vec::new();

        if !self.at(&TokenKind::RParen) {
            loop {
                if let Some(param_type) = self.parse_type_annotation() {
                    params.push(param_type);
                } else {
                    break;
                }

                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.skip(&TokenKind::RParen);

        let return_type = if self.skip(&TokenKind::Arrow) {
            self.parse_type_annotation()
        } else {
            None
        };

        Some(Type::Function {
            params,
            return_type: Box::new(return_type.unwrap_or(Type::Void)),
        })
    }
}
