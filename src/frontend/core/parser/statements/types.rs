//! Type-related statement parsing

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::ParserState;

/// Extension trait for type parsing
pub trait TypeStatementParser {
    /// Parse a type annotation
    fn parse_type_annotation(&mut self) -> Option<Type>;
}

impl TypeStatementParser for ParserState<'_> {
    fn parse_type_annotation(&mut self) -> Option<Type> {
        match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(_)) => self.parse_simple_type(),
            Some(TokenKind::LParen) => self.parse_tuple_type(),
            Some(TokenKind::LBrace) => self.parse_struct_type(),
            _ => None,
        }
    }
}

impl ParserState<'_> {
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
    fn parse_generic_type(
        &mut self,
        name: String,
    ) -> Option<Type> {
        self.skip(&TokenKind::Lt); // consume '<'

        let mut args = Vec::new();

        if !self.at(&TokenKind::Gt) {
            while let Some(arg) = self.parse_type_annotation() {
                args.push(arg);

                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.skip(&TokenKind::Gt); // consume '>'

        Some(Type::Generic { name, args })
    }

    /// Parse a tuple type like `(T, U, V)`
    fn parse_tuple_type(&mut self) -> Option<Type> {
        self.skip(&TokenKind::LParen); // consume '('

        let mut types = Vec::new();

        if !self.at(&TokenKind::RParen) {
            while let Some(ty) = self.parse_type_annotation() {
                types.push(ty);

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
            while let Some(TokenKind::Identifier(name)) = self.current().map(|t| &t.kind) {
                let name = name.clone();
                self.bump();

                self.skip(&TokenKind::Colon);

                let field_type = self.parse_type_annotation()?;

                fields.push((name, field_type));

                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.skip(&TokenKind::RBrace); // consume '}'

        Some(Type::Struct(fields))
    }
}
