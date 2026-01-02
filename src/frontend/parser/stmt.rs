//! Statement parsing

use super::state::*;
use super::ast::*;
use super::super::lexer::tokens::*;
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
    fn parse_var_stmt(&mut self, span: Span) -> Option<Stmt> {
        // Check for mutability
        let is_mut = if self.skip(&TokenKind::KwMut) {
            true
        } else {
            false
        };

        // Parse variable name (identifier)
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
                ));
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
    fn parse_type_stmt(&mut self, span: Span) -> Option<Stmt> {
        self.bump(); // consume 'type'

        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
                ));
                return None;
            }
        };
        self.bump();

        if !self.expect(&TokenKind::Eq) {
            return None;
        }

        let definition = self.parse_type_anno()?;

        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::TypeDef { name, definition },
            span,
        })
    }

    /// Parse module definition: `mod Name { ... }`
    fn parse_module_stmt(&mut self, span: Span) -> Option<Stmt> {
        self.bump(); // consume 'mod'

        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
                ));
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
    fn parse_use_stmt(&mut self, span: Span) -> Option<Stmt> {
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
            if !self.skip(&TokenKind::ColonColon) {
                break;
            }
        }

        if parts.is_empty() {
            self.error(super::ParseError::UnexpectedToken(
                self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
            ));
            None
        } else {
            Some(parts.join("::"))
        }
    }

    /// Parse statement starting with identifier: function definition or expression
    fn parse_identifier_stmt(&mut self, span: Span) -> Option<Stmt> {
        // Look ahead to determine what kind of statement this is
        // 1. Function definition: name(types) -> type = (params) => body
        // 2. Variable declaration: name[: type] [= expr]
        // 3. Expression statement: name expr...

        // Use peek_nth to look ahead without consuming tokens
        let next = self.peek();

        // Check for potential function definition: identifier followed by LParen
        if matches!(next.map(|t| &t.kind), Some(TokenKind::LParen)) {
            // Look ahead to find matching RParen, then check what's after it
            // For function definition: name(types) -> type = ...
            // For function call: name(args) ...

            // Count depth to find matching RParen
            let mut depth = 1;
            let mut pos = 2; // Start after identifier and LParen

            while depth > 0 {
                match self.peek_nth(pos).map(|t| &t.kind) {
                    Some(TokenKind::LParen) => {
                        depth += 1;
                        pos += 1;
                    }
                    Some(TokenKind::RParen) => {
                        depth -= 1;
                        if depth > 0 {
                            pos += 1;
                        }
                    }
                    Some(TokenKind::Comma) => {
                        pos += 1;
                    }
                    Some(_) => {
                        pos += 1;
                    }
                    None => {
                        // Reached end of tokens
                        break;
                    }
                }
            }

            // Check what's after the RParen
            let after_rparen = self.peek_nth(pos);

            // If after RParen comes -> or =, this is a function definition
            if matches!(after_rparen.map(|t| &t.kind), Some(TokenKind::Arrow) | Some(TokenKind::Eq)) {
                // This is a function definition - parse as expression
                return self.parse_expr_stmt(span);
            }

            // Otherwise, this is a function call or other expression
            // Just parse as expression (the Pratt parser will handle it)
            return self.parse_expr_stmt(span);
        }

        // Consume the identifier first
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
                ));
                return None;
            }
        };
        self.bump();

        // After identifier, check for : or =
        if self.at(&TokenKind::Colon) || self.at(&TokenKind::Eq) {
            // This is a variable declaration (without mut)

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
        let expr = self.parse_expression(BP_LOWEST)?;

        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::Expr(Box::new(expr)),
            span,
        })
    }

    /// Parse function definition: `name(types) -> type = (params) => body`
    /// Example: `add(Int, Int) -> Int = (a, b) => a + b`
    fn parse_fn_stmt(&mut self, span: Span) -> Option<Stmt> {
        // Parse function name
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
                ));
                return None;
            }
        };
        self.bump();

        // Parse parameter types in parentheses: (Int, String)
        if !self.expect(&TokenKind::LParen) {
            return None;
        }
        let _param_types = self.parse_type_list()?;
        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        // Parse return type: -> ReturnType
        let return_type = if self.skip(&TokenKind::Arrow) {
            self.parse_type_anno()
        } else {
            None
        };

        // Expect equals sign
        if !self.skip(&TokenKind::Eq) {
            self.error(super::ParseError::UnexpectedToken(
                self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
            ));
            return None;
        }

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
            self.parse_block_body()?
        } else {
            // Single expression body
            let expr = self.parse_expression(BP_LOWEST)?;
            (Vec::new(), Some(Box::new(expr)))
        };

        Some(Stmt {
            kind: StmtKind::Expr(Box::new(Expr::FnDef {
                name,
                params,
                return_type,
                body: Box::new(Block {
                    stmts,
                    expr,
                    span: self.span(),
                }),
                is_async: false,
                span,
            })),
            span,
        })
    }

    /// Parse function parameters
    fn parse_fn_params(&mut self) -> Option<Vec<Param>> {
        let mut params = Vec::new();

        while !self.at(&TokenKind::RParen) && !self.at_end() {
            if !params.is_empty() {
                if !self.expect(&TokenKind::Comma) {
                    return None;
                }
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
    fn parse_return_stmt(&mut self, span: Span) -> Option<Stmt> {
        self.bump(); // consume 'return'

        let value = if self.at(&TokenKind::Semicolon)
            || self.at(&TokenKind::RBrace)
            || self.at_end()
        {
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
    fn parse_break_stmt(&mut self, span: Span) -> Option<Stmt> {
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
    fn parse_continue_stmt(&mut self, span: Span) -> Option<Stmt> {
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
    fn parse_for_stmt(&mut self, span: Span) -> Option<Stmt> {
        self.bump(); // consume 'for'

        // Parse loop variable
        let var = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
                ));
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
    fn parse_block_stmt(&mut self, span: Span) -> Option<Stmt> {
        let block = self.parse_block_expression()?;
        Some(Stmt {
            kind: StmtKind::Expr(Box::new(Expr::Block(block))),
            span,
        })
    }

    /// Parse expression statement
    fn parse_expr_stmt(&mut self, span: Span) -> Option<Stmt> {
        let expr = self.parse_expression(BP_LOWEST)?;

        // Handle statement-terminating semicolon
        self.skip(&TokenKind::Semicolon);

        Some(Stmt {
            kind: StmtKind::Expr(Box::new(expr)),
            span,
        })
    }
}
