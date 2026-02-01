//! Prefix expression parsing (nud - null denotation)

use super::super::lexer::tokens::*;
use super::ast::*;
use super::state::*;
use crate::util::span::Span;
/// Extension trait for prefix parsing
pub trait PrefixParser {
    /// Parse prefix expression at current position
    fn parse_prefix(
        &mut self,
        bp: u8,
    ) -> Option<Expr>;
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
        let value = if self.at(&TokenKind::Semicolon)
            || self.at(&TokenKind::RBrace)
            || self.at(&TokenKind::Comma)
            || self.at_end()
        {
            None
        } else {
            Some(Box::new(self.parse_expression(BP_LOWEST)?))
        };

        Some(Expr::Return(value, span))
    }

    /// Parse break expression: `break [::label]`
    fn parse_break_expr(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'break'

        let label = if self.at(&TokenKind::ColonColon) {
            self.parse_loop_label()
        } else {
            None
        };

        Some(Expr::Break(label, span))
    }

    /// Parse continue expression: `continue [::label]`
    fn parse_continue_expr(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'continue'

        let label = if self.at(&TokenKind::ColonColon) {
            self.parse_loop_label()
        } else {
            None
        };

        Some(Expr::Continue(label, span))
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
            // Check for lambda: () => body
            if self.at(&TokenKind::FatArrow) {
                return self.parse_lambda_body(vec![], start_span);
            }
            return Some(Expr::Tuple(vec![], start_span));
        }

        // Try to parse as lambda parameter list with type annotations
        // Save position before attempting param list parsing
        let param_list_start = self.save_position();

        // Attempt to parse parameter list
        let mut params = Vec::new();
        let mut is_param_list = true;

        loop {
            if self.at(&TokenKind::RParen) {
                // End of parameter list
                self.bump(); // consume RParen
                break;
            }

            // Parse parameter: identifier [: type]
            let param_name = match self.current().map(|t| &t.kind) {
                Some(TokenKind::Identifier(n)) => n.clone(),
                _ => {
                    // Not a parameter list
                    is_param_list = false;
                    self.restore_position(param_list_start);
                    break;
                }
            };
            self.bump(); // consume identifier

            // Parse optional type annotation
            let param_type = if self.skip(&TokenKind::Colon) {
                self.parse_type_anno()
            } else {
                None
            };

            // Get parameter span
            let param_span = self.span();

            params.push(Param {
                name: param_name,
                ty: param_type,
                span: param_span,
            });

            // Check for comma or closing paren
            if self.at(&TokenKind::Comma) {
                self.bump(); // consume comma
                continue;
            } else if self.at(&TokenKind::RParen) {
                self.bump(); // consume RParen
                break;
            } else {
                // Invalid syntax
                is_param_list = false;
                self.restore_position(param_list_start);
                break;
            }
        }

        // If we successfully parsed a parameter list and next token is =>, it's a lambda
        if is_param_list && self.at(&TokenKind::FatArrow) {
            return self.parse_lambda_body(params, start_span);
        }

        // If param parsing succeeded but it's not a lambda, restore position and parse normally
        // This handles cases like (a, b, c) = (1, 2, 3) which is tuple destructuring, not a lambda
        if is_param_list && !self.at(&TokenKind::FatArrow) {
            self.restore_position(param_list_start);
        }

        // We're rewound to after '(', now parse normally
        // Single element in parens: (expr)
        let first = self.parse_expression(BP_LOWEST)?;

        // Tuple with multiple elements: (a, b, c)
        let mut elements = vec![first];
        if self.skip(&TokenKind::Comma) {
            while !self.at(&TokenKind::RParen) && !self.at_end() {
                elements.push(self.parse_expression(BP_LOWEST)?);
                if !self.skip(&TokenKind::Comma) {
                    break;
                }
            }
        }

        if !self.expect(&TokenKind::RParen) {
            return None;
        }

        // Check for lambda: (params) => body
        if self.at(&TokenKind::FatArrow) {
            // Convert expressions to params
            let mut params = Vec::new();
            for expr in elements {
                // Support both simple identifiers and typed parameters: (x: Int) => ...
                match expr {
                    Expr::Var(name, span) => {
                        params.push(Param {
                            name,
                            ty: None,
                            span,
                        });
                    }
                    _ => {
                        // Invalid lambda parameter - we don't support complex expressions as params
                        self.error(super::ParseError::InvalidExpression { span: self.span() });
                        return None;
                    }
                }
            }
            return self.parse_lambda_body(params, start_span);
        }

        // If multiple elements, it's a tuple
        if elements.len() > 1 {
            return Some(Expr::Tuple(elements, start_span));
        }

        // Just grouped expression: (expr)
        Some(elements.into_iter().next().unwrap())
    }

    /// Parse list literal or list comprehension: [expr, ...] or [x for x in iter if cond]
    fn parse_list_or_comp(&mut self) -> Option<Expr> {
        let start_span = self.span();
        self.bump(); // consume '['

        // Check if this is a list comprehension: [expr for ...
        // Look ahead to see if we have identifier followed by KwFor
        if let Some(TokenKind::Identifier(_)) = self.current().map(|t| &t.kind) {
            let after_identifier = self.peek();
            if matches!(after_identifier.map(|t| &t.kind), Some(TokenKind::KwFor)) {
                return self.parse_list_comprehension(start_span);
            }
        }

        // Parse as regular list literal: [expr, expr, ...]
        let mut elements = Vec::new();
        while !self.at(&TokenKind::RBracket) && !self.at_end() {
            elements.push(self.parse_expression(BP_LOWEST)?);
            if !self.skip(&TokenKind::Comma) {
                break;
            }
        }

        if !self.expect(&TokenKind::RBracket) {
            return None;
        }

        Some(Expr::List(elements, start_span))
    }

    /// Parse list comprehension: [element for var in iterable if condition]
    fn parse_list_comprehension(
        &mut self,
        start_span: Span,
    ) -> Option<Expr> {
        // Parse element expression
        let element = self.parse_expression(BP_LOWEST)?;

        // Expect 'for'
        if !self.expect(&TokenKind::KwFor) {
            return None;
        }

        // Parse loop variable (must be identifier)
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

        // Expect 'in'
        if !self.expect(&TokenKind::KwIn) {
            return None;
        }

        // Parse iterable expression
        let iterable = self.parse_expression(BP_LOWEST)?;

        // Parse optional condition: 'if' expr
        let condition = if self.skip(&TokenKind::KwIf) {
            Some(Box::new(self.parse_expression(BP_LOWEST)?))
        } else {
            None
        };

        // Expect ']'
        if !self.expect(&TokenKind::RBracket) {
            return None;
        }

        Some(Expr::ListComp {
            element: Box::new(element),
            var,
            iterable: Box::new(iterable),
            condition,
            span: start_span,
        })
    }

    fn parse_lambda_body(
        &mut self,
        params: Vec<Param>,
        span: Span,
    ) -> Option<Expr> {
        self.bump(); // consume '=>'

        // Lambda body can be a block or single expression
        let body = if self.at(&TokenKind::LBrace) {
            // We need to parse block expression but return Block struct
            // parse_block returns Expr::Block
            if !self.expect(&TokenKind::LBrace) {
                return None;
            }
            let (stmts, expr) = self.parse_block_body()?;
            if !self.expect(&TokenKind::RBrace) {
                return None;
            }
            Block {
                stmts,
                expr,
                span: self.span(),
            }
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
            span,
        })
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
        self.parse_match_inner()
    }

    fn parse_match_inner(&mut self) -> Option<Expr> {
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

        // Parse body as block expression (supports both => expr and => { block })
        let body = self.parse_block_expr()?;

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

        // Check for Or pattern first: pattern | pattern | ...
        let first = self.parse_simple_pattern()?;

        // Check for Or (|) to combine patterns
        if self.at(&TokenKind::Pipe) {
            let mut patterns = vec![first];
            while self.skip(&TokenKind::Pipe) {
                patterns.push(self.parse_simple_pattern()?);
            }
            return Some(Pattern::Or(patterns));
        }

        // Check for Guard pattern: pattern if condition
        if self.skip(&TokenKind::KwIf) {
            let condition = self.parse_expression(BP_LOWEST)?;
            return Some(Pattern::Guard {
                pattern: Box::new(first),
                condition,
            });
        }

        Some(first)
    }

    /// Parse a simple pattern (without Or or Guard)
    fn parse_simple_pattern(&mut self) -> Option<Pattern> {
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

                // Check for Union/Variant pattern: Variant(pattern)
                if self.at(&TokenKind::LParen) {
                    self.bump(); // consume '('
                    let inner_pattern = if self.at(&TokenKind::RParen) {
                        None
                    } else {
                        Some(Box::new(self.parse_pattern()?))
                    };
                    if !self.expect(&TokenKind::RParen) {
                        return None;
                    }
                    let name_clone = name.clone();
                    return Some(Pattern::Union {
                        name,
                        variant: name_clone,
                        pattern: inner_pattern,
                    });
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
                self.error(super::ParseError::InvalidPattern { span: self.span() });
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
