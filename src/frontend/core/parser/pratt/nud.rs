//! Prefix expression parsing (nud - null denotation)
//!
//! This module implements prefix parsing for the Pratt parser with RFC-010/011 support.

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::ParserState;
use crate::frontend::core::parser::pratt::precedence::*;
use crate::util::span::Span;

/// Extension trait for prefix parsing
pub trait PrefixParser {
    /// Parse prefix expression at current position
    fn parse_prefix(&mut self) -> Option<Expr>;
}

impl<'a> PrefixParser for ParserState<'a> {
    fn parse_prefix(&mut self) -> Option<Expr> {
        self.prefix_info().and_then(|(_bp, parser_fn)| {
            // For now, just call the parser function directly
            parser_fn(self)
        })
    }
}

impl<'a> ParserState<'a> {
    /// Get prefix binding power and parser for current token
    #[inline]
    #[allow(clippy::type_complexity)]
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
            // List literal or list comprehension
            Some(TokenKind::LBracket) => Some((BP_HIGHEST, Self::parse_list_or_comp)),
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
            // ref 关键字：创建 Arc
            Some(TokenKind::KwRef) => Some((BP_HIGHEST, Self::parse_ref)),
            // Control flow expressions (return, break, continue)
            Some(TokenKind::KwReturn) => Some((BP_LOWEST, Self::parse_return_expr)),
            Some(TokenKind::KwBreak) => Some((BP_LOWEST, Self::parse_break_expr)),
            Some(TokenKind::KwContinue) => Some((BP_LOWEST, Self::parse_continue_expr)),
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

    /// Parse ref expression: `ref expr` creates an Arc
    fn parse_ref(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'ref'

        // Parse operand with higher binding power
        let expr = self.parse_expression(BP_UNARY + 1)?;

        Some(Expr::Ref {
            expr: Box::new(expr),
            span,
        })
    }

    /// Parse return expression: `return [expr]`
    fn parse_return_expr(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'return'

        // Check if there's a value to return
        // Stop at common terminators: comma, RBrace, semicolon, or end
        let value =
            if self.at(&TokenKind::Semicolon) || self.at(&TokenKind::RBrace) || self.at_end() {
                None
            } else {
                self.parse_expression(BP_LOWEST)
            };

        Some(Expr::Return(value.map(Box::new), span))
    }

    /// Parse break expression: `break [label]`
    fn parse_break_expr(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'break'

        let label =
            if !self.at(&TokenKind::Semicolon) && !self.at(&TokenKind::RBrace) && !self.at_end() {
                self.parse_expression(BP_LOWEST)
            } else {
                None
            };

        // Convert Expr to String label if present
        let label_str = label.and_then(|expr| {
            if let Expr::Var(name, _) = expr {
                Some(name)
            } else {
                None
            }
        });

        Some(Expr::Break(label_str, span))
    }

    /// Parse continue expression: `continue [label]`
    fn parse_continue_expr(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'continue'

        let label =
            if !self.at(&TokenKind::Semicolon) && !self.at(&TokenKind::RBrace) && !self.at_end() {
                self.parse_expression(BP_LOWEST)
            } else {
                None
            };

        // Convert Expr to String label if present
        let label_str = label.and_then(|expr| {
            if let Expr::Var(name, _) = expr {
                Some(name)
            } else {
                None
            }
        });

        Some(Expr::Continue(label_str, span))
    }

    /// Parse identifier expression
    fn parse_identifier(&mut self) -> Option<Expr> {
        let span = self.span();
        let token = self.current().cloned()?;
        if let TokenKind::Identifier(name) = token.kind {
            self.bump();
            Some(Expr::Var(name, span))
        } else {
            None
        }
    }

