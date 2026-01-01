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
            // module definition
            Some(TokenKind::KwMod) => self.parse_module_stmt(start_span),
            // use import
            Some(TokenKind::KwUse) => self.parse_use_stmt(start_span),
            // function definition
            Some(TokenKind::KwFn) => self.parse_fn_stmt(start_span),
            // return statement
            Some(TokenKind::KwReturn) => self.parse_return_stmt(start_span),
            // break statement
            Some(TokenKind::KwBreak) => self.parse_break_stmt(start_span),
            // continue statement
            Some(TokenKind::KwContinue) => self.parse_continue_stmt(start_span),
            // block as statement
            Some(TokenKind::LBrace) => self.parse_block_stmt(start_span),
            // variable declaration: [mut] identifier [: type] [= expr]
            Some(TokenKind::KwMut) => self.parse_var_stmt(start_span),
            Some(TokenKind::Identifier(_)) => {
                // Check if this might be a variable declaration: identifier followed by : or =
                self.parse_var_or_expr_stmt(start_span)
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

    /// Try to parse as variable declaration, fall back to expression
    fn parse_var_or_expr_stmt(&mut self, span: Span) -> Option<Stmt> {
        let start_span = self.span();
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => return self.parse_expr_stmt(start_span),
        };

        // Peek ahead to check if this looks like a variable declaration
        // Pattern: identifier followed by : or =
        // We need to check the next non-whitespace token
        self.bump(); // consume identifier

        // Check next token for : or =
        let next_is_decl = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Colon) | Some(TokenKind::Eq) => true,
            _ => false,
        };

        if next_is_decl {
            // This is a variable declaration without 'mut'
            // We're at : or =, need to backtrack and call parse_var_stmt
            // But since we've already consumed the identifier, we need to handle this differently

            // For simplicity, reconstruct the logic here
            let type_annotation = if self.skip(&TokenKind::Colon) {
                self.parse_type_anno()
            } else {
                None
            };

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

        // Not a variable declaration, parse as expression
        // Put the identifier back and parse as expression
        self.parse_expr_stmt(start_span)
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

    /// Parse function definition
    fn parse_fn_stmt(&mut self, span: Span) -> Option<Stmt> {
        let _is_async = self.skip(&TokenKind::KwAsync);
        if !self.expect(&TokenKind::KwFn) {
            return None;
        }

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

        // Parse function parameters
        if !self.expect(&TokenKind::LParen) {
            return None;
        }
        let params = self.parse_fn_params()?;
        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        // Parse return type
        let return_type = if self.skip(&TokenKind::Arrow) {
            self.parse_type_anno()
        } else {
            None
        };

        // Parse function body
        if !self.expect(&TokenKind::LBrace) {
            return None;
        }
        let (stmts, expr) = self.parse_block_body()?;
        if !self.expect(&TokenKind::RBrace) {
            return None;
        }

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
