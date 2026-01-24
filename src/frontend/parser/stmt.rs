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
                self.parse_identifier_stmt(start_span)
            }
            // expression statement
            Some(_) => self.parse_expr_stmt(start_span),
            None => None,
        }
    }

    /// Parse variable declaration: `[mut] name[: type] [= expr];`
    /// New syntax: `x: int = 42` or `mut y: int = 10`
    /// Also handles function definition: `name: (ParamTypes) -> ReturnType = (params) => body`
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
                let span = self.current().map(|t| t.span).unwrap_or_else(Span::dummy);
                self.error(super::ParseError::UnexpectedToken {
                    found: self
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                    span,
                });
                return None;
            }
        };
        self.bump();

        // Optional type annotation
        let type_annotation = if self.skip(&TokenKind::Colon) {
            self.parse_type_anno()
        } else {
            None
        };

        // Check for invalid syntax: name: Type (params) => body (missing =)
        // Only report error if there's no = sign (i.e., it's likely a function definition missing =)
        // If there IS an =, then ( might be part of a tuple value, not function params
        if type_annotation.is_some() && self.at(&TokenKind::LParen) && !self.at(&TokenKind::Eq) {
            // This looks like "name: Type (params) => body" which is missing the =
            // Report an error
            let span = self.current().map(|t| t.span).unwrap_or_else(Span::dummy);
            self.error(super::ParseError::UnexpectedToken {
                found: self
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span,
            });
            return None;
        }

        // Optional initializer
        if self.skip(&TokenKind::Eq) {
            // Check if this is a function definition: name: Type = (params) => body
            // The key difference: after =, if we see ( and then later =>, it's a function
            if self.at(&TokenKind::LParen) {
                // Save current position to potentially backtrack
                let saved_position = self.save_position();

                // Try to parse as function definition
                if let Some(fn_stmt) = self.parse_fn_stmt_with_name_and_type(
                    name.clone(),
                    type_annotation.clone(),
                    span,
                ) {
                    self.skip(&TokenKind::Semicolon);
                    return Some(fn_stmt);
                }

                // Function parsing failed, backtrack and clear errors
                // The ( is likely the start of a tuple value, not function params
                self.restore_position(saved_position);
                self.clear_errors();
            }

            // Regular variable initializer: parse as expression
            let initializer = Some(Box::new(self.parse_expression(BP_LOWEST)?));

            self.skip(&TokenKind::Semicolon);

            return Some(Stmt {
                kind: StmtKind::Var {
                    name,
                    type_annotation,
                    initializer,
                    is_mut,
                },
                span,
            });
        }

        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::Var {
                name,
                type_annotation,
                initializer: None,
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
                let span = self.current().map(|t| t.span).unwrap_or_else(Span::dummy);
                self.error(super::ParseError::UnexpectedToken {
                    found: self
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                    span,
                });
                return None;
            }
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
                        }
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
                        }
                        Type::Name(name) => {
                            variants.push(VariantDef {
                                name: name.clone(),
                                params: Vec::new(),
                                span: self.span(),
                            });
                        }
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
                let span = self.current().map(|t| t.span).unwrap_or_else(Span::dummy);
                self.error(super::ParseError::UnexpectedToken {
                    found: self
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                    span,
                });
                return None;
            }
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
            }
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
                let span = self.current().map(|t| t.span).unwrap_or_else(Span::dummy);
                self.error(super::ParseError::UnexpectedToken {
                    found: self
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                    span,
                });
                return None;
            }
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
                    }
                    Some(TokenKind::KwPub) => {
                        // Skip 'pub' in import items
                        self.bump();
                    }
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
                }
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
            let span = self.current().map(|t| t.span).unwrap_or_else(Span::dummy);
            self.error(super::ParseError::UnexpectedToken {
                found: self
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span,
            });
            None
        } else {
            Some(parts.join("."))
        }
    }

    /// Parse statement starting with identifier: function definition or expression or variable declaration
    /// 语法:
    /// - `name = (params) => body` - 函数定义，= 后是 (params) => body
    /// - `name = expr` - 变量声明（如果没有类型注解）
    /// - `name: type = expr` - 变量声明（带类型注解）
    /// - `name expr` - 表达式语句
    fn parse_identifier_stmt(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        let next = self.peek();

        // Check if identifier is followed by =
        if matches!(next.map(|t| &t.kind), Some(TokenKind::Eq)) {
            // 这可能是变量声明或函数定义
            // 保存当前位置以便回溯
            let saved_position = self.save_position();

            // 消费 identifier
            let name = match self.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => n.clone(),
                _ => {
                    let span = self.current().map(|t| t.span).unwrap_or_else(Span::dummy);
                    self.error(super::ParseError::UnexpectedToken {
                        found: self
                            .current()
                            .map(|t| t.kind.clone())
                            .unwrap_or(TokenKind::Eof),
                        span,
                    });
                    return None;
                }
            };
            self.bump(); // consume identifier

            // 检查 = 后是否紧跟着 (，尝试解析为函数定义
            if self.at(&TokenKind::Eq) {
                self.bump(); // consume =

                // 如果 = 后是 (，尝试解析为函数定义
                if self.at(&TokenKind::LParen) {
                    // 尝试解析为函数定义: name = (params) => body
                    if let Some(stmt) = self.parse_fn_stmt_with_name(name.clone(), span) {
                        self.skip(&TokenKind::Semicolon);
                        return Some(stmt);
                    }
                    // 函数定义解析失败，回溯
                    self.restore_position(saved_position);
                    self.clear_errors();
                } else if let Some(TokenKind::Identifier(_)) = self.current().map(|t| &t.kind) {
                    // = 后是 identifier，可能是简单的函数定义: name = param => body
                    // 保存位置以便回溯
                    let saved_position2 = self.save_position();

                    // 尝试解析为简单函数定义
                    if let Some(stmt) = self.parse_fn_stmt_with_name_simple(name.clone(), span) {
                        self.skip(&TokenKind::Semicolon);
                        return Some(stmt);
                    }

                    // 回溯，尝试作为变量声明处理
                    self.restore_position(saved_position2);
                    self.clear_errors();
                }
            }

            // 回溯并作为变量声明处理
            self.restore_position(saved_position);
            self.clear_errors();

            // 调用 parse_var_stmt 来处理变量声明
            return self.parse_var_stmt(span);
        }

        // Check for variable declaration: identifier followed by :
        if matches!(next.map(|t| &t.kind), Some(TokenKind::Colon)) {
            // 调用 parse_var_stmt，它会消费 identifier 和 :
            return self.parse_var_stmt(span);
        }

        // Check for function definition with type annotation: name(types) -> type = (params) => body
        if matches!(next.map(|t| &t.kind), Some(TokenKind::LParen)) {
            // 这可能是函数定义: name(types) -> type = (params) => body
            // 保存位置以便回溯
            let saved_position = self.save_position();

            // 尝试解析为函数定义
            if let Some(stmt) = self.parse_fn_stmt_with_type_anno(span) {
                return Some(stmt);
            }

            // 回溯，尝试作为表达式解析
            self.restore_position(saved_position);
            self.clear_errors();
        }

        // Otherwise, parse as expression
        self.parse_expr_stmt(span)
    }

    /// Parse function definition with type annotation: `name(types) -> type = (params) => body`
    fn parse_fn_stmt_with_type_anno(
        &mut self,
        span: Span,
    ) -> Option<Stmt> {
        // Parse function name
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => return None,
        };
        self.bump(); // consume function name

        // Parse type parameters: (Type1, Type2, ...)
        if !self.expect(&TokenKind::LParen) {
            return None;
        }
        let type_params = self.parse_type_param_list()?;
        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        // Parse return type: -> Type
        if !self.expect(&TokenKind::Arrow) {
            return None;
        }
        let return_type = self.parse_type_anno()?;

        // Parse function body: = (params) => body
        if !self.expect(&TokenKind::Eq) {
            return None;
        }

        // Parse params in parentheses
        if !self.expect(&TokenKind::LParen) {
            return None;
        }
        let params = self.parse_fn_params()?;
        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        // Expect fat arrow
        if !self.expect(&TokenKind::FatArrow) {
            return None;
        }

        // Parse body (expression or block)
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

        // Build function type annotation
        let fn_type = Type::Fn {
            params: type_params,
            return_type: Box::new(return_type),
        };

        Some(Stmt {
            kind: StmtKind::Fn {
                name,
                type_annotation: Some(fn_type),
                params,
                body: (stmts, expr),
            },
            span,
        })
    }

    /// Parse a list of types (for function type parameters)
    fn parse_type_param_list(&mut self) -> Option<Vec<Type>> {
        let mut types = Vec::new();

        while !self.at(&TokenKind::RParen) && !self.at_end() {
            if !types.is_empty() && !self.expect(&TokenKind::Comma) {
                return None;
            }

            if self.at(&TokenKind::RParen) {
                break;
            }

            let ty = self.parse_type_anno()?;
            types.push(ty);
        }

        Some(types)
    }

    /// Parse function definition with already parsed name (no parentheses around params)
    /// Handles: `name = param => body` (single param without parentheses)
    fn parse_fn_stmt_with_name_simple(
        &mut self,
        name: String,
        span: Span,
    ) -> Option<Stmt> {
        // Parse single parameter name
        let param_span = self.span();
        let param_name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                return None;
            }
        };
        self.bump(); // consume param name

        // Expect fat arrow
        if !self.expect(&TokenKind::FatArrow) {
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

        Some(Stmt {
            kind: StmtKind::Fn {
                name,
                type_annotation: None,
                params: vec![Param {
                    name: param_name,
                    ty: None,
                    span: param_span,
                }],
                body: (stmts, expr),
            },
            span,
        })
    }

    /// Parse function definition with already parsed name
    fn parse_fn_stmt_with_name(
        &mut self,
        name: String,
        span: Span,
    ) -> Option<Stmt> {
        // Parse implementation: (params) => body
        if !self.expect(&TokenKind::LParen) {
            return None;
        }
        let params = self.parse_fn_params()?;
        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        // Expect fat arrow
        if !self.expect(&TokenKind::FatArrow) {
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

        Some(Stmt {
            kind: StmtKind::Fn {
                name,
                type_annotation: None,
                params,
                body: (stmts, expr),
            },
            span,
        })
    }

    /// Parse function definition with already parsed name and type annotation
    fn parse_fn_stmt_with_name_and_type(
        &mut self,
        name: String,
        type_annotation: Option<Type>,
        span: Span,
    ) -> Option<Stmt> {
        // Parse implementation: (params) => body
        if !self.expect(&TokenKind::LParen) {
            return None;
        }
        let params = self.parse_fn_params()?;
        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        // Expect fat arrow
        if !self.expect(&TokenKind::FatArrow) {
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
                }
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
                let span = self.current().map(|t| t.span).unwrap_or_else(Span::dummy);
                self.error(super::ParseError::UnexpectedToken {
                    found: self
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                    span,
                });
                return None;
            }
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
