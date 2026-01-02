//! Infix expression parsing (led - left denotation)

use super::state::*;
use super::ast::*;
use super::super::lexer::tokens::*;

/// Extension trait for infix parsing
pub trait InfixParser {
    /// Parse infix expression with given left binding power
    fn parse_infix(&mut self, lhs: Expr, bp: u8) -> Option<Expr>;
}

impl<'a> ParserState<'a> {
    /// Get infix binding power and parser for current token
    #[inline]
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
            _ => None,
        }
    }

    /// Parse assignment expression
    fn parse_assign(&mut self, lhs: Expr, _left_bp: u8) -> Option<Expr> {
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
    fn parse_binary(&mut self, lhs: Expr, left_bp: u8) -> Option<Expr> {
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
            _ => {
                self.error(super::ParseError::InvalidExpression);
                return None;
            }
        };
        self.bump();

        // Use left_bp as right_bp so that operators of the same precedence can chain
        // e.g., a > b > c should parse as (a > b) > c, not a > (b > c)
        let rhs = self.parse_expression(left_bp)?;

        Some(Expr::BinOp {
            op,
            left: Box::new(lhs),
            right: Box::new(rhs),
            span,
        })
    }

    /// Parse function call expression
    fn parse_call(&mut self, lhs: Expr, _left_bp: u8) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume '('

        let mut args = Vec::new();

        // Empty call: foo()
        if self.at(&TokenKind::RParen) {
            self.bump();
            return Some(Expr::Call {
                func: Box::new(lhs),
                args,
                span: start_span,
            });
        }

        // Parse arguments
        while !self.at(&TokenKind::RParen) && !self.at_end() {
            if !args.is_empty() {
                if !self.expect(&TokenKind::Comma) {
                    return None;
                }
            }

            // Handle named arguments: foo(x: 1, y: 2)
            if let Some(TokenKind::Identifier(_)) = self.current().map(|t| &t.kind) {
                let name = match self.current().map(|t| &t.kind) {
                    Some(TokenKind::Identifier(n)) => n.clone(),
                    _ => {
                        args.push(self.parse_expression(BP_LOWEST)?);
                        continue;
                    }
                };

                // Peek ahead to see if next token is '=' (named arg)
                if let Some(next) = self.peek() {
                    if matches!(next.kind, TokenKind::Eq) {
                        // Named argument
                        self.bump(); // consume identifier
                        self.bump(); // consume '='

                        let value = self.parse_expression(BP_LOWEST)?;
                        args.push(Expr::BinOp {
                            op: BinOp::Assign,
                            left: Box::new(Expr::Var(name, self.span())),
                            right: Box::new(value),
                            span: self.span(),
                        });
                        continue;
                    }
                }
            }

            args.push(self.parse_expression(BP_LOWEST)?);
        }

        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        Some(Expr::Call {
            func: Box::new(lhs),
            args,
            span: start_span,
        })
    }

    /// Parse field access expression
    fn parse_field(&mut self, lhs: Expr, _left_bp: u8) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume '.'

        let field = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                self.error(super::ParseError::UnexpectedToken(
                    self.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
                ));
                return None;
            }
        };
        self.bump();

        // Check for optional semicolon after field (for statement-like syntax)
        self.skip(&TokenKind::Semicolon);

        // Handle optional call after field access
        if self.at(&TokenKind::LParen) {
            let call_span = self.span();
            self.bump(); // consume '('

            let mut args = Vec::new();

            if !self.at(&TokenKind::RParen) {
                while !self.at(&TokenKind::RParen) && !self.at_end() {
                    if !args.is_empty() {
                        if !self.expect(&TokenKind::Comma) {
                            return None;
                        }
                    }
                    args.push(self.parse_expression(BP_LOWEST)?);
                }
            }

            if !self.expect(&TokenKind::RParen) {
                return None;
            }

            // Chain field access and call
            return Some(Expr::Call {
                func: Box::new(Expr::FieldAccess {
                    expr: Box::new(lhs),
                    field: field.clone(),
                    span: start_span,
                }),
                args,
                span: call_span,
            });
        }

        Some(Expr::FieldAccess {
            expr: Box::new(lhs),
            field,
            span: start_span,
        })
    }

    /// Parse index expression
    fn parse_index(&mut self, lhs: Expr, _left_bp: u8) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume '['

        let index = self.parse_expression(BP_LOWEST)?;

        if !self.expect(&TokenKind::RBracket) {
            return None;
        }

        Some(Expr::Index {
            expr: Box::new(lhs),
            index: Box::new(index),
            span: start_span,
        })
    }

    /// Parse type cast expression
    fn parse_cast(&mut self, lhs: Expr, _left_bp: u8) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume 'as'

        let target_type = self.parse_type_anno()?;

        Some(Expr::Cast {
            expr: Box::new(lhs),
            target_type,
            span: start_span,
        })
    }
}