    /// Parse integer literal expression
    fn parse_int_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        let token = self.current().cloned()?;
        if let TokenKind::IntLiteral(n) = token.kind {
            self.bump();
            Some(Expr::Lit(Literal::Int(n), span))
        } else {
            None
        }
    }

    /// Parse float literal expression
    fn parse_float_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        let token = self.current().cloned()?;
        if let TokenKind::FloatLiteral(f) = token.kind {
            self.bump();
            Some(Expr::Lit(Literal::Float(f), span))
        } else {
            None
        }
    }

    /// Parse string literal expression
    fn parse_string_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        let token = self.current().cloned()?;
        if let TokenKind::StringLiteral(s) = token.kind {
            self.bump();
            Some(Expr::Lit(Literal::String(s), span))
        } else {
            None
        }
    }

    /// Parse char literal expression
    fn parse_char_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        let token = self.current().cloned()?;
        if let TokenKind::CharLiteral(c) = token.kind {
            self.bump();
            Some(Expr::Lit(Literal::Char(c), span))
        } else {
            None
        }
    }

    /// Parse bool literal expression
    fn parse_bool_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        let token = self.current().cloned()?;
        if let TokenKind::BoolLiteral(b) = token.kind {
            self.bump();
            Some(Expr::Lit(Literal::Bool(b), span))
        } else {
            None
        }
    }

    /// Parse grouped expression or tuple: `(expr)` or `(expr, expr, ...)`
    fn parse_group_or_tuple(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '('

        if self.at(&TokenKind::RParen) {
            // Empty tuple: ()
            self.bump();
            return Some(Expr::Tuple(Vec::new(), span));
        }

        // Check if this looks like a typed parameter list: (a: Type, ...)
        // If the first element is identifier followed by ':', parse as typed params
        if let Some(params) = self.try_parse_typed_param_list() {
            return Some(Expr::Lambda {
                params,
                body: Box::new(Block {
                    stmts: Vec::new(),
                    expr: None,
                    span: self.span(),
                }),
                span,
            });
        }

        let first_expr = self.parse_expression(BP_LOWEST)?;

        if self.skip(&TokenKind::Comma) {
            // This is a tuple: (expr, expr, ...)
            let mut elements = vec![first_expr];

            while !self.at(&TokenKind::RParen) {
                let expr = self.parse_expression(BP_LOWEST)?;
                elements.push(expr);

                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }

            self.expect(&TokenKind::RParen);
            Some(Expr::Tuple(elements, span))
        } else {
            // This is a grouped expression: (expr)
            self.expect(&TokenKind::RParen);
            Some(first_expr)
        }
    }

    /// Try to parse a typed parameter list like (a: Int, b: String)
    /// Returns None if this is not a typed param list, and restores position
    fn try_parse_typed_param_list(&mut self) -> Option<Vec<Param>> {
        use crate::frontend::core::parser::statements::declarations::parse_type_annotation;

        let saved = self.save_position();

        // Check if first token is identifier followed by ':'
        let first_name =
            if let Some(TokenKind::Identifier(name)) = self.current().map(|t| t.kind.clone()) {
                name
            } else {
                self.restore_position(saved);
                return None;
            };

        let first_span = self.span();
        self.bump(); // consume identifier

        if !self.at(&TokenKind::Colon) {
            self.restore_position(saved);
            return None;
        }

        self.bump(); // consume ':'

        let first_type = parse_type_annotation(self)?;

        let mut params = vec![Param {
            name: first_name,
            ty: Some(first_type),
            span: first_span,
        }];

        // Parse remaining typed parameters
        while self.skip(&TokenKind::Comma) {
            if self.at(&TokenKind::RParen) {
                break;
            }

            let param_name =
                if let Some(TokenKind::Identifier(name)) = self.current().map(|t| t.kind.clone()) {
                    name
                } else {
                    self.restore_position(saved);
                    return None;
                };

            let param_span = self.span();
            self.bump();

            if !self.skip(&TokenKind::Colon) {
                self.restore_position(saved);
                return None;
            }

            let param_type = parse_type_annotation(self)?;

            params.push(Param {
                name: param_name,
                ty: Some(param_type),
                span: param_span,
            });
        }

        if !self.skip(&TokenKind::RParen) {
            self.restore_position(saved);
            return None;
        }

        Some(params)
    }

    /// Parse list literal or list comprehension: `[expr]` or `[expr for x in y]`
    fn parse_list_or_comp(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '['

        if self.at(&TokenKind::RBracket) {
            // Empty list: []
            self.bump();
            return Some(Expr::List(Vec::new(), span));
        }

        let first_expr = self.parse_expression(BP_LOWEST)?;

        // Check for list comprehension
        if self.skip(&TokenKind::KwFor) {
            let elements = vec![first_expr];
            self.parse_list_comp(span, elements)
        } else {
            // This is a list literal: [expr, expr, ...]
            let mut elements = vec![first_expr];

            while !self.at(&TokenKind::RBracket) {
                self.skip(&TokenKind::Comma);

                if self.at(&TokenKind::RBracket) {
                    break;
                }

                let expr = self.parse_expression(BP_LOWEST)?;
                elements.push(expr);
            }

            self.expect(&TokenKind::RBracket);
            Some(Expr::List(elements, span))
        }
    }

    /// Parse list comprehension
    fn parse_list_comp(
        &mut self,
        span: Span,
        elements: Vec<Expr>,
    ) -> Option<Expr> {
        // Parse pattern: `x`
        let pattern = self.parse_expression(BP_LOWEST)?;

        self.expect(&TokenKind::KwIn);

        // Parse iterable: `expr`
        let iterable = self.parse_expression(BP_LOWEST)?;

        self.expect(&TokenKind::RBracket);

        Some(Expr::ListComp {
            element: Box::new(elements[0].clone()),
            var: if let Expr::Var(name, _) = pattern {
                name
            } else {
                // Fallback: use a default variable name if pattern is not a simple identifier
                "_".to_string()
            },
            iterable: Box::new(iterable),
            condition: None,
            span,
        })
    }

    /// Parse block expression: `{ stmt; ... expr? }`
    pub fn parse_block(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '{'

        let mut stmts = Vec::new();

        while !self.at(&TokenKind::RBrace) && !self.at_end() {
            if self.at(&TokenKind::Semicolon) {
                // Empty statement, skip it
                self.bump();
                continue;
            }

            // Parse statement
            if let Some(stmt) = self.parse_statement() {
                stmts.push(stmt);
            } else {
                // Failed to parse statement, skip token to avoid infinite loop
                self.bump();
            }
        }

        self.expect(&TokenKind::RBrace);

        // Check if block ends with an expression (without semicolon)
        let expr = if stmts
            .last()
            .is_some_and(|s| matches!(s.kind, StmtKind::Expr(_)))
        {
            // Last statement is an expression, extract it
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

        Some(Expr::Block(Block { stmts, expr, span }))
    }

    /// Parse if expression: `if cond { then } else { else }`
    fn parse_if(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'if'

        // Parse condition
        let condition = self.parse_expression(BP_LOWEST)?;

        // Parse then branch
        let then_branch = self.parse_block_expr()?;

        // Parse optional else branch
        let else_branch = if self.skip(&TokenKind::KwElse) {
            if self.at(&TokenKind::KwIf) {
                // Else if: parse another if expression
                // For else-if, we need to parse it as a block containing the if expression
                let if_expr = self.parse_if()?;
                Some(Box::new(Block {
                    stmts: Vec::new(),
                    expr: Some(Box::new(if_expr)),
                    span: self.span(),
                }))
            } else {
                // Regular else: parse block
                Some(Box::new(self.parse_block_expr()?))
            }
        } else {
            None
        };

        Some(Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            elif_branches: Vec::new(), // No elif branches for simple if expressions
            else_branch,
            span,
        })
    }

    /// Parse elif branch helper
    fn parse_elif_branch(&mut self) -> Option<(Expr, Block)> {
        self.expect(&TokenKind::KwElif);

        let condition = self.parse_expression(BP_LOWEST)?;
        let body = self.parse_block_expr()?;

        Some((condition, body))
    }

    /// Parse match expression: `match expr { pattern => expr, ... }`
    fn parse_match(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'match'

        let expr = self.parse_expression(BP_LOWEST)?;
        self.expect(&TokenKind::LBrace);

        let mut arms = Vec::new();

        while !self.at(&TokenKind::RBrace) && !self.at_end() {
            // Parse pattern
            let pattern = self.parse_expression(BP_LOWEST)?;

            self.expect(&TokenKind::FatArrow);

            // Parse body expression
            let body = self.parse_expression(BP_LOWEST)?;

            arms.push(MatchArm {
                pattern: if let Expr::Var(name, _) = pattern {
                    Pattern::Identifier(name)
                } else {
                    // For now, use a simple pattern from the expression
                    // In a full implementation, you'd have a proper pattern parser
                    Pattern::Wildcard
                },
                body,
                span: self.span(),
            });

            // Skip comma if present
            self.skip(&TokenKind::Comma);
        }

        self.expect(&TokenKind::RBrace);

        Some(Expr::Match {
            expr: Box::new(expr),
            arms,
            span,
        })
    }

    /// Parse while expression: `while cond { body }`
    fn parse_while(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'while'

        let condition = self.parse_expression(BP_LOWEST)?;
        let body = self.parse_block_expr()?;

        Some(Expr::While {
            condition: Box::new(condition),
            body: Box::new(body),
            label: None, // No label for simple while expressions
            span,
        })
    }

    /// Parse for expression: `for pattern in iterable { body }`
    fn parse_for(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'for'

        let pattern = self.parse_expression(BP_LOWEST)?;
        self.expect(&TokenKind::KwIn);
        let iterable = self.parse_expression(BP_LOWEST)?;
        let body = self.parse_block_expr()?;

        Some(Expr::For {
            var: if let Expr::Var(name, _) = pattern {
                name
            } else {
                // Fallback: use a default variable name if pattern is not a simple identifier
                "_".to_string()
            },
            iterable: Box::new(iterable),
            body: Box::new(body),
            label: None, // No label for simple for expressions
            span,
        })
    }

    /// Helper to parse a block expression
    fn parse_block_expr(&mut self) -> Option<Block> {
        if let Expr::Block(block) = self.parse_block()? {
            Some(block)
        } else {
            // If we didn't get a block, create an empty one with the expression as the body
            Some(Block {
                stmts: Vec::new(),
                expr: None,
                span: self.span(),
            })
        }
    }
}
