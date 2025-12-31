//! Prefix expression parsing (nud - null denotation)

use super::state::*;
use super::ast::*;
use super::super::lexer::tokens::*;

/// Extension trait for prefix parsing
pub trait PrefixParser {
    /// Parse prefix expression at current position
    fn parse_prefix(&mut self, bp: u8) -> Option<Expr>;
}

impl<'a> ParserState<'a> {
    /// Get prefix binding power and parser for current token
    #[inline]
    pub(crate) fn prefix_info(&self) -> Option<(u8, fn(&mut Self) -> Option<Expr>)> {
        match self.current().map(|t| &t.kind) {
            // Unary operators
            Some(TokenKind::Minus) | Some(TokenKind::Plus) | Some(TokenKind::Not) => {
                Some((BP_UNARY, Self::parse_unary))
            }
            // Literals
            Some(TokenKind::IntLiteral(_)) => Some((BP_HIGHEST, Self::parse_int_literal)),
            Some(TokenKind::FloatLiteral(_)) => Some((BP_HIGHEST, Self::parse_float_literal)),
            Some(TokenKind::StringLiteral(_)) => Some((BP_HIGHEST, Self::parse_string_literal)),
            Some(TokenKind::CharLiteral(_)) => Some((BP_HIGHEST, Self::parse_char_literal)),
            Some(TokenKind::BoolLiteral(_)) => Some((BP_HIGHEST, Self::parse_bool_literal)),
            // Identifier or path
            Some(TokenKind::Identifier(_)) => Some((BP_HIGHEST, Self::parse_identifier)),
            // Grouped expression or tuple
            Some(TokenKind::LParen) => Some((BP_HIGHEST, Self::parse_group_or_tuple)),
            // Block expression
            Some(TokenKind::LBrace) => Some((BP_HIGHEST, Self::parse_block)),
            // If expression
            Some(TokenKind::KwIf) => Some((BP_HIGHEST, Self::parse_if)),
            // Match expression
            Some(TokenKind::KwMatch) => Some((BP_HIGHEST, Self::parse_match)),
            // While expression
            Some(TokenKind::KwWhile) => Some((BP_HIGHEST, Self::parse_while)),
            // For expression
            Some(TokenKind::KwFor) => Some((BP_HIGHEST, Self::parse_for)),
            // Lambda expression
            Some(TokenKind::Pipe) => Some((BP_HIGHEST, Self::parse_lambda)),
            _ => None,
        }
    }

