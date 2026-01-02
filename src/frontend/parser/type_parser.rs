//! Type annotation parsing

use super::state::*;
use super::ast::*;
use super::super::lexer::tokens::*;
use crate::util::span::Span;

impl<'a> ParserState<'a> {
    /// Parse a type annotation
    #[inline]
    pub fn parse_type_anno(&mut self) -> Option<Type> {
        self.parse_type()
    }

    /// Parse a type
    pub fn parse_type(&mut self) -> Option<Type> {
        let start_span = self.span();

        match self.current().map(|t| &t.kind) {
            // Function type: (param_types) -> return_type (using parentheses without fn keyword)
            Some(TokenKind::LParen) => self.parse_tuple_or_parens_type(start_span),
            // List type: [Type]
            Some(TokenKind::LBracket) => self.parse_list_type(start_span),
            // Named type or generic type (including: void, bool, char, string, bytes, int, float)
            Some(TokenKind::Identifier(_)) => self.parse_named_or_generic_type(start_span),
            _ => None,
        }
    }

    /// Parse function type: `fn(...) -> ...`
    fn parse_fn_type(&mut self, _span: Span) -> Option<Type> {
        self.bump(); // consume 'fn'

        if !self.expect(&TokenKind::LParen) {
            return None;
        }

        let params = self.parse_type_list()?;
        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        let return_type = if self.skip(&TokenKind::Arrow) {
            Box::new(self.parse_type()?)
        } else {
            Box::new(Type::Void)
        };

        Some(Type::Fn {
            params,
            return_type,
        })
    }

    /// Parse tuple or parenthesized type
    fn parse_tuple_or_parens_type(&mut self, _span: Span) -> Option<Type> {
        self.bump(); // consume '('

        // Empty tuple: ()
        if self.at(&TokenKind::RParen) {
            self.bump();
            return Some(Type::Tuple(vec![]));
        }

        let first = self.parse_type()?;

        // Single type in parens: (Type)
        if self.at(&TokenKind::RParen) {
            self.bump();
            return Some(first);
        }

        // Tuple type: (Type1, Type2, ...)
        if self.skip(&TokenKind::Comma) {
            let mut types = vec![first];
            while !self.at(&TokenKind::RParen) && !self.at_end() {
                types.push(self.parse_type()?);
                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
            if !self.expect(&TokenKind::RParen) {
                return None;
            }
            return Some(Type::Tuple(types));
        }

        // Just parenthesized type: (Type)
        if !self.expect(&TokenKind::RParen) {
            return None;
        }
        Some(first)
    }

    /// Parse list type: `[Type]`
    fn parse_list_type(&mut self, _span: Span) -> Option<Type> {
        self.bump(); // consume '['

        let inner_type = self.parse_type()?;
        if !self.expect(&TokenKind::RBracket) {
            return None;
        }

        Some(Type::List(Box::new(inner_type)))
    }

    /// Parse named type or generic type
    fn parse_named_or_generic_type(&mut self, _span: Span) -> Option<Type> {
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => return None,
        };
        self.bump();

        // Check for generic arguments: List<Int> or Dict<String, Int>
        if self.at(&TokenKind::Lt) {
            self.bump(); // consume '<'

            let args = self.parse_generic_args()?;
            if !self.expect(&TokenKind::Gt) {
                return None;
            }

            return Some(Type::Generic { name, args });
        }

        // Built-in type name mapping (can be shadowed by user-defined types)
        match name.as_str() {
            "void" => Some(Type::Void),
            "bool" => Some(Type::Bool),
            "char" => Some(Type::Char),
            "string" => Some(Type::String),
            "bytes" => Some(Type::Bytes),
            "int" => self.parse_int_type_from_name(),
            "float" => self.parse_float_type_from_name(),
            _ => Some(Type::Name(name)),
        }
    }

    /// Parse integer type with optional bit width (called from parse_named_or_generic_type)
    fn parse_int_type_from_name(&mut self) -> Option<Type> {
        // Check for bit width: Int<32>, Int<64>
        if self.at(&TokenKind::Lt) {
            self.bump(); // consume '<'

            let size = match self.current().map(|t| &t.kind) {
                Some(TokenKind::IntLiteral(n)) => {
                    let s = *n as usize;
                    self.bump();
                    s
                }
                _ => {
                    self.error(super::ParseError::UnexpectedToken(
                        self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
                    ));
                    return Some(Type::Int(64)); // default
                }
            };

            if !self.expect(&TokenKind::Gt) {
                return None;
            }
            return Some(Type::Int(size));
        }

        Some(Type::Int(64)) // default to 64-bit
    }

    /// Parse float type with optional bit width (called from parse_named_or_generic_type)
    fn parse_float_type_from_name(&mut self) -> Option<Type> {
        // Check for bit width: Float<32>, Float<64>
        if self.at(&TokenKind::Lt) {
            self.bump(); // consume '<'

            let size = match self.current().map(|t| &t.kind) {
                Some(TokenKind::IntLiteral(n)) => {
                    let s = *n as usize;
                    self.bump();
                    s
                }
                _ => {
                    self.error(super::ParseError::UnexpectedToken(
                        self.current().map(|t: &Token| t.kind.clone()).unwrap_or(TokenKind::Eof),
                    ));
                    return Some(Type::Float(64)); // default
                }
            };

            if !self.expect(&TokenKind::Gt) {
                return None;
            }
            return Some(Type::Float(size));
        }

        Some(Type::Float(64)) // default to 64-bit
    }

    /// Parse generic type arguments
    fn parse_generic_args(&mut self) -> Option<Vec<Type>> {
        let mut args = Vec::new();

        while !self.at(&TokenKind::Gt) && !self.at_end() {
            if !args.is_empty() {
                if !self.expect(&TokenKind::Comma) {
                    return None;
                }
            }

            args.push(self.parse_type()?);
        }

        Some(args)
    }

    /// Parse a list of types (for function parameters)
    pub(crate) fn parse_type_list(&mut self) -> Option<Vec<Type>> {
        let mut types = Vec::new();

        while !self.at(&TokenKind::RParen) && !self.at_end() {
            if !types.is_empty() {
                if !self.expect(&TokenKind::Comma) {
                    return None;
                }
            }

            types.push(self.parse_type()?);
        }

        Some(types)
    }
}
