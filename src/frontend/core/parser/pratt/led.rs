//! Infix expression parsing (led - left denotation)
//!
//! This module implements infix parsing for the Pratt parser with RFC-010/011 support.

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::ParserState;
use crate::frontend::core::parser::pratt::precedence::*;
use crate::frontend::core::parser::ParseError;
use crate::frontend::core::parser::statements::TypeStatementParser;

/// Extension trait for infix parsing
pub trait InfixParser {
    /// Parse infix expression with given left binding power
    fn parse_infix(
        &mut self,
        lhs: Expr,
        bp: u8,
    ) -> Option<Expr>;
}

impl<'a> InfixParser for ParserState<'a> {
    fn parse_infix(
        &mut self,
        lhs: Expr,
        bp: u8,
    ) -> Option<Expr> {
        self.infix_info()
            .and_then(|(_bp_left, _bp_right, parser_fn)| {
                // For now, just call the parser function directly
                parser_fn(self, lhs, bp)
            })
    }
}

impl<'a> ParserState<'a> {
    /// Get infix binding power and parser for current token
    #[inline]
    #[allow(clippy::type_complexity)]
    pub(crate) fn infix_info(&self) -> Option<(u8, u8, fn(&mut Self, Expr, u8) -> Option<Expr>)> {
        match self.current().map(|t| &t.kind) {
            // Assignment
            Some(TokenKind::Eq) => Some((BP_ASSIGN, BP_ASSIGN + 1, Self::parse_assign)),
            // Range
            Some(TokenKind::DotDot) => Some((BP_RANGE, BP_RANGE + 1, Self::parse_binary)),
            // Logical OR
            Some(TokenKind::Or) => Some((BP_OR, BP_OR + 1, Self::parse_binary)),
            // Logical AND
            Some(TokenKind::And) => Some((BP_AND, BP_AND + 1, Self::parse_binary)),
            // Equality
            Some(TokenKind::EqEq) | Some(TokenKind::Neq) => {
                Some((BP_EQ, BP_EQ + 1, Self::parse_binary))
            }
            // Comparison
            Some(TokenKind::Lt | TokenKind::Le | TokenKind::Gt | TokenKind::Ge) => {
                Some((BP_CMP, BP_CMP + 1, Self::parse_binary))
            }
            // Addition/Subtraction
            Some(TokenKind::Plus | TokenKind::Minus) => {
                Some((BP_ADD, BP_ADD + 1, Self::parse_binary))
            }
            // Multiplication/Division/Modulo
            Some(TokenKind::Star | TokenKind::Slash | TokenKind::Percent) => {
                Some((BP_MUL, BP_MUL + 1, Self::parse_binary))
            }
            // Function call
            Some(TokenKind::LParen) => Some((BP_CALL, BP_CALL + 1, Self::parse_call)),
            // Field access
            Some(TokenKind::Dot) => Some((BP_CALL, BP_CALL + 1, Self::parse_field)),
            // Indexing
            Some(TokenKind::LBracket) => Some((BP_CALL, BP_CALL + 1, Self::parse_index)),
            // Type cast
            Some(TokenKind::KwAs) => Some((BP_ADD, BP_ADD + 1, Self::parse_cast)),
            // Try operator (error propagation)
            Some(TokenKind::Question) => Some((BP_CALL, BP_CALL + 1, Self::parse_try)),
            // Lambda (single parameter)
            Some(TokenKind::FatArrow) => Some((11, 1, Self::parse_lambda_infix)),
            _ => None,
        }
    }

    /// Parse assignment expression
    fn parse_assign(
        &mut self,
        lhs: Expr,
        _left_bp: u8,
    ) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '='

        let rhs = self.parse_expression(BP_ASSIGN)?;

