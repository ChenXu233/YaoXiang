//! Type annotation parsing

use super::super::lexer::tokens::*;
use super::ast::*;
use super::state::*;
use crate::util::span::Span;

impl<'a> ParserState<'a> {
    /// Parse a type annotation
    #[inline]
    pub fn parse_type_anno(&mut self) -> Option<Type> {
        // Consume optional leading generic parameter list like `<T, U>`
        if self.at(&TokenKind::Lt) {
            let mut depth = 1;
            self.bump(); // consume '<'
            while depth > 0 {
                match self.current().map(|t| &t.kind) {
                    Some(TokenKind::Lt) => {
                        depth += 1;
                        self.bump();
                    }
                    Some(TokenKind::Gt) => {
                        depth -= 1;
                        self.bump();
                    }
                    Some(_) => {
                        self.bump();
                    }
                    None => break,
                }
            }
        }

        self.parse_type()
    }

    /// Parse a type
    pub fn parse_type(&mut self) -> Option<Type> {
        let start_span = self.span();

        let mut ty = match self.current().map(|t| &t.kind) {
            // Function type: (param_types) -> return_type (using parentheses without fn keyword)
            Some(TokenKind::LParen) => self.parse_tuple_or_parens_type(start_span),
            // List type: [Type]
            Some(TokenKind::LBracket) => self.parse_list_type(start_span),
            // Struct type: { field: Type, ... }
            Some(TokenKind::LBrace) => self.parse_struct_type(start_span),
            // Named type or generic type (including: void, bool, char, string, bytes, int, float)
            Some(TokenKind::Identifier(_)) => self.parse_named_or_generic_type(start_span),
            _ => return None,
        }?;

        // Handle function type arrow: T -> R
        // This handles `Int -> Int` or `[Int] -> Int`
        if self.skip(&TokenKind::Arrow) {
            let return_type = self.parse_type()?;
            ty = Type::Fn {
                params: vec![ty],
                return_type: Box::new(return_type),
            };
        }

        Some(ty)
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
            // Check for function type: () -> Ret
            if self.skip(&TokenKind::Arrow) {
                let return_type = Box::new(self.parse_type()?);
                return Some(Type::Fn {
                    params: vec![],
                    return_type,
                });
            }
            return Some(Type::Tuple(vec![]));
        }

        let first = self.parse_type()?;
        let mut types = vec![first];

        // Tuple type: (Type1, Type2, ...)
        if self.skip(&TokenKind::Comma) {
            while !self.at(&TokenKind::RParen) && !self.at_end() {
                types.push(self.parse_type()?);
                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
        }

        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        // Check for function type: (T1, T2) -> Ret
        if self.skip(&TokenKind::Arrow) {
            let return_type = Box::new(self.parse_type()?);
            return Some(Type::Fn {
                params: types,
                return_type,
            });
        }

        // If multiple types, it's a tuple
        if types.len() > 1 {
            return Some(Type::Tuple(types));
        }

        // Single type in parens: (Type) -> Tuple([Type])
        // This matches the test expectation that (int) is a tuple of 1 element.
        Some(Type::Tuple(types))
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

    /// Parse struct type: `{ field: Type, ... }`
    fn parse_struct_type(&mut self, _span: Span) -> Option<Type> {
        self.bump(); // consume '{'

        let mut fields = Vec::new();

        while !self.at(&TokenKind::RBrace) && !self.at_end() {
            let name = match self.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => n.clone(),
                _ => return None,
            };
            self.bump();

            if !self.expect(&TokenKind::Colon) {
                return None;
            }

            let ty = self.parse_type()?;
            fields.push((name, ty));

            if !self.skip(&TokenKind::Comma) {
                break;
            }
        }

        if !self.expect(&TokenKind::RBrace) {
            return None;
        }

        Some(Type::Struct(fields))
    }

    /// Parse named type or generic type
    fn parse_named_or_generic_type(&mut self, _span: Span) -> Option<Type> {
        let mut name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => return None,
        };
        self.bump();

        // Handle qualified names: std.io.Reader
        while self.skip(&TokenKind::Dot) {
            match self.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => {
                    name.push('.');
                    name.push_str(n);
                    self.bump();
                }
                _ => return None, // Expected identifier after dot
            }
        }

        // Built-in type name mapping (can be shadowed by user-defined types)
        // Check built-ins first to handle int<32> correctly
        match name.as_str() {
            "void" | "Void" => return Some(Type::Void),
            "bool" | "Bool" => return Some(Type::Bool),
            "char" | "Char" => return Some(Type::Char),
            "string" | "String" => return Some(Type::String),
            "bytes" | "Bytes" => return Some(Type::Bytes),
            "int" | "Int" => return self.parse_int_type_from_name(),
            "float" | "Float" => return self.parse_float_type_from_name(),
            _ => {}
        }

        // Check for generic arguments or struct fields
        let (_open, close) = if self.at(&TokenKind::Lt) {
            (TokenKind::Lt, TokenKind::Gt)
        } else if self.at(&TokenKind::LBracket) {
            (TokenKind::LBracket, TokenKind::RBracket)
        } else if self.at(&TokenKind::LParen) {
            (TokenKind::LParen, TokenKind::RParen)
        } else {
            return Some(Type::Name(name));
        };

        self.bump(); // consume open

        let mut args = Vec::new();
        let mut named_fields = Vec::new();
        let mut is_named = false;

        // Check if first arg is named
        if !self.at(&close) && !self.at_end() {
            if let Some(TokenKind::Identifier(_)) = self.current().map(|t| &t.kind) {
                if matches!(self.peek().map(|t| &t.kind), Some(TokenKind::Colon)) {
                    is_named = true;
                }
            }
        }

        while !self.at(&close) && !self.at_end() {
            if !args.is_empty() || !named_fields.is_empty() {
                if !self.expect(&TokenKind::Comma) {
                    return None;
                }
            }

            if is_named {
                let field_name = match self.current().map(|t| &t.kind) {
                    Some(TokenKind::Identifier(n)) => n.clone(),
                    _ => return None,
                };
                self.bump();
                if !self.expect(&TokenKind::Colon) {
                    return None;
                }
                let ty = self.parse_type()?;
                named_fields.push((field_name, ty));
            } else {
                args.push(self.parse_type()?);
            }
        }

        if !self.expect(&close) {
            return None;
        }

        if is_named {
            Some(Type::NamedStruct {
                name,
                fields: named_fields,
            })
        } else {
            Some(Type::Generic { name, args })
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
                        self.current()
                            .map(|t| t.kind.clone())
                            .unwrap_or(TokenKind::Eof),
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
                        self.current()
                            .map(|t: &Token| t.kind.clone())
                            .unwrap_or(TokenKind::Eof),
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

            // Check for trailing comma
            if self.at(&TokenKind::RParen) {
                break;
            }

            let ty = self.parse_type()?;
            types.push(ty);
        }

        Some(types)
    }
}
