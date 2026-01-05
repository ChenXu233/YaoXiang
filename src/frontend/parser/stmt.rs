//! Statement parsing

use super::super::lexer::tokens::*;
use super::ast::*;
use super::state::*;
use crate::util::span::Span;

impl<'a> ParserState<'a> {
    /// Parse a statement
    #[inline]
    pub fn parse_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.span();

        match self.current().map(|t| &t.kind) {
            // type definition
            Some(TokenKind::KwType) => self.parse_type_stmt(start_span),
            // use import
            Some(TokenKind::KwUse) => self.parse_use_stmt(start_span),
            // return statement
            Some(TokenKind::KwReturn) => self.parse_return_stmt(start_span),
            // break statement
            Some(TokenKind::KwBreak) => self.parse_break_stmt(start_span),
            // continue statement
            Some(TokenKind::KwContinue) => self.parse_continue_stmt(start_span),
            // for loop
            Some(TokenKind::KwFor) => self.parse_for_stmt(start_span),
            // block as statement
            Some(TokenKind::LBrace) => self.parse_block_stmt(start_span),
            // variable declaration: [mut] identifier [: type] [= expr]
            Some(TokenKind::KwMut) => self.parse_var_stmt(start_span),
            Some(TokenKind::Identifier(_)) => {
                // Check if this is a function definition: name(types) -> type = (params) => body
                // Or a simple assignment/expression: name = expr or just name expr
                eprintln!(
                    "[DEBUG] parse_stmt: calling parse_identifier_stmt, current: {:?}",
                    self.current().map(|t| &t.kind)
                );
                let result = self.parse_identifier_stmt(start_span);
                eprintln!(
                    "[DEBUG] parse_stmt: parse_identifier_stmt returned: {:?}",
                    result.is_some()
                );
                result
            },
            // expression statement
            Some(_) => self.parse_expr_stmt(start_span),
            None => None,
        }
    }

    /// Parse variable declaration: `[mut] name[: type] [= expr];`
    /// New syntax: `x: int = 42` or `mut y: int = 10`
    fn parse_var_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        // Check for mutability
        let is_mut = self.skip(&TokenKind::KwMut);

        // Parse variable name (identifier)
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                ));
                return None;
            },
        };
        self.bump();

        // Optional type annotation
        let type_annotation = if self.skip(&TokenKind::Colon) {
            self.parse_type_anno()
        } else {
            None
        };

        // Optional initializer
        let initializer = if self.skip(&TokenKind::Eq) {
            Some(Box::new(self.parse_expression(BP_LOWEST)?))
        } else {
            None
        };

        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::Var {
                name,
                type_annotation,
                initializer,
                is_mut,
            },
            span,
        })
    }

    /// Parse type definition: `type Name = Type;`
    /// Supports:
    /// - Simple type: `type Color = red`
    /// - Union type: `type Color = red | green | blue`
    /// - Generic union: `type Result[T, E] = ok(T) | err(E)`
    /// - Struct type: `type Point = Point(x: Float, y: Float)`
    fn parse_type_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        self.bump(); // consume 'type'

        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                ));
                return None;
            },
        };
        self.bump();

        // Parse generic parameters: type Result[T, E] = ...
        let _generic_params = self.parse_type_generic_params()?;

        if !self.expect(&TokenKind::Eq) {
            return None;
        }

        let definition = self.parse_type_definition()?;

        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::TypeDef { name, definition },
            span,
        })
    }

    /// Parse generic parameters for type definition: [T, E] or <T, E>
    fn parse_type_generic_params(&mut self) -> Option<Vec<String>> {
        let open = if self.at(&TokenKind::LBracket) {
            self.bump();
            TokenKind::RBracket
        } else if self.at(&TokenKind::Lt) {
            self.bump();
            TokenKind::Gt
        } else {
            return Some(Vec::new());
        };

        let mut params = Vec::new();
        while !self.at(&open) && !self.at_end() {
            if let Some(TokenKind::Identifier(n)) = self.current().map(|t| &t.kind) {
                params.push(n.clone());
                self.bump();
                self.skip(&TokenKind::Comma);
            } else {
                break;
            }
        }

        if !self.expect(&open) {
            return None;
        }

        Some(params)
    }

    /// Parse type definition (handles union types with |)
    fn parse_type_definition(&mut self) -> Option<Type> {
        let first_type = self.parse_type_anno()?;

        if self.at(&TokenKind::Pipe) {
            let mut types = vec![first_type];
            while self.skip(&TokenKind::Pipe) {
                types.push(self.parse_type_anno()?);
            }

            // Check if all types are variant-like (Name, Generic, or NamedStruct)
            let all_variants = types.iter().all(|t| {
                matches!(
                    t,
                    Type::Name(_) | Type::Generic { .. } | Type::NamedStruct { .. }
                )
            });

            if all_variants {
                // Try to convert to VariantDef
                let mut variants = Vec::new();
                for ty in types.iter() {
                    match ty {
                        Type::Generic { name, args } => {
                            let params = args.iter().map(|a| (None, a.clone())).collect();
                            variants.push(VariantDef {
                                name: name.clone(),
                                params,
                                span: self.span(),
                            });
                        },
                        Type::NamedStruct { name, fields } => {
                            let params = fields
                                .iter()
                                .map(|(n, t)| (Some(n.clone()), t.clone()))
                                .collect();
                            variants.push(VariantDef {
                                name: name.clone(),
                                params,
                                span: self.span(),
                            });
                        },
                        Type::Name(name) => {
                            variants.push(VariantDef {
                                name: name.clone(),
                                params: Vec::new(),
                                span: self.span(),
                            });
                        },
                        _ => unreachable!(),
                    }
                }
                return Some(Type::Variant(variants));
            } else {
                // Otherwise return Sum type
                return Some(Type::Sum(types));
            }
        }

        Some(first_type)
    }

    /// Parse a constructor: `Name` or `Name(params)`
    fn parse_constructor(&mut self) -> Option<VariantDef> {
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                ));
                return None;
            },
        };
        self.bump();

        // Check for constructor params: Point(x: Float, y: Float) or Box(Int)
        let params = if self.at(&TokenKind::LParen) {
            self.parse_constructor_params()?
        } else {
            Vec::new()
        };

        Some(VariantDef {
            name,
            params,
            span: self.span(),
        })
    }

    /// Parse constructor parameters: (x: Type, y: Type) or generic args: (Type1, Type2)
    fn parse_constructor_params(&mut self) -> Option<Vec<(Option<String>, Type)>> {
        if !self.expect(&TokenKind::LParen) {
            return None;
        }

        // Check if first element has a name (identifier followed by colon)
        // or is just a type (identifier followed by comma or rparen)
        let has_named_params = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(_)) => {
                // Look ahead to see if next token is Colon
                matches!(self.peek().map(|t| &t.kind), Some(TokenKind::Colon))
            },
            _ => false,
        };

        let mut params = Vec::new();

        if has_named_params {
            // Parse named fields: (x: Type, y: Type)
            while !self.at(&TokenKind::RParen) && !self.at_end() {
                // Parse parameter name
                let name = match self.current().map(|t| &t.kind) {
                    Some(TokenKind::Identifier(n)) => n.clone(),
                    _ => break,
                };
                self.bump();

                // Expect colon
                if !self.expect(&TokenKind::Colon) {
                    return None;
                }

                // Parse parameter type
                let ty = match self.parse_type_anno() {
                    Some(t) => t,
                    None => break,
                };

                params.push((Some(name), ty));

                // Expect comma or RParen
                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
        } else {
            // Parse type arguments: (Type1, Type2)
            // These are generic type arguments without names (None for name)
            while !self.at(&TokenKind::RParen) && !self.at_end() {
                // Parse type
                let ty = match self.parse_type_anno() {
                    Some(t) => t,
                    None => break,
                };

                // No name for type arguments
                params.push((None, ty));

                // Expect comma or RParen
                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
        }

        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        Some(params)
    }

    /// Parse module definition: `mod Name { ... }`
    fn parse_module_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        self.bump(); // consume 'mod'

        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                ));
                return None;
            },
        };
        self.bump();

        if !self.expect(&TokenKind::LBrace) {
            return None;
        }

        let mut items = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_end() {
            if let Some(stmt) = self.parse_stmt() {
                items.push(stmt);
            } else {
                self.synchronize();
            }
        }

        if !self.expect(&TokenKind::RBrace) {
            return None;
        }

        Some(Stmt {
            kind: StmtKind::Module { name, items },
            span,
        })
    }

    /// Parse use import: `use path;` or `use path::{item1, item2};`
    fn parse_use_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        self.bump(); // consume 'use'

        let path = self.parse_use_path()?;

        // Parse import items: use path::{item1, item2};
        let items = if self.skip(&TokenKind::LBrace) {
            let mut items = Vec::new();
            while !self.at(&TokenKind::RBrace) && !self.at_end() {
                match self.current().map(|t| &t.kind) {
                    Some(TokenKind::Identifier(n)) => {
                        items.push(n.clone());
                        self.bump();
                        self.skip(&TokenKind::Comma);
                    },
                    Some(TokenKind::KwPub) => {
                        // Skip 'pub' in import items
                        self.bump();
                    },
                    _ => break,
                }
            }
            self.expect(&TokenKind::RBrace);
            Some(items)
        } else {
            None
        };

        // Parse alias: use path as alias;
        let alias = if self.skip(&TokenKind::KwAs) {
            match self.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => {
                    let a = n.clone();
                    self.bump();
                    Some(a)
                },
                _ => None,
            }
        } else {
            None
        };

        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::Use { path, items, alias },
            span,
        })
    }

    /// Parse use path (dot-separated identifiers)
    fn parse_use_path(&mut self) -> Option<String> {
        let mut parts = Vec::new();

        while let Some(TokenKind::Identifier(n)) = self.current().map(|t| &t.kind) {
            parts.push(n.clone());
            self.bump();
            if !self.skip(&TokenKind::Dot) {
                break;
            }
        }

        if parts.is_empty() {
            self.error(super::ParseError::UnexpectedToken(
                self.current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
            ));
            None
        } else {
            Some(parts.join("::"))
        }
    }

    /// Parse statement starting with identifier: function definition or expression
    fn parse_identifier_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        // Look ahead to determine what kind of statement this is
        // 1. Function definition: name(types) -> type = (params) => body
        // 2. Variable declaration: name[: type] [= expr]
        // 3. Expression statement: name expr...

        // Use peek_nth to look ahead without consuming tokens
        let next = self.peek();

        // Check for potential function definition: identifier followed by LParen
        // Also handle optional generic parameters after the identifier, e.g. `name<T>(...)`.
        let mut pos_of_lparen: Option<usize> = None;
        if matches!(next.map(|t| &t.kind), Some(TokenKind::LParen)) {
            pos_of_lparen = Some(1);
        } else if matches!(next.map(|t| &t.kind), Some(TokenKind::Lt)) {
            // Find matching '>' for generic parameters and then check for '(' after it
            let mut depth = 1;
            let mut p = 2; // start after identifier and '<'
            while depth > 0 {
                match self.peek_nth(p).map(|t| &t.kind) {
                    Some(TokenKind::Lt) => {
                        depth += 1;
                        p += 1;
                    },
                    Some(TokenKind::Gt) => {
                        p += 1;
                        break;
                    },
                    Some(_) => {
                        p += 1;
                    },
                    None => break,
                }
            }

            // After finding '>', check if next token is LParen
            if matches!(self.peek_nth(p).map(|t| &t.kind), Some(TokenKind::LParen)) {
                pos_of_lparen = Some(p);
            }
        }

        if let Some(lparen_pos) = pos_of_lparen {
            // Look ahead to find matching RParen, then check what's after it
            let mut depth = 1;
            let mut pos = lparen_pos + 1; // Start after the found LParen

            while depth > 0 {
                match self.peek_nth(pos).map(|t| &t.kind) {
                    Some(TokenKind::LParen) => {
                        depth += 1;
                        pos += 1;
                    },
                    Some(TokenKind::RParen) => {
                        depth -= 1;
                        if depth > 0 {
                            pos += 1;
                        }
                    },
                    Some(TokenKind::Comma) => {
                        pos += 1;
                    },
                    Some(_) => {
                        pos += 1;
                    },
                    None => {
                        // Reached end of tokens
                        break;
                    },
                }
            }

            // Check what's after the RParen
            let rparen = self.peek_nth(pos);
            let after_rparen = self.peek_nth(pos + 1);

            eprintln!(
                "[DEBUG] parse_identifier_stmt: pos={}, rparen={:?}, after_rparen={:?}",
                pos,
                rparen.map(|t| &t.kind),
                after_rparen.map(|t| &t.kind)
            );

            // If current pos is RParen AND after it comes -> or =, this is a function definition
            if matches!(rparen.map(|t| &t.kind), Some(TokenKind::RParen))
                && matches!(
                    after_rparen.map(|t| &t.kind),
                    Some(TokenKind::Arrow) | Some(TokenKind::Eq)
                )
            {
                eprintln!("[DEBUG] parse_identifier_stmt: Detected function definition!");
                // This is a function definition - parse as expression
                return self.parse_fn_stmt(span);
            }

            eprintln!("[DEBUG] parse_identifier_stmt: Falling through to expression statement");

            // Otherwise, this is a function call or other expression
            // Just parse as expression (the Pratt parser will handle it)
            return self.parse_expr_stmt(span);
        }

        // Check if this is a variable declaration: name: type or name = expr
        // We check next token (peek) because current is identifier
        if matches!(
            next.map(|t| &t.kind),
            Some(TokenKind::Colon) | Some(TokenKind::Eq)
        ) {
            // Consume the identifier first
            let name = match self.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => n.clone(),
                _ => {
                    self.error(super::ParseError::UnexpectedToken(
                        self.current()
                            .map(|t| t.kind.clone())
                            .unwrap_or(TokenKind::Eof),
                    ));
                    return None;
                },
            };
            self.bump();

            // This is a variable declaration (without mut)

            // Optional type annotation
            let type_annotation = if self.skip(&TokenKind::Colon) {
                self.parse_type_anno()
            } else {
                None
            };

            // Check if this is a function definition: `name[: Type] = (params) => body`
            // Both with and without type annotation can be function definitions
            // Key difference from variable init:
            // - Function def: `= (identifier, ...) => body` (lambda with identifier params)
            // - Function def: `= identifier => body` (single param without parens)
            // - Variable init: `= (expr, ...)` or `= expr` (not a lambda)
            let next_after_type = self.peek_nth(0);
            if matches!(next_after_type.map(|t| &t.kind), Some(TokenKind::Eq)) {
                // Look ahead to check what comes after `=`
                let next_after_eq = self.peek_nth(1);

                // Case 1: `= (identifier, ...) => body` - multi-param function
                if matches!(next_after_eq.map(|t| &t.kind), Some(TokenKind::LParen)) {
                    // Check if the first thing in the parens is an identifier (parameter name)
                    let first_in_paren = self.peek_nth(2);

                    let is_fn_def = if matches!(
                        first_in_paren.map(|t| &t.kind),
                        Some(TokenKind::Identifier(_))
                    ) {
                        true
                    } else if matches!(first_in_paren.map(|t| &t.kind), Some(TokenKind::RParen)) {
                        // Check if followed by =>
                        matches!(self.peek_nth(3).map(|t| &t.kind), Some(TokenKind::FatArrow))
                    } else {
                        false
                    };

                    if is_fn_def {
                        // This is a function definition with parentheses!
                        let fn_span = span;

                        // Expect `=`
                        self.expect(&TokenKind::Eq);

                        // Parse lambda: `(params) => body`
                        if !self.expect(&TokenKind::LParen) {
                            return None;
                        }
                        let params = self.parse_fn_params()?;
                        if !self.expect(&TokenKind::RParen) {
                            return None;
                        }
                        if !self.expect(&TokenKind::FatArrow) {
                            return None;
                        }

                        // Parse body
                        let (stmts, expr) = if self.at(&TokenKind::LBrace) {
                            if !self.expect(&TokenKind::LBrace) {
                                return None;
                            }
                            let body = self.parse_block_body()?;
                            if !self.expect(&TokenKind::RBrace) {
                                return None;
                            }
                            body
                        } else {
                            let expr = self.parse_expression(BP_LOWEST)?;
                            (Vec::new(), Some(Box::new(expr)))
                        };

                        // Optional semicolon
                        self.skip(&TokenKind::Semicolon);

                        let type_annotation = type_annotation.map(|ty| match ty {
                            Type::Fn {
                                params,
                                return_type,
                            } => {
                                let mut fn_params = params;
                                if fn_params.len() == 1 {
                                    if let Type::Tuple(inner) = fn_params[0].clone() {
                                        fn_params = inner;
                                    }
                                }
                                Type::Fn {
                                    params: fn_params,
                                    return_type,
                                }
                            },
                            other => other,
                        });

                        return Some(Stmt {
                            kind: StmtKind::Fn {
                                name: name.clone(),
                                type_annotation,
                                params,
                                body: (stmts, expr),
                            },
                            span: fn_span,
                        });
                    }
                }
                // Case 2: `= identifier => body` - single param without parens
                // e.g., `inc: Int -> Int = x => x + 1`
                else if matches!(
                    next_after_eq.map(|t| &t.kind),
                    Some(TokenKind::Identifier(_))
                ) {
                    // Check if followed by =>
                    let after_identifier = self.peek_nth(2);
                    if matches!(after_identifier.map(|t| &t.kind), Some(TokenKind::FatArrow)) {
                        // This is a single-param function definition!
                        let fn_span = span;

                        // Expect `=`
                        self.expect(&TokenKind::Eq);

                        // Parse single parameter (without parentheses)
                        let param_span = self.span();
                        let param_name = match self.current().map(|t| &t.kind) {
                            Some(TokenKind::Identifier(n)) => n.clone(),
                            _ => {
                                self.error(super::ParseError::UnexpectedToken(
                                    self.current()
                                        .map(|t| t.kind.clone())
                                        .unwrap_or(TokenKind::Eof),
                                ));
                                return None;
                            },
                        };
                        self.bump();

                        // Expect fat arrow
                        if !self.expect(&TokenKind::FatArrow) {
                            return None;
                        }

                        // Parse body
                        let (stmts, expr) = if self.at(&TokenKind::LBrace) {
                            if !self.expect(&TokenKind::LBrace) {
                                return None;
                            }
                            let body = self.parse_block_body()?;
                            if !self.expect(&TokenKind::RBrace) {
                                return None;
                            }
                            body
                        } else {
                            let expr = self.parse_expression(BP_LOWEST)?;
                            (Vec::new(), Some(Box::new(expr)))
                        };

                        // Create single parameter with no type annotation
                        let params = vec![super::ast::Param {
                            name: param_name,
                            ty: None,
                            span: param_span,
                        }];

                        // Optional semicolon
                        self.skip(&TokenKind::Semicolon);

                        let type_annotation = type_annotation.map(|ty| match ty {
                            Type::Fn {
                                params,
                                return_type,
                            } => {
                                let mut fn_params = params;
                                if fn_params.len() == 1 {
                                    if let Type::Tuple(inner) = fn_params[0].clone() {
                                        fn_params = inner;
                                    }
                                }
                                Type::Fn {
                                    params: fn_params,
                                    return_type,
                                }
                            },
                            other => other,
                        });

                        return Some(Stmt {
                            kind: StmtKind::Fn {
                                name: name.clone(),
                                type_annotation,
                                params,
                                body: (stmts, expr),
                            },
                            span: fn_span,
                        });
                    }
                }
                // Not a function definition, fall through to variable initializer handling
            }

            // Check if this is a function definition: `name: Type = (params) => body`
            // Even without the standard pattern, check if `=` followed by lambda params
            if self.at(&TokenKind::Eq) {
                // Peek ahead to check if this is a lambda (function definition)
                let next_after_eq = self.peek_nth(1);
                if matches!(next_after_eq.map(|t| &t.kind), Some(TokenKind::LParen)) {
                    // This looks like a function definition: name: Type = (params) => body
                    // Check if the first thing in the parens is an identifier (parameter name)
                    let first_in_paren = self.peek_nth(2);

                    let is_fn_def = if matches!(
                        first_in_paren.map(|t| &t.kind),
                        Some(TokenKind::Identifier(_))
                    ) {
                        true
                    } else if matches!(first_in_paren.map(|t| &t.kind), Some(TokenKind::RParen)) {
                        // Check if followed by =>
                        matches!(self.peek_nth(3).map(|t| &t.kind), Some(TokenKind::FatArrow))
                    } else {
                        false
                    };

                    if is_fn_def {
                        // This is a function definition!
                        let fn_span = span;

                        // Expect `=`
                        self.expect(&TokenKind::Eq);

                        // Parse lambda: `(params) => body`
                        if !self.expect(&TokenKind::LParen) {
                            return None;
                        }
                        let params = self.parse_fn_params()?;
                        if !self.expect(&TokenKind::RParen) {
                            return None;
                        }
                        if !self.expect(&TokenKind::FatArrow) {
                            return None;
                        }

                        // Parse body
                        let (stmts, expr) = if self.at(&TokenKind::LBrace) {
                            if !self.expect(&TokenKind::LBrace) {
                                return None;
                            }
                            let body = self.parse_block_body()?;
                            if !self.expect(&TokenKind::RBrace) {
                                return None;
                            }
                            body
                        } else {
                            let expr = self.parse_expression(BP_LOWEST)?;
                            (Vec::new(), Some(Box::new(expr)))
                        };

                        // Optional semicolon
                        self.skip(&TokenKind::Semicolon);

                        let type_annotation = type_annotation.map(|ty| match ty {
                            Type::Fn {
                                params,
                                return_type,
                            } => {
                                let mut fn_params = params;
                                if fn_params.len() == 1 {
                                    if let Type::Tuple(inner) = fn_params[0].clone() {
                                        fn_params = inner;
                                    }
                                }
                                Type::Fn {
                                    params: fn_params,
                                    return_type,
                                }
                            },
                            other => other,
                        });

                        return Some(Stmt {
                            kind: StmtKind::Fn {
                                name: name.clone(),
                                type_annotation,
                                params,
                                body: (stmts, expr),
                            },
                            span: fn_span,
                        });
                    }
                }
            }

            // Optional initializer
            let initializer = if self.skip(&TokenKind::Eq) {
                let expr = self.parse_expression(BP_LOWEST)?;
                // Note: Type checking for lambda without type annotation
                // is deferred to the type checker, not the parser.
                // Parser is lenient: allows all syntactically valid declarations.
                Some(Box::new(expr))
            } else {
                None
            };

            // If we have a type annotation but no initializer and no semicolon,
            // check if the next token could be part of an invalid pattern
            // If next token is LParen followed by Identifier and FatArrow,
            // this looks like `name: Type (params) => body` without the `=`
            // which is invalid syntax
            if type_annotation.is_some()
                && initializer.is_none()
                && !self.skip(&TokenKind::Semicolon)
            {
                let next = self.current().map(|t| &t.kind);
                if matches!(next, Some(TokenKind::LParen)) {
                    // Look ahead to see if this is a lambda-like pattern without =
                    // Pattern: ( identifier ) => or ( identifier , ... ) =>
                    // For single param: ( a ) => - peek at 1, 2, 3
                    // For multi param: ( a , b ) => - peek at 1, 2, 3 (first comma or RParen)
                    let second = self.peek_nth(1);
                    let third = self.peek_nth(2);
                    let fourth = self.peek_nth(3);
                    // Check for pattern: ( identifier ) => or ( identifier , ...
                    let is_lambda_like =
                        if matches!(second.map(|t| &t.kind), Some(TokenKind::Identifier(_))) {
                            // Check if third is ) and fourth is => (single param case)
                            // Or third is , (multi param case)
                            matches!(third.map(|t| &t.kind), Some(TokenKind::RParen))
                                || matches!(third.map(|t| &t.kind), Some(TokenKind::Comma))
                        } else {
                            false
                        };
                    if is_lambda_like
                        && matches!(fourth.map(|t| &t.kind), Some(TokenKind::FatArrow))
                    {
                        // This is `name: Type (param) => body` without `=`
                        self.error(super::ParseError::Generic(
                            "Missing '=' before lambda in function definition".to_string(),
                        ));
                        return None;
                    }
                }
            }

            self.skip(&TokenKind::Semicolon);

            return Some(Stmt {
                kind: StmtKind::Var {
                    name,
                    type_annotation,
                    initializer,
                    is_mut: false,
                },
                span,
            });
        }

        // This is an expression starting with identifier
        // Parse the full expression starting with the identifier
        self.parse_expr_stmt(span)
    }

    /// Parse function definition: `name(types) -> type = (params) => body`
    /// Example: `add(Int, Int) -> Int = (a, b) => a + b`
    /// Also supports: `name() = (params) => body` (inferred types)
    fn parse_fn_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        eprintln!(
            "[DEBUG] parse_fn_stmt called, current token: {:?}",
            self.current().map(|t| &t.kind)
        );
        // Parse function name
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                ));
                return None;
            },
        };
        eprintln!("[DEBUG] parse_fn_stmt: function name = {}", name);
        self.bump();

        // Parse parameter types in parentheses: (Int, String) or ()
        eprintln!(
            "[DEBUG] parse_fn_stmt: expecting LParen, current: {:?}",
            self.current().map(|t| &t.kind)
        );
        if !self.expect(&TokenKind::LParen) {
            eprintln!("[DEBUG] parse_fn_stmt: failed to expect LParen");
            return None;
        }

        // Check if it's empty parens `()`
        let param_types = if self.at(&TokenKind::RParen) {
            Vec::new()
        } else {
            eprintln!("[DEBUG] parse_fn_stmt: parsing type list");
            self.parse_type_list()?
        };

        eprintln!("[DEBUG] parse_fn_stmt: param_types = {:?}", param_types);

        if !self.expect(&TokenKind::RParen) {
            eprintln!("[DEBUG] parse_fn_stmt: failed to expect RParen");
            return None;
        }

        // Parse return type: -> ReturnType
        let return_type = if self.skip(&TokenKind::Arrow) {
            self.parse_type_anno()
        } else {
            None
        };

        eprintln!("[DEBUG] parse_fn_stmt: return_type = {:?}", return_type);

        // Expect equals sign
        if !self.skip(&TokenKind::Eq) {
            self.error(super::ParseError::UnexpectedToken(
                self.current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
            ));
            eprintln!("[DEBUG] parse_fn_stmt: failed to expect Eq");
            return None;
        }

        // Parse implementation: (params) => body
        eprintln!(
            "[DEBUG] parse_fn_stmt: expecting LParen for params, current: {:?}",
            self.current().map(|t| &t.kind)
        );
        if !self.expect(&TokenKind::LParen) {
            eprintln!("[DEBUG] parse_fn_stmt: failed to expect LParen for params");
            return None;
        }
        let params = self.parse_fn_params()?;
        if !self.expect(&TokenKind::RParen) {
            eprintln!("[DEBUG] parse_fn_stmt: failed to expect RParen for params");
            return None;
        }

        // Expect fat arrow
        if !self.expect(&TokenKind::FatArrow) {
            eprintln!("[DEBUG] parse_fn_stmt: failed to expect FatArrow");
            return None;
        }

        // Parse body (expression or block)
        let (stmts, expr) = if self.at(&TokenKind::LBrace) {
            // Block body
            if !self.expect(&TokenKind::LBrace) {
                return None;
            }
            let body = self.parse_block_body()?;
            if !self.expect(&TokenKind::RBrace) {
                return None;
            }
            body
        } else {
            // Single expression body
            let expr = self.parse_expression(BP_LOWEST)?;
            (Vec::new(), Some(Box::new(expr)))
        };

        eprintln!("[DEBUG] parse_fn_stmt: success!");

        // Validate function definition rules
        // Note: Parser is lenient. Type checking will handle type inference.
        // We allow all syntactically valid function definitions.
        if param_types.is_empty() && return_type.is_none() {
            // No-signature form: name() = (params) => body
            // This is allowed - type checker will infer types
            // Previously required no parameters, but now we allow parameters
            // and let the type checker handle inference
        } else {
            // Standard form: name(Types) -> Ret = (params) => body
            // Note: Parser is lenient. Type checking will handle type inference.
            // We allow parameter types without return type (type checker will infer).

            // Normalize parameter types: allow a single tuple type to represent multiple params
            let mut normalized_param_types = param_types.clone();
            if normalized_param_types.len() == 1 {
                if let Type::Tuple(inner) = &normalized_param_types[0] {
                    normalized_param_types = inner.clone();
                }
            }

            // Parameter count must match type count
            if normalized_param_types.len() != params.len() {
                self.error(super::ParseError::Generic(format!(
                    "Parameter count mismatch: expected {}, got {}",
                    normalized_param_types.len(),
                    params.len()
                )));
                return None;
            }
        }

        // Construct function type if types were provided
        let type_annotation = if !param_types.is_empty() || return_type.is_some() {
            // If the original param_types was a single Tuple wrapping multiple types,
            // unwrap it so the function type stores each parameter type separately.
            let mut fn_params = param_types;
            if fn_params.len() == 1 {
                if let Type::Tuple(inner) = &fn_params[0] {
                    fn_params = inner.clone();
                }
            }

            Some(Type::Fn {
                params: fn_params,
                return_type: Box::new(return_type.clone().unwrap_or(Type::Name("_".to_string()))),
            })
        } else {
            None
        };

        Some(Stmt {
            kind: StmtKind::Fn {
                name,
                type_annotation,
                params,
                body: (stmts, expr),
            },
            span,
        })
    }

    /// Parse function parameters
    fn parse_fn_params(&mut self) -> Option<Vec<Param>> {
        let mut params = Vec::new();

        while !self.at(&TokenKind::RParen) && !self.at_end() {
            if !params.is_empty() && !self.expect(&TokenKind::Comma) {
                return None;
            }

            // Check for trailing comma
            if self.at(&TokenKind::RParen) {
                break;
            }

            let param_span = self.span();

            // Handle '...' for variadic parameters
            let _is_variadic = self.skip(&TokenKind::DotDotDot);

            // Parse parameter name
            let name = match self.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => n.clone(),
                _ => {
                    break;
                },
            };
            self.bump();

            // Parse parameter type
            let ty = if self.skip(&TokenKind::Colon) {
                self.parse_type_anno()
            } else {
                None
            };

            params.push(Param {
                name,
                ty,
                span: param_span,
            });
        }

        Some(params)
    }

    /// Parse return statement
    fn parse_return_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        self.bump(); // consume 'return'

        let value =
            if self.at(&TokenKind::Semicolon) || self.at(&TokenKind::RBrace) || self.at_end() {
                None
            } else {
                Some(Box::new(self.parse_expression(BP_LOWEST)?))
            };

        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::Expr(Box::new(Expr::Return(value, span))),
            span,
        })
    }

    /// Parse break statement
    fn parse_break_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        self.bump(); // consume 'break'

        let label = if self.at(&TokenKind::ColonColon) {
            self.parse_loop_label()
        } else {
            None
        };

        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::Expr(Box::new(Expr::Break(label, span))),
            span,
        })
    }

    /// Parse continue statement
    fn parse_continue_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        self.bump(); // consume 'continue'

        let label = if self.at(&TokenKind::ColonColon) {
            self.parse_loop_label()
        } else {
            None
        };

        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::Expr(Box::new(Expr::Continue(label, span))),
            span,
        })
    }

    /// Parse for loop: `for item in iterable { body }`
    fn parse_for_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        self.bump(); // consume 'for'

        // Parse loop variable
        let var = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                ));
                return None;
            },
        };
        self.bump();

        // Expect 'in' keyword
        if !self.expect(&TokenKind::KwIn) {
            return None;
        }

        // Parse iterable expression
        let iterable = Box::new(self.parse_expression(BP_LOWEST)?);

        // Parse body
        let body = if self.at(&TokenKind::LBrace) {
            self.parse_block_expression()
        } else {
            // Single expression as body - use a default span since Expr doesn't expose span directly
            let expr = self.parse_expression(BP_LOWEST)?;
            let span = self.span();
            Some(Block {
                stmts: Vec::new(),
                expr: Some(Box::new(expr)),
                span,
            })
        };

        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::For {
                var,
                iterable,
                body: Box::new(body?),
                label: None,
            },
            span,
        })
    }

    /// Parse block as statement
    fn parse_block_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        let block = self.parse_block_expression()?;
        Some(Stmt {
            kind: StmtKind::Expr(Box::new(Expr::Block(block))),
            span,
        })
    }

    /// Parse expression statement
    fn parse_expr_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        let expr = self.parse_expression(BP_LOWEST)?;

        // Handle statement-terminating semicolon
        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::Expr(Box::new(expr)),
            span,
        })
    }
}