    /// Parse unary operator expression
    fn parse_unary(&mut self) -> Option<Expr> {
        let span = self.span();
        let op = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Minus) => UnOp::Neg,
            Some(TokenKind::Plus) => UnOp::Pos,
            Some(TokenKind::Not) => UnOp::Not,
            _ => return None,
        };
        self.bump();

        // Parse operand with higher binding power
        let operand = self.parse_expression(BP_UNARY + 1)?;

        Some(Expr::UnOp {
            op,
            expr: Box::new(operand),
            span,
        })
    }

    /// Parse integer literal
    fn parse_int_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        let value = match self.current().map(|t| &t.kind) {
            Some(TokenKind::IntLiteral(n)) => *n,
            _ => return None,
        };
        self.bump();
        Some(Expr::Lit(Literal::Int(value), span))
    }

    /// Parse float literal
    fn parse_float_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        let value = match self.current().map(|t| &t.kind) {
            Some(TokenKind::FloatLiteral(n)) => *n,
            _ => return None,
        };
        self.bump();
        Some(Expr::Lit(Literal::Float(value), span))
    }

    /// Parse string literal
    fn parse_string_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        let value = match self.current().map(|t| &t.kind) {
            Some(TokenKind::StringLiteral(s)) => s.clone(),
            _ => return None,
        };
        self.bump();
        Some(Expr::Lit(Literal::String(value), span))
    }

    /// Parse char literal
    fn parse_char_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        let value = match self.current().map(|t| &t.kind) {
            Some(TokenKind::CharLiteral(c)) => *c,
            _ => return None,
        };
        self.bump();
        Some(Expr::Lit(Literal::Char(value), span))
    }

    /// Parse bool literal
    fn parse_bool_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        let value = match self.current().map(|t| &t.kind) {
            Some(TokenKind::BoolLiteral(b)) => *b,
            _ => return None,
        };
        self.bump();
        Some(Expr::Lit(Literal::Bool(value), span))
    }

    /// Parse identifier (variable reference or path)
    fn parse_identifier(&mut self) -> Option<Expr> {
        let span = self.span();
        let name = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(name)) => name.clone(),
            _ => return None,
        };
        self.bump();
        Some(Expr::Var(name, span))
    }

    /// Parse grouped expression or tuple
    fn parse_group_or_tuple(&mut self) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume '('

        // Empty tuple: ()
        if self.skip(&TokenKind::RParen) {
            return Some(Expr::Tuple(vec![], start_span));
        }

        // Single element in parens: (expr)
        let first = self.parse_expression(BP_LOWEST)?;

        // Tuple with multiple elements: (a, b, c)
        if self.skip(&TokenKind::Comma) {
            let mut elements = vec![first];
            while !self.at(&TokenKind::RParen) && !self.at_end() {
                elements.push(self.parse_expression(BP_LOWEST)?);
                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
            if !self.expect(&TokenKind::RParen) {
                return None;
            }
            return Some(Expr::Tuple(elements, start_span));
        }

        // Just grouped expression: (expr)
        if !self.expect(&TokenKind::RParen) {
            return None;
        }
        Some(first)
    }

    /// Parse block expression
    fn parse_block(&mut self) -> Option<Expr> {
        let start_span = self.span();
        if !self.expect(&TokenKind::LBrace) {
            return None;
        }

        let (stmts, expr) = self.parse_block_body()?;

        if !self.expect(&TokenKind::RBrace) {
            return None;
        }

        Some(Expr::Block(Block {
            stmts,
            expr,
            span: start_span,
        }))
    }

    /// Parse block body (statements and optional trailing expression)
    #[inline]
    pub(crate) fn parse_block_body(&mut self) -> Option<(Vec<Stmt>, Option<Box<Expr>>)> {
        let mut stmts = Vec::new();
        let mut expr = None;

        while !self.at(&TokenKind::RBrace) && !self.at_end() {
            if self.can_start_stmt() {
                if let Some(stmt) = self.parse_stmt() {
                    // If statement ends with expression and is the last statement,
                    // treat it as the block's expression
                    if self.at(&TokenKind::RBrace) {
                        if let StmtKind::Expr(e) = &stmt.kind {
                            expr = Some(Box::new(*e.clone()));
                            break;
                        }
                    }
                    stmts.push(stmt);
                    continue;
                }
            }
            // If we can't parse a statement, synchronize
            self.synchronize();
        }

        Some((stmts, expr))
    }

    /// Parse if expression
    fn parse_if(&mut self) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume 'if'

        let condition = self.parse_expression(BP_LOWEST)?;

        if !self.expect(&TokenKind::LBrace) {
            return None;
        }

        // Already past LBrace, use parse_block_body directly
        let (then_stmts, then_expr) = self.parse_block_body()?;

        if !self.expect(&TokenKind::RBrace) {
            return None;
        }

        let then_branch = Block {
            stmts: then_stmts,
            expr: then_expr,
            span: start_span,
        };

        // Parse elif branches
        let mut elif_branches = Vec::new();
        while self.skip(&TokenKind::KwElif) {
            let elif_condition = self.parse_expression(BP_LOWEST)?;
            if !self.expect(&TokenKind::LBrace) {
                return None;
            }
            let (elif_stmts, elif_expr) = self.parse_block_body()?;
            if !self.expect(&TokenKind::RBrace) {
                return None;
            }
            let elif_body = Block {
                stmts: elif_stmts,
                expr: elif_expr,
                span: self.span(),
            };
            elif_branches.push((Box::new(elif_condition), Box::new(elif_body)));
        }

        // Parse else branch
        let else_branch = if self.skip(&TokenKind::KwElse) {
            if !self.expect(&TokenKind::LBrace) {
                return None;
            }
            let (else_stmts, else_expr) = self.parse_block_body()?;
            if !self.expect(&TokenKind::RBrace) {
                return None;
            }
            Some(Box::new(Block {
                stmts: else_stmts,
                expr: else_expr,
                span: self.span(),
            }))
        } else {
            None
        };

        Some(Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            elif_branches,
            else_branch,
            span: start_span,
        })
    }

    /// Parse match expression
    fn parse_match(&mut self) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume 'match'

        let expr = self.parse_expression(BP_LOWEST)?;

        if !self.expect(&TokenKind::LBrace) {
            return None;
        }

        let mut arms = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at_end() {
            // Skip commas between arms
            self.skip(&TokenKind::Comma);
            if self.at(&TokenKind::RBrace) {
                break;
            }

            if let Some(arm) = self.parse_match_arm() {
                arms.push(arm);
            } else {
                self.synchronize();
            }
        }

        if !self.expect(&TokenKind::RBrace) {
            return None;
        }

        Some(Expr::Match {
            expr: Box::new(expr),
            arms,
            span: start_span,
        })
    }

    /// Parse match arm
    fn parse_match_arm(&mut self) -> Option<MatchArm> {
        let start_span = self.span();

        let pattern = self.parse_pattern()?;

        if !self.expect(&TokenKind::FatArrow) {
            return None;
        }

        let body = self.parse_expression(BP_LOWEST)?;

        self.skip(&TokenKind::Semicolon);

        Some(MatchArm {
            pattern,
            body,
            span: start_span,
        })
    }

    /// Parse pattern
    fn parse_pattern(&mut self) -> Option<Pattern> {
        let _start_span = self.span();

        match self.current().map(|t| &t.kind) {
            Some(TokenKind::Underscore) => {
                self.bump();
                Some(Pattern::Wildcard)
            }
            Some(TokenKind::Identifier(_)) => {
                let name = match self.current().map(|t| &t.kind) {
                    Some(TokenKind::Identifier(n)) => n.clone(),
                    _ => return None,
                };
                self.bump();

                // Check for struct pattern
                if self.skip(&TokenKind::LBrace) {
                    let mut fields = Vec::new();
                    while !self.at(&TokenKind::RBrace) && !self.at_end() {
                        let field_name = match self.current().map(|t| &t.kind) {
                            Some(TokenKind::Identifier(n)) => n.clone(),
                            _ => return None,
                        };
                        self.bump();
                        if !self.expect(&TokenKind::Colon) {
                            return None;
                        }
                        let field_pattern = self.parse_pattern()?;
                        fields.push((field_name, field_pattern));
                        self.skip(&TokenKind::Comma);
                    }
                    if !self.expect(&TokenKind::RBrace) {
                        return None;
                    }
                    return Some(Pattern::Struct { name, fields });
                }

                Some(Pattern::Identifier(name))
            }
            Some(TokenKind::IntLiteral(n)) => {
                let value = *n;
                self.bump();
                Some(Pattern::Literal(Literal::Int(value)))
            }
            Some(TokenKind::StringLiteral(s)) => {
                let value = s.clone();
                self.bump();
                Some(Pattern::Literal(Literal::String(value)))
            }
            Some(TokenKind::CharLiteral(c)) => {
                let value = *c;
                self.bump();
                Some(Pattern::Literal(Literal::Char(value)))
            }
            Some(TokenKind::BoolLiteral(b)) => {
                let value = *b;
                self.bump();
                Some(Pattern::Literal(Literal::Bool(value)))
            }
            Some(TokenKind::LParen) => {
                self.bump();
                let mut patterns = Vec::new();
                while !self.at(&TokenKind::RParen) && !self.at_end() {
                    patterns.push(self.parse_pattern()?);
                    self.skip(&TokenKind::Comma);
                }
                if !self.expect(&TokenKind::RParen) {
                    return None;
                }
                Some(Pattern::Tuple(patterns))
            }
            _ => {
                self.error(super::ParseError::InvalidPattern);
                None
            }
        }
    }

    /// Parse while expression
    fn parse_while(&mut self) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume 'while'

        let condition = self.parse_expression(BP_LOWEST)?;

        let label = self.parse_loop_label();

        if !self.expect(&TokenKind::LBrace) {
            return None;
        }
        let (body_stmts, body_expr) = self.parse_block_body()?;
        if !self.expect(&TokenKind::RBrace) {
            return None;
        }
        let body = Block {
            stmts: body_stmts,
            expr: body_expr,
            span: self.span(),
        };

        Some(Expr::While {
            condition: Box::new(condition),
            body: Box::new(body),
            label,
            span: start_span,
        })
    }

    /// Parse for expression
    fn parse_for(&mut self) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume 'for'

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

        if !self.expect(&TokenKind::KwIn) {
            return None;
        }
        let iterable = self.parse_expression(BP_LOWEST)?;

        let label = self.parse_loop_label();

        if !self.expect(&TokenKind::LBrace) {
            return None;
        }
        let (body_stmts, body_expr) = self.parse_block_body()?;
        if !self.expect(&TokenKind::RBrace) {
            return None;
        }
        let body = Block {
            stmts: body_stmts,
            expr: body_expr,
            span: self.span(),
        };

        Some(Expr::For {
            var,
            iterable: Box::new(iterable),
            body: Box::new(body),
            label,
            span: start_span,
        })
    }

    /// Parse loop label
    #[inline]
    pub(crate) fn parse_loop_label(&mut self) -> Option<String> {
        if self.skip(&TokenKind::ColonColon) {
            match self.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => {
                    let label = n.clone();
                    self.bump();
                    Some(label)
                }
                _ => None,
            }
        } else {
            None
        }
    }

    /// Parse lambda expression
    fn parse_lambda(&mut self) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume '|'

        let params = self.parse_lambda_params()?;

        if !self.expect(&TokenKind::FatArrow) {
            return None;
        }

        // Lambda body can be a block or single expression
        let body = if self.at(&TokenKind::LBrace) {
            self.parse_block_expression()?
        } else {
            let expr = self.parse_expression(BP_LOWEST)?;
            Block {
                stmts: vec![],
                expr: Some(Box::new(expr)),
                span: self.span(),
            }
        };

        Some(Expr::FnDef {
            name: String::new(), // Anonymous function
            params,
            return_type: None,
            body: Box::new(body),
            is_async: false,
            span: start_span,
        })
    }

    /// Parse lambda parameters
    fn parse_lambda_params(&mut self) -> Option<Vec<Param>> {
        // Empty params: || expr
        if self.at(&TokenKind::FatArrow) {
            return Some(vec![]);
        }

        if self.at(&TokenKind::Pipe) {
            return Some(vec![]);
        }

        let mut params = Vec::new();

        while !self.at(&TokenKind::Pipe) && !self.at_end() {
            // Skip leading comma
            if !params.is_empty() {
                if !self.expect(&TokenKind::Comma) {
                    return None;
                }
            }

            let param_span = self.span();
            let name = match self.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => n.clone(),
                _ => break,
            };
            self.bump();

            // Optional type annotation
            let ty = if self.skip(&TokenKind::Colon) {
                self.parse_type()
            } else {
                None
            };

            params.push(Param {
                name,
                ty,
                span: param_span,
            });
        }

        if !self.expect(&TokenKind::Pipe) {
            return None;
        }
        Some(params)
    }

    /// Parse a block and return it as an expression
    #[inline]
    pub(crate) fn parse_block_expression(&mut self) -> Option<Block> {
        let start_span = self.span();
        if !self.expect(&TokenKind::LBrace) {
            return None;
        }

        let (stmts, expr) = self.parse_block_body()?;

        if !self.expect(&TokenKind::RBrace) {
            return None;
        }

        Some(Block {
            stmts,
            expr,
            span: start_span,
        })
    }
}