        Some(Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(lhs),
            right: Box::new(rhs),
            span,
        })
    }

    /// Parse binary operator expression
    fn parse_binary(
        &mut self,
        lhs: Expr,
        _left_bp: u8,
    ) -> Option<Expr> {
        let span = self.span();
        let op = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Plus) => BinOp::Add,
            Some(TokenKind::Minus) => BinOp::Sub,
            Some(TokenKind::Star) => BinOp::Mul,
            Some(TokenKind::Slash) => BinOp::Div,
            Some(TokenKind::Percent) => BinOp::Mod,
            Some(TokenKind::EqEq) => BinOp::Eq,
            Some(TokenKind::Neq) => BinOp::Neq,
            Some(TokenKind::Lt) => BinOp::Lt,
            Some(TokenKind::Le) => BinOp::Le,
            Some(TokenKind::Gt) => BinOp::Gt,
            Some(TokenKind::Ge) => BinOp::Ge,
            Some(TokenKind::And) => BinOp::And,
            Some(TokenKind::Or) => BinOp::Or,
            Some(TokenKind::DotDot) => BinOp::Range,
            _ => return None,
        };
        self.bump();

        let rhs = self.parse_expression(_left_bp)?;

        Some(Expr::BinOp {
            op,
            left: Box::new(lhs),
            right: Box::new(rhs),
            span,
        })
    }

    /// Parse function call expression
    fn parse_call(
        &mut self,
        lhs: Expr,
        _left_bp: u8,
    ) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '('

        let mut args = Vec::new();

        // Empty argument list
        if self.at(&TokenKind::RParen) {
            self.bump(); // consume ')'
            return Some(Expr::Call {
                func: Box::new(lhs),
                args,
                span,
            });
        }

        loop {
            let arg = self.parse_expression(BP_LOWEST)?;
            args.push(arg);

            if self.skip(&TokenKind::Comma) {
                // Handle trailing comma
                if self.at(&TokenKind::RParen) {
                    self.bump(); // consume ')'
                    break;
                }
            } else {
                self.expect(&TokenKind::RParen);
                break;
            }
        }

        Some(Expr::Call {
            func: Box::new(lhs),
            args,
            span,
        })
    }

    /// Parse field access expression
    fn parse_field(
        &mut self,
        lhs: Expr,
        _left_bp: u8,
    ) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '.'

        let token = self.current().cloned()?;
        if let TokenKind::Identifier(name) = token.kind {
            self.bump();
            Some(Expr::FieldAccess {
                expr: Box::new(lhs),
                field: name,
                span,
            })
        } else {
            self.error(ParseError::UnexpectedToken {
                found: token.kind,
                span: self.span(),
            });
            None
        }
    }

    /// Parse indexing expression
    fn parse_index(
        &mut self,
        lhs: Expr,
        _left_bp: u8,
    ) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '['

        let index = self.parse_expression(BP_LOWEST)?;

        self.expect(&TokenKind::RBracket);

        Some(Expr::Index {
            expr: Box::new(lhs),
            index: Box::new(index),
            span,
        })
    }

    /// Parse type cast expression
    fn parse_cast(
        &mut self,
        lhs: Expr,
        _left_bp: u8,
    ) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'as'

        let ty = self.parse_type_annotation()?;

        Some(Expr::Cast {
            expr: Box::new(lhs),
            target_type: ty,
            span,
        })
    }

    /// Parse try operator (error propagation)
    fn parse_try(
        &mut self,
        lhs: Expr,
        _left_bp: u8,
    ) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '?'

        Some(Expr::Try {
            expr: Box::new(lhs),
            span,
        })
    }

    /// Parse lambda expression: `x => expr`, `(x) => expr`, or `(a, b) => expr`
    fn parse_lambda_infix(
        &mut self,
        lhs: Expr,
        _left_bp: u8,
    ) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '=>'

        // Convert the left-hand side to lambda parameters
        let params = self.expr_to_params(&lhs)?;

        // Parse the body - can be a block or an expression
        let body = if self.at(&TokenKind::LBrace) {
            self.parse_lambda_block()?
        } else {
            // Single expression body
            let expr = self.parse_expression(BP_LOWEST)?;
            Block {
                stmts: Vec::new(),
                expr: Some(Box::new(expr)),
                span: self.span(),
            }
        };

        Some(Expr::Lambda {
            params,
            body: Box::new(body),
            span,
        })
    }

    /// Convert an expression to lambda parameters
    fn expr_to_params(
        &self,
        expr: &Expr,
    ) -> Option<Vec<Param>> {
        match expr {
            // Single identifier: x => expr
            Expr::Var(name, span) => Some(vec![Param {
                name: name.clone(),
                ty: None,
                is_mut: false,
                span: *span,
            }]),
            // Empty tuple: () => expr
            Expr::Tuple(elements, _) if elements.is_empty() => Some(Vec::new()),
            // Tuple of identifiers: (a, b) => expr
            Expr::Tuple(elements, _) => {
                let mut params = Vec::new();
                for elem in elements {
                    if let Expr::Var(name, span) = elem {
                        params.push(Param {
                            name: name.clone(),
                            ty: None,
                            is_mut: false,
                            span: *span,
                        });
                    } else {
                        // Non-identifier in parameter list
                        return None;
                    }
                }
                Some(params)
            }
            // Typed parameter list from parse_group_or_tuple: (a: Int, b: Int) => expr
            Expr::Lambda { params, .. } => Some(params.clone()),
            _ => None,
        }
    }

    /// Parse a block for lambda body
    fn parse_lambda_block(&mut self) -> Option<Block> {
        let span = self.span();
        self.bump(); // consume '{'

        let mut stmts = Vec::new();

        while !self.at(&TokenKind::RBrace) && !self.at_end() {
            if self.at(&TokenKind::Semicolon) {
                self.bump();
                continue;
            }

            if let Some(stmt) = self.parse_statement() {
                stmts.push(stmt);
            } else {
                self.bump();
            }
        }

        self.expect(&TokenKind::RBrace);

        // Check if block ends with an expression (without semicolon)
        let expr = if stmts
            .last()
            .is_some_and(|s| matches!(s.kind, StmtKind::Expr(_)))
        {
            stmts.pop().and_then(|stmt| {
                if let StmtKind::Expr(expr) = stmt.kind {
                    Some(expr)
                } else {
                    None
                }
            })
        } else {
            None
        };

        Some(Block { stmts, expr, span })
    }
}
