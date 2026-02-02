//! Pratt parser implementation
//! Handles expression parsing with binding power

pub mod led;
pub mod nud;
pub mod precedence;

pub use nud::*;
pub use led::*;
pub use precedence::*;

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::ParserState;

/// Public entry point for expression parsing
pub fn parse_expression_impl(
    state: &mut ParserState<'_>,
    min_bp: u8,
) -> Option<Expr> {
    state.parse_expression_internal(min_bp)
}

impl ParserState<'_> {
    /// Internal expression parsing method
    pub fn parse_expression_internal(
        &mut self,
        min_bp: u8,
    ) -> Option<Expr> {
        let left = self.parse_prefix()?;

        let mut left = left;

        while let Some(_token) = self.current().cloned() {
            // Handle function call: expr(args)
            if matches!(_token.kind, TokenKind::LParen) {
                if BP_CALL < min_bp {
                    break;
                }
                // Parse function call
                self.bump(); // consume '('
                let args = self.parse_call_args()?;
                if !self.skip(&TokenKind::RParen) {
                    return None;
                }
                left = Expr::Call {
                    func: Box::new(left),
                    args,
                    span: _token.span,
                };
                continue;
            }

            // Handle lambda expressions: expr => body
            if matches!(_token.kind, TokenKind::FatArrow) {
                // Lambda has very low precedence (11), right-associative
                if 11 < min_bp {
                    break;
                }
                let span = _token.span;
                self.bump(); // consume '=>'

                // Convert the left-hand side to lambda parameters
                let params = self.expr_to_lambda_params(&left)?;

                // Parse the body - can be a block or an expression
                let body = if self.at(&TokenKind::LBrace) {
                    self.parse_lambda_body_block()?
                } else if self.at_end() {
                    // Missing lambda body
                    self.error(crate::frontend::core::parser::ParseError::Message(
                        "Expected expression after '=>' in lambda".to_string(),
                    ));
                    return None;
                } else {
                    // Single expression body
                    match self.parse_expression(BP_LOWEST) {
                        Some(expr) => Block {
                            stmts: Vec::new(),
                            expr: Some(Box::new(expr)),
                            span: self.span(),
                        },
                        None => {
                            // Missing or invalid lambda body expression
                            self.error(crate::frontend::core::parser::ParseError::Message(
                                "Expected expression after '=>' in lambda".to_string(),
                            ));
                            return None;
                        }
                    }
                };

                left = Expr::Lambda {
                    params,
                    body: Box::new(body),
                    span,
                };
                continue;
            }

            let (bp, _) = self.get_binding_power(&_token.kind);
            if bp < min_bp {
                break;
            }

            // Don't try to parse operators for tokens that aren't operators
            if !matches!(
                _token.kind,
                TokenKind::Plus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Percent
                    | TokenKind::EqEq
                    | TokenKind::Neq
                    | TokenKind::Lt
                    | TokenKind::Gt
                    | TokenKind::Le
                    | TokenKind::Ge
                    | TokenKind::And
                    | TokenKind::Or
                    | TokenKind::Eq
                    | TokenKind::DotDot
            ) {
                break;
            }

            self.bump();

            let right = self.parse_expression(bp + 1)?;

            left = Expr::BinOp {
                op: self.token_kind_to_binop(&_token.kind)?,
                left: Box::new(left),
                right: Box::new(right),
                span: _token.span,
            };
        }

        Some(left)
    }

    fn get_binding_power(
        &self,
        kind: &TokenKind,
    ) -> (u8, u8) {
        match kind {
            TokenKind::Eq => (BP_ASSIGN, BP_ASSIGN),
            TokenKind::DotDot => (BP_RANGE, BP_RANGE),
            TokenKind::Or => (BP_LOGICAL_OR, BP_LOGICAL_OR),
            TokenKind::And => (BP_LOGICAL_AND, BP_LOGICAL_AND),
            TokenKind::EqEq | TokenKind::Neq => (BP_EQUALITY, BP_EQUALITY),
            TokenKind::Lt | TokenKind::Gt | TokenKind::Le | TokenKind::Ge => {
                (BP_COMPARISON, BP_COMPARISON)
            }
            TokenKind::Plus | TokenKind::Minus => (BP_TERM, BP_TERM),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => (BP_FACTOR, BP_FACTOR),
            _ => (0, 0),
        }
    }

    /// Convert token kind to binary operator
    pub fn token_kind_to_binop(
        &self,
        kind: &TokenKind,
    ) -> Option<BinOp> {
        match kind {
            TokenKind::Eq => Some(BinOp::Assign),
            TokenKind::Plus => Some(BinOp::Add),
            TokenKind::Minus => Some(BinOp::Sub),
            TokenKind::Star => Some(BinOp::Mul),
            TokenKind::Slash => Some(BinOp::Div),
            TokenKind::Percent => Some(BinOp::Mod),
            TokenKind::EqEq => Some(BinOp::Eq),
            TokenKind::Neq => Some(BinOp::Neq),
            TokenKind::Lt => Some(BinOp::Lt),
            TokenKind::Gt => Some(BinOp::Gt),
            TokenKind::Le => Some(BinOp::Le),
            TokenKind::Ge => Some(BinOp::Ge),
            TokenKind::And => Some(BinOp::And),
            TokenKind::Or => Some(BinOp::Or),
            TokenKind::DotDot => Some(BinOp::Range),
            _ => None,
        }
    }

    fn parse_call_args(&mut self) -> Option<Vec<Expr>> {
        let mut args = Vec::new();

        // Empty argument list
        if self.at(&TokenKind::RParen) {
            return Some(args);
        }

        loop {
            let arg = self.parse_expression(BP_LOWEST)?;
            args.push(arg);

            if !self.skip(&TokenKind::Comma) {
                break;
            }

            // Handle trailing comma
            if self.at(&TokenKind::RParen) {
                break;
            }
        }

        Some(args)
    }

    /// Convert an expression to lambda parameters
    fn expr_to_lambda_params(
        &self,
        expr: &Expr,
    ) -> Option<Vec<Param>> {
        match expr {
            // Single identifier: x => expr
            Expr::Var(name, span) => Some(vec![Param {
                name: name.clone(),
                ty: None,
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
    fn parse_lambda_body_block(&mut self) -> Option<Block> {
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
