//! Prefix expression parsing (nud - null denotation)
//!
//! This module implements prefix parsing for the Pratt parser with RFC-010/011 support.

use crate::ast::*;
use crate::frontend::core::lexer::lex_interpolations;
use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::pratt::precedence::*;
use crate::frontend::core::parser::ParserState;
use crate::util::diagnostic::ErrorCodeDefinition;
use crate::util::span::Span;

impl<'a> ParserState<'a> {
    /// Parse prefix expression at current position
    pub fn parse_prefix(&mut self) -> Option<Expr> {
        self.prefix_info().and_then(|(_bp, parser_fn)| {
            // For now, just call the parser function directly
            parser_fn(self)
        })
    }
}

impl<'a> ParserState<'a> {
    /// Get prefix binding power and parser for current token
    ///
    /// 前缀算子的绑定力与策略来自 `precedence::PREFIX_OPS`（唯一声明处）；
    /// 本函数只做「查表 + 原子层回落」，不再列第二份 token → 绑定力表。
    #[inline]
    #[allow(clippy::type_complexity)]
    pub(crate) fn prefix_info(&self) -> Option<(u8, fn(&mut Self) -> Option<Expr>)> {
        let kind = self.current().map(|t| &t.kind)?;
        if let Some(entry) = prefix_op(kind) {
            let parser_fn: fn(&mut Self) -> Option<Expr> = match entry.strategy {
                PrefixStrategy::Unary => Self::parse_unary,
                PrefixStrategy::Borrow => Self::parse_borrow,
            };
            return Some((entry.bp, parser_fn));
        }
        // 原子层（字面量 / 标识符 / 分组 / 块 / if / match …）：绑定力是原子层常量，
        // 不是逐算子的属性，故留在这里而不是进算子表。
        match kind {
            TokenKind::IntLiteral(_) => Some((BP_HIGHEST, Self::parse_int_literal)),
            TokenKind::FloatLiteral(_) => Some((BP_HIGHEST, Self::parse_float_literal)),
            TokenKind::StringLiteral(_) => Some((BP_HIGHEST, Self::parse_string_literal)),
            TokenKind::CharLiteral(_) => Some((BP_HIGHEST, Self::parse_char_literal)),
            TokenKind::BoolLiteral(_) => Some((BP_HIGHEST, Self::parse_bool_literal)),
            TokenKind::VoidLiteral => Some((BP_HIGHEST, Self::parse_void_literal)),
            // RFC-012: F-string literal
            TokenKind::FStringLiteral(_) => Some((BP_HIGHEST, Self::parse_fstring)),
            // Identifier or path
            TokenKind::Identifier(_) => Some((BP_HIGHEST, Self::parse_identifier)),
            // Wildcard pattern (_)
            TokenKind::Underscore => Some((BP_HIGHEST, Self::parse_wildcard)),
            // Grouped expression or tuple
            TokenKind::LParen => Some((BP_HIGHEST, Self::parse_group_or_tuple)),
            // List literal or list comprehension
            TokenKind::LBracket => Some((BP_HIGHEST, Self::parse_list_or_comp)),
            // Block expression
            TokenKind::LBrace => Some((BP_HIGHEST, Self::parse_block)),
            // If expression
            TokenKind::KwIf => Some((BP_HIGHEST, Self::parse_if)),
            // Match expression
            TokenKind::KwMatch => Some((BP_HIGHEST, Self::parse_match)),
            // While expression
            TokenKind::KwWhile => Some((BP_HIGHEST, Self::parse_while)),
            // For expression
            TokenKind::KwFor => Some((BP_HIGHEST, Self::parse_for)),
            // ref 关键字：创建 Arc（前缀策略见 PREFIX_OPS；保留在原子层是因为它绑定 body 整块）
            TokenKind::KwRef => Some((BP_HIGHEST, Self::parse_ref)),
            // unsafe 块：系统级操作
            TokenKind::KwUnsafe => Some((BP_HIGHEST, Self::parse_unsafe)),
            // Spawn expression: spawn { ... } or spawn for ... in ... { ... }
            TokenKind::KwSpawn => Some((BP_HIGHEST, Self::parse_spawn)),
            // Control flow expressions (return)
            // #314：break/continue 移出表达式位置——仅语句，Python 风格定案
            TokenKind::KwReturn => Some((BP_LOWEST, Self::parse_return_expr)),
            _ => None,
        }
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

    /// Parse unsafe block: `unsafe { ... }`
    fn parse_unsafe(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'unsafe'

        // Parse the block body
        let body = if self.at(&TokenKind::LBrace) {
            self.parse_block_expr()?
        } else {
            // Unsafe without braces is a single expression
            let expr = self.parse_expression(BP_UNARY)?;
            // 合成体语句：不经过 parse_statement（身份的统一发放点），就地发放身份
            let span = self.span();
            let plan_id = self.alloc_plan_id();
            Block {
                stmts: vec![Stmt {
                    kind: StmtKind::Expr(Box::new(expr)),
                    span,
                    plan_id,
                }],
                span,
            }
        };

        Some(Expr::Unsafe {
            body: Box::new(body),
            span,
        })
    }

    /// Parse spawn expression: `spawn { ... }` or `spawn for ... in ... { ... }`
    fn parse_spawn(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'spawn'

        match self.current().map(|t| &t.kind) {
            // spawn for item in items { ... }
            Some(TokenKind::KwFor) => self.parse_spawn_for(span),
            // spawn { ... }
            Some(TokenKind::LBrace) => self.parse_spawn_block(span),
            _ => {
                self.error(
                    ErrorCodeDefinition::expected_expression("block or 'for' after spawn")
                        .at(self.span())
                        .build(),
                );
                None
            }
        }
    }

    /// Parse spawn block: `spawn { ... }`
    fn parse_spawn_block(
        &mut self,
        span: Span,
    ) -> Option<Expr> {
        let body = self.parse_block_expr()?;
        Some(Expr::Spawn {
            body: Box::new(body),
            span,
        })
    }

    /// Parse spawn for: `spawn for item in items { ... }`
    fn parse_spawn_for(
        &mut self,
        span: Span,
    ) -> Option<Expr> {
        self.bump(); // consume 'for'

        // 解析迭代变量名
        let var = match self.current() {
            Some(tok) if matches!(tok.kind, TokenKind::Identifier(_)) => {
                let name = match &tok.kind {
                    TokenKind::Identifier(n) => n.clone(),
                    _ => unreachable!(),
                };
                self.bump();
                name
            }
            _ => {
                self.error(
                    ErrorCodeDefinition::expected_expression("variable name after 'spawn for'")
                        .at(self.span())
                        .build(),
                );
                return None;
            }
        };

        // 可选 mut
        let var_mut = self.at(&TokenKind::KwMut);
        if var_mut {
            self.bump();
        }

        // 期望 'in'
        if !self.at(&TokenKind::KwIn) {
            self.error(
                ErrorCodeDefinition::expected_expression("'in' after variable name")
                    .at(self.span())
                    .build(),
            );
            return None;
        }
        self.bump(); // consume 'in'

        // 解析可迭代表达式
        let iterable = self.parse_expression(BP_LOWEST)?;

        // 解析循环体
        let body = self.parse_block_expr()?;

        Some(Expr::SpawnFor {
            var,
            var_mut,
            iterable: Box::new(iterable),
            body: Box::new(body),
            span,
        })
    }

    /// Parse unary operator expression
    fn parse_unary(&mut self) -> Option<Expr> {
        let span = self.span();
        let op = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Minus) => UnOp::Neg,
            Some(TokenKind::Plus) => UnOp::Pos,
            Some(TokenKind::Not) => UnOp::Not,
            Some(TokenKind::Star) => UnOp::Deref,
            _ => return None,
        };
        self.bump();

        // Zig 式（SPEC §2.2 / RFC-010）：所有一元运算符（含 !）紧绑定，
        // 高于比较/算术（!a == b ≡ (!a) == b），只低于调用与成员访问
        let operand = self.parse_expression(BP_UNARY + 1)?;

        Some(Expr::UnOp {
            op,
            expr: Box::new(operand),
            span,
        })
    }

    /// Parse borrow expression: `&expr` or `&mut expr`
    fn parse_borrow(&mut self) -> Option<Expr> {
        let span = self.span();
        let mutable = match self.current().map(|t| &t.kind) {
            Some(TokenKind::Ampersand) => false,
            Some(TokenKind::MutRef) => true,
            _ => return None,
        };
        self.bump(); // consume & or &mut

        // Parse operand with higher binding power
        let expr = self.parse_expression(BP_UNARY + 1)?;

        Some(Expr::Borrow {
            mutable,
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

    /// Parse wildcard pattern expression (_)
    fn parse_wildcard(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '_'
        Some(Expr::Var("_".to_string(), span))
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

    /// Parse void literal expression
    fn parse_void_literal(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'void'
        Some(Expr::Lit(Literal::Void, span))
    }

    /// RFC-012: Parse f-string literal into FString AST node
    ///
    /// 词法期已把内容切成段（FStringToken.segments）。本方法只做两件事：
    /// 1) 对本 f-string 的全部插值表达式做一次性批量词法（lex_interpolations）；
    /// 2) 逐段建 AST，插值段用独立解析器实例解析其 token 流。
    ///
    /// parser 不再逐插值重入公共词法入口（05 §f-string 嵌套编译必须显式处理）。
    fn parse_fstring(&mut self) -> Option<Expr> {
        let span = self.span();
        let token = self.current().cloned()?;
        if let TokenKind::FStringLiteral(fstring) = token.kind {
            self.bump();
            let segments = self.parse_fstring_segments(&fstring, span)?;
            Some(Expr::FString { segments, span })
        } else {
            None
        }
    }

    /// RFC-012: 把词法期切好的段转成 AST 段。
    ///
    /// 插值表达式一次性词法，结果按出现顺序与插值段一一对应；空插值在词法期已剔除。
    /// 解析失败时的既有回退语义逐字保留：回退成字面文本；深度闸拒绝额外传导
    /// E0019（issue #540）。
    fn parse_fstring_segments(
        &mut self,
        fstring: &FStringToken,
        span: Span,
    ) -> Option<Vec<FStringSegment>> {
        // 一次性满足本 f-string 全部插值的词法需求（N 个插值 1 次调用、单点实现）。
        // 每项携带插值 expr 起始的源文件绝对位置（段上的 base，#574）——插值
        // 子词法以它为词法起点，插值内诊断直接落到源文件绝对行列。
        let interpolation_sources: Vec<(&str, crate::util::span::Position)> = fstring
            .segments
            .iter()
            .filter_map(|segment| match segment {
                FStringTokenSegment::Text(_) => None,
                FStringTokenSegment::Interpolation { expr, base, .. } => {
                    Some((expr.as_str(), *base))
                }
            })
            .collect();
        let mut interpolation_tokens = lex_interpolations(&interpolation_sources).into_iter();

        let mut segments = Vec::new();
        for segment in &fstring.segments {
            match segment {
                FStringTokenSegment::Text(text) => {
                    segments.push(FStringSegment::Text(text.clone()));
                }
                FStringTokenSegment::Interpolation {
                    format_spec, raw, ..
                } => {
                    let literal_fallback = FStringSegment::Text(format!("{{{raw}}}"));
                    match interpolation_tokens.next() {
                        Some(Ok(tokens)) => {
                            let mut parser = ParserState::new(&tokens, self.probe());
                            // f-string 内嵌表达式深度继承外层（issue #540）：
                            // 内外各数各的会让「1024 层外层 + 1024 层内嵌」
                            // 双闸都拦不住，真实栈深已翻倍。
                            parser.inherit_parse_depth_from(self);
                            if let Some(expr) = parser
                                .parse_expression(crate::frontend::core::parser::pratt::BP_LOWEST)
                            {
                                segments.push(FStringSegment::Interpolation {
                                    expr: Box::new(expr),
                                    format_spec: format_spec.clone(),
                                });
                            } else {
                                // 深度闸拒绝传导（issue #540 修复轮）：内层实例的
                                // E0019 推进在它自己的 errors 里，实例本随回退丢弃
                                // ——不传导则程序编译成功、运行输出字面插值源文本。
                                // 其余解析失败的既有回退语义（字面文本）逐字保留，
                                // 只有深度闸拒绝这一条路径传导。
                                if parser.is_depth_rejected() {
                                    self.absorb_depth_rejection(parser.take_errors());
                                }
                                segments.push(literal_fallback);
                            }
                        }
                        // 词法错误 / 数量不匹配：回退成字面文本（既有语义）
                        _ => segments.push(literal_fallback),
                    }
                }
            }
        }

        let _ = span; // Used by caller for the FString node span
        Some(segments)
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
                    span: self.span(),
                }),
                span,
                plan_id: self.alloc_plan_id(),
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

    /// Try to parse a typed parameter list like (a: Int, b: String) or (mut a, b: Int)
    /// Returns None if this is not a typed param list, and restores position
    fn try_parse_typed_param_list(&mut self) -> Option<Vec<Param>> {
        use crate::frontend::core::parser::statements::types::parse_type_annotation;

        let saved = self.save_position();

        // Check for optional 'mut' keyword on first parameter
        let first_is_mut = if self.at(&TokenKind::KwMut) {
            self.bump();
            true
        } else {
            false
        };

        // Check if first token is identifier
        let first_name =
            if let Some(TokenKind::Identifier(name)) = self.current().map(|t| t.kind.clone()) {
                name
            } else {
                self.restore_position(saved);
                return None;
            };

        let first_span = self.span();
        self.bump(); // consume identifier

        // Parse optional type annotation
        let first_type = if self.at(&TokenKind::Colon) {
            self.bump(); // consume ':'
            parse_type_annotation(self)
        } else {
            None
        };

        // If no mut and no type annotation, this is not a param list
        if !first_is_mut && first_type.is_none() {
            self.restore_position(saved);
            return None;
        }

        let mut params = vec![Param {
            name: first_name,
            ty: first_type,
            is_mut: first_is_mut,
            quantity: None,
            span: first_span,
        }];

        // Parse remaining parameters
        while self.skip(&TokenKind::Comma) {
            if self.at(&TokenKind::RParen) {
                break;
            }

            // Check for optional 'mut' keyword
            let param_is_mut = if self.at(&TokenKind::KwMut) {
                self.bump();
                true
            } else {
                false
            };

            let param_name =
                if let Some(TokenKind::Identifier(name)) = self.current().map(|t| t.kind.clone()) {
                    name
                } else {
                    self.restore_position(saved);
                    return None;
                };

            let param_span = self.span();
            self.bump();

            // Parse optional type annotation
            let param_type = if self.skip(&TokenKind::Colon) {
                parse_type_annotation(self)
            } else {
                None
            };

            params.push(Param {
                name: param_name,
                ty: param_type,
                is_mut: param_is_mut,
                quantity: None,
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
            self.parse_list_comp(span, first_expr)
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

    /// Parse list comprehension: `[expr for x in y (if c)? (for v in w (if d)?)*]`
    ///
    /// #401：文法承诺的 `if` 过滤在此消费；多生成器子句按嵌套循环语义展开，
    /// 后续子句的 iterable/condition 可引用先前子句绑定的迭代变量。
    fn parse_list_comp(
        &mut self,
        span: Span,
        element: Expr,
    ) -> Option<Expr> {
        let mut generators = Vec::new();

        loop {
            // 迭代变量：只可能是裸标识符。'in' 已注册为中缀运算符（#299 §3），
            // pattern 不能走完整 pratt，否则会把 `in items` 吃成 membership 表达式
            let var_span = self.span();
            let var = {
                let tok = self.current()?;
                if let TokenKind::Identifier(name) = tok.kind.clone() {
                    self.bump();
                    name
                } else {
                    return None;
                }
            };

            self.expect(&TokenKind::KwIn);

            let iterable = self.parse_expression(BP_LOWEST)?;

            let condition = if self.skip(&TokenKind::KwIf) {
                Some(Box::new(self.parse_expression(BP_LOWEST)?))
            } else {
                None
            };

            generators.push(ListCompGenerator {
                var,
                iterable: Box::new(iterable),
                condition,
                span: var_span,
            });

            if !self.skip(&TokenKind::KwFor) {
                break;
            }
        }

        self.expect(&TokenKind::RBracket);

        Some(Expr::ListComp {
            element: Box::new(element),
            generators,
            span,
        })
    }

    /// Parse block expression: `{ stmt; ... expr? }`
    /// Parse block expression: `{ stmt; ... expr? }`
    ///
    /// # `{` 的唯一判定点
    ///
    /// 三种形态在此**一次**分开，下游不再各自判断：
    ///
    /// | 形态 | 结果 | 值 |
    /// | ---- | ---- | -- |
    /// | `{}` | 空块 | `Void` |
    /// | `{ "key": ... }` | 字典字面量 | `Dict(K, V)` |
    /// | 其它 | 块 | 尾表达式 |
    ///
    /// **`{}` 是空块而非空字典**：判据是内容——`Dict` 文法要求至少一个键，
    /// `{}` 无内容可依据，故取块结构的零形态（与 `unsafe {}` / `spawn {}` 一致）。
    /// 空块的值是 `Void`。
    ///
    /// 非空形态由**内容**决定：`{ "a": 1 }` 是字典（键值对可自描述），
    /// `{ println("f") }` 是块。
    pub fn parse_block(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume '{'

        // 形态一：空块 `{}`——无内容，取块结构的零形态（值 Void）
        if self.at(&TokenKind::RBrace) {
            self.bump();
            return Some(Expr::Block(Block {
                stmts: Vec::new(),
                span,
            }));
        }

        // Check if this looks like a dict literal
        // Dict literal pattern: string_literal ':' expr
        if let Some(TokenKind::StringLiteral(_)) = self.current().map(|t| &t.kind) {
            // Save position in case this is not a dict
            let saved = self.save_position();

            // Try to parse as dict
            if let Some(dict) = self.try_parse_dict(span) {
                return Some(dict);
            }

            // Not a dict, restore position and parse as block
            self.restore_position(saved);
        }

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

        // RFC-010a 规则①：块的值 = 尾表达式（唯一出口）。
        // 此处不过早剥离末尾表达式——块整体交给下游求值，
        // 由 L3 的类型检查 / IR 生成按尾表达式规则取值。
        Some(Expr::Block(Block { stmts, span }))
    }

    /// Try to parse a dict literal: {"key": value, "key2": value2, ...}
    /// Returns None if the syntax doesn't match a dict literal.
    fn try_parse_dict(
        &mut self,
        span: Span,
    ) -> Option<Expr> {
        let mut pairs = Vec::new();

        loop {
            // Parse key (must be a string literal)
            let key = match self.current().map(|t| &t.kind) {
                Some(TokenKind::StringLiteral(_)) => self.parse_expression(BP_LOWEST)?,
                _ => return None,
            };

            // Expect ':'
            if !self.skip(&TokenKind::Colon) {
                return None;
            }

            // Parse value
            let value = self.parse_expression(BP_LOWEST)?;

            pairs.push((key, value));

            // Check for comma or end
            if self.skip(&TokenKind::Comma) {
                // More pairs
                if self.at(&TokenKind::RBrace) {
                    break;
                }
            } else {
                // No comma, must be end
                break;
            }
        }

        // Expect '}'
        if !self.at(&TokenKind::RBrace) {
            return None;
        }
        self.bump(); // consume '}'

        Some(Expr::Dict(pairs, span))
    }

    /// Parse if expression: `if cond { then } else { else }`
    fn parse_if(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'if'

        // Parse condition
        let condition = self.parse_expression(BP_LOWEST)?;

        let then_branch = self.parse_block_expr()?;

        // Parse optional else if / else branches (flat syntax)
        let mut else_if_branches = Vec::new();
        let mut else_branch = None;

        while self.at(&TokenKind::KwElse) {
            self.bump(); // consume 'else'
            if self.at(&TokenKind::KwIf) {
                self.bump(); // consume 'if'
                let else_if_condition = self.parse_expression(BP_LOWEST)?;
                let else_if_body = self.parse_block_expr()?;
                else_if_branches.push((Box::new(else_if_condition), Box::new(else_if_body)));
            } else {
                // Regular else: parse block
                else_branch = Some(Box::new(self.parse_block_expr()?));
                break;
            }
        }

        Some(Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_if_branches,
            else_branch,
            span,
        })
    }

    /// Parse match expression: `match expr { pattern => expr, ... }`
    fn parse_match(&mut self) -> Option<Expr> {
        let span = self.span();
        self.bump(); // consume 'match'

        let expr = self.parse_expression(BP_LOWEST)?;
        self.expect(&TokenKind::LBrace);

        let mut arms = Vec::new();

        while !self.at(&TokenKind::RBrace) && !self.at_end() {
            // Parse pattern - use BP_LAMBDA + 1 to prevent => from being parsed as lambda
            // Lambda binding power is 11, so we use 12 to stop before =>
            //
            // RFC-010b: 变体模式 `ok(v)` 的调用结合力（BP_CALL=10）低于阈值 12，
            // parse_expression(BP_PATTERN_PROBE) 会把 `ok` 停成裸 Var、`(` 成意外 token——
            // 特判 Var 后跟 LParen 的形态，手动走 parse_call 重组调用
            //（expr_to_pattern 随后把它转成 Union 变体模式）。
            let pattern = {
                let mut patterns: Vec<Pattern> = vec![self.parse_pattern()?];
                // RFC-010b: 或模式 `1 | 2` / `some(v) | none()`——`|` 结合力
                // 低于模式解析阈值，表达式循环到不了它，这里显式收链
                while self.at(&TokenKind::Pipe) {
                    self.bump();
                    patterns.push(self.parse_pattern()?);
                }
                if patterns.len() == 1 {
                    patterns.pop().unwrap()
                } else {
                    Pattern::Or(patterns)
                }
            };
            // 守卫 `pat if cond -> ...`（syntax.md §2.8）：条件位是完整 Expr，
            // 臂箭头 `->` 不参与表达式文法，故条件解析自然停在 `->`——不需要
            // 任何「禁止 lambda”的旁路开关。
            let pattern = if self.at(&TokenKind::KwIf) {
                self.bump();
                let condition = self.parse_expression(BP_LOWEST);
                Pattern::Guard {
                    pattern: Box::new(pattern),
                    condition: condition?,
                }
            } else {
                pattern
            };

            // 分支箭头 `->`（syntax.md §2.8）。`=>` 只用于 lambda：遇到时给出
            // 可操作建议（per-site 文案键 E0011.branch_arrow.*），并消费该 token
            // 继续按分支体解析，避免级联诊断。
            if self.at(&TokenKind::FatArrow) {
                let arrow_span = self.span();
                let diagnostic =
                    crate::util::diagnostic::ErrorCodeDefinition::unexpected_token("=>")
                        .param_from_key("token", "E0011.branch_arrow.message")
                        .with_help_key("E0011.branch_arrow.help")
                        .at(arrow_span)
                        .build();
                self.error(diagnostic);
                self.bump();
            } else {
                self.expect(&TokenKind::Arrow);
            }

            // Parse body - can be a block expression or a simple expression
            let body = if self.at(&TokenKind::LBrace) {
                self.parse_block_expr()?
            } else {
                // Simple expression body - use lower precedence for the body
                let body_expr = self.parse_expression(BP_LOWEST)?;
                // 合成体语句：不经过 parse_statement（身份的统一发放点），就地发放身份
                let span = self.span();
                let plan_id = self.alloc_plan_id();
                Block {
                    stmts: vec![Stmt {
                        kind: StmtKind::Expr(Box::new(body_expr)),
                        span,
                        plan_id,
                    }],
                    span,
                }
            };

            arms.push(MatchArm {
                pattern,
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
            var_mut: false, // for 表达式不支持 mut 语法
            iterable: Box::new(iterable),
            body: Box::new(body),
            span,
        })
    }

    /// Helper to parse a block expression
    /// 解析块体并保证返回 Block。
    ///
    /// `parse_block` 只在字典字面量形态下返回非 Block（`{ "k": v }`）；
    /// 块体位置（函数体 / `unsafe` / `spawn` / `if` 分支）出现字典字面量是
    /// 语法错误——字典是**值**，不是代码体。
    ///
    /// 旧实现遇到非 Block 就静默回退成**空 Block**，把字典默默丢掉：
    /// `x = unsafe {"a": 1}` 会变空块且不报错。现改为显式诊断。
    fn parse_block_expr(&mut self) -> Option<Block> {
        match self.parse_block()? {
            Expr::Block(block) => Some(block),
            other => {
                let span = match &other {
                    Expr::Dict(_, s) => *s,
                    _ => self.span(),
                };
                self.error(
                    ErrorCodeDefinition::expected_expression(
                        "block body — a dict literal is a value, not a code block",
                    )
                    .at(span)
                    .build(),
                );
                None
            }
        }
    }

    /// 解析单个模式（RFC-010b）：parse_expression(BP_PATTERN_PROBE) 探测 + 三分支——
    /// Var+LParen 变体调用重组（`ok(v)` 停成裸 Var 的特判）、Var+LBrace
    /// 结构体模式（`Point { x, y: b }`，字段为简写绑定或 `名: 子模式`）、
    /// 其余经 expr_to_pattern 转换
    ///
    /// 解析递归深度闸的**模式汇点**（issue #540 修复轮）：struct 模式
    /// 字段臂（`名: 子模式`）直接自递归，每层的表达式探针 enter/exit 成对
    /// 不累积计数——模式递归栈不受闸保护（实测登记：`P{a: …}` 嵌套 1MB
    /// 参照线程崩溃起点 980 层）。本入口与类型汇点同机制过同一
    /// [`MAX_PARSE_DEPTH`](crate::frontend::core::parser::parser_state::MAX_PARSE_DEPTH)
    /// 计数，闸上发射 E0019 并以 `None` 逐层弹出。
    fn parse_pattern(&mut self) -> Option<Pattern> {
        if !self.enter_nested_parse() {
            return None;
        }
        let result = self.parse_pattern_nested();
        self.exit_nested_parse();
        result
    }

    /// [`Self::parse_pattern`] 的闸内本体（原实现逐字保留）。
    fn parse_pattern_nested(&mut self) -> Option<Pattern> {
        let probe = self.parse_expression(BP_PATTERN_PROBE)?;
        // issue #405：范围模式 `lo..hi`——`..` 中缀绑定力（BP_RANGE_LEFT/BP_RANGE_RIGHT）低于模式阈值 BP_PATTERN_PROBE，
        // pratt 循环停在 DotDot、probe 持有下界端点表达式，此处收编为 Range 模式。
        // 端点限定：字面量（负号前缀归一化为负值）；标识符端点在模式位是绑定，
        // 无判定语义，显式拒绝（E0010 点名范围端点）。上界不走 pratt 探测——
        // 负号上界的前缀操作数以 BP_UNARY+1 解析，`=>`（绑定力 11）会被吞成
        // lambda 中缀报 E0012，故上界按 token 直读。
        if self.at(&TokenKind::DotDot) {
            let span = self.span();
            let lo = match range_bound_from_expr(&probe) {
                Some(lit) => lit,
                None => {
                    self.error(
                        ErrorCodeDefinition::expected_token(
                            "integer literal (range pattern lower bound)",
                            &format!(
                                "{:?}",
                                self.current()
                                    .map(|t| t.kind.clone())
                                    .unwrap_or(TokenKind::Eof)
                            ),
                        )
                        .at(span)
                        .build(),
                    );
                    return None;
                }
            };
            self.bump(); // consume '..'
            let hi = match self.parse_range_bound() {
                Some(lit) => lit,
                None => {
                    self.error(
                        ErrorCodeDefinition::expected_token(
                            "integer literal (range pattern upper bound)",
                            &format!(
                                "{:?}",
                                self.current()
                                    .map(|t| t.kind.clone())
                                    .unwrap_or(TokenKind::Eof)
                            ),
                        )
                        .at(self.span())
                        .build(),
                    );
                    return None;
                }
            };
            return Some(Pattern::Range { lo, hi });
        }
        if self.at(&TokenKind::LParen) && matches!(&probe, Expr::Var(_, _)) {
            let called = match self.parse_call(probe.clone(), 0) {
                Some(c) => c,
                None => probe,
            };
            return Some(self.expr_to_pattern(&called));
        }
        if self.at(&TokenKind::LBrace) {
            if let Expr::Var(name, _) = &probe {
                self.bump(); // {
                let mut fields: Vec<(String, bool, Box<Pattern>)> = Vec::new();
                while !self.at(&TokenKind::RBrace) && !self.at_end() {
                    let fname = match self.current().map(|t| &t.kind) {
                        Some(TokenKind::Identifier(n)) => n.clone(),
                        _ => return None,
                    };
                    self.bump();
                    let fpat = if self.skip(&TokenKind::Colon) {
                        Box::new(self.parse_pattern()?)
                    } else {
                        Box::new(Pattern::Identifier(fname.clone()))
                    };
                    fields.push((fname, false, fpat));
                    if !self.skip(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RBrace);
                return Some(Pattern::Struct {
                    name: name.clone(),
                    fields,
                });
            }
        }
        Some(self.expr_to_pattern(&probe))
    }

    /// 范围模式上界端点：token 直读一个字面量（负号前缀归一化为负值）。
    /// 不走 pratt 探测（见 [`Self::parse_pattern_nested`] 内注释——负上界
    /// 会被 lambda 中缀吞掉）。返回 None = 当前 token 不是端点字面量，
    /// 调用点发射 E0010。
    fn parse_range_bound(&mut self) -> Option<Literal> {
        match self.current().map(|t| &t.kind) {
            Some(TokenKind::IntLiteral(n)) => {
                let v = *n;
                self.bump();
                Some(Literal::Int(v))
            }
            Some(TokenKind::FloatLiteral(f)) => {
                let v = *f;
                self.bump();
                Some(Literal::Float(v))
            }
            Some(TokenKind::BoolLiteral(b)) => {
                let v = *b;
                self.bump();
                Some(Literal::Bool(v))
            }
            Some(TokenKind::CharLiteral(c)) => {
                let v = *c;
                self.bump();
                Some(Literal::Char(v))
            }
            Some(TokenKind::StringLiteral(s)) => {
                let v = s.clone();
                self.bump();
                Some(Literal::String(v))
            }
            Some(TokenKind::VoidLiteral) => {
                self.bump();
                Some(Literal::Void)
            }
            Some(TokenKind::Minus) => {
                self.bump();
                match self.current().map(|t| &t.kind) {
                    Some(TokenKind::IntLiteral(n)) => {
                        let v = (*n).checked_neg()?;
                        self.bump();
                        Some(Literal::Int(v))
                    }
                    Some(TokenKind::FloatLiteral(f)) => {
                        let v = -*f;
                        self.bump();
                        Some(Literal::Float(v))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Convert an expression to a pattern for match arms
    #[allow(clippy::only_used_in_recursion)]
    fn expr_to_pattern(
        &mut self,
        expr: &Expr,
    ) -> Pattern {
        match expr {
            Expr::Var(name, _) => {
                if name == "_" {
                    Pattern::Wildcard
                } else {
                    Pattern::Identifier(name.clone())
                }
            }
            Expr::Lit(lit, _) => Pattern::Literal(lit.clone()),
            // RFC-010b: 嵌套或模式——载荷位的 `ok(1 | 2)`：实参表达式解析
            // 把 `|` 收成 BinOp(BitOr)，这里还原为 Or 模式（顶层或模式由
            // parse_match 的显式收链处理，走不到此臂）
            Expr::BinOp {
                op: crate::ast::BinOp::BitOr,
                left,
                right,
                ..
            } => Pattern::Or(vec![
                self.expr_to_pattern(left),
                self.expr_to_pattern(right),
            ]),
            Expr::Tuple(elements, _) => {
                let patterns = elements.iter().map(|e| self.expr_to_pattern(e)).collect();
                Pattern::Tuple(patterns)
            }
            Expr::Call { func, args, .. } => {
                // Constructor pattern: Name(patterns)
                if let Expr::Var(name, _) = func.as_ref() {
                    // 零载荷变体模式：`none()` 空括号 = 无载荷绑定（Union 内
                    // 模式为 None）。此前空参落进下方多参分支被编码成空元组
                    // 内模式，L3 的类型检查 / IR 按「尚未实现的元组模式」拒绝——
                    // 零载荷变体从此无法 match。
                    if args.is_empty() {
                        return Pattern::Union {
                            name: name.clone(),
                            variant: name.clone(),
                            pattern: None,
                        };
                    }
                    // Check if this looks like a struct pattern: Name { field, field, ... }
                    // This would be parsed as a call with the struct literal
                    if args.len() == 1 {
                        if let Expr::Block(block) = &args[0] {
                            // This is a struct pattern: Name { ... }
                            let fields = self.parse_struct_pattern_fields(block);
                            return Pattern::Struct {
                                name: name.clone(),
                                fields,
                            };
                        }
                        Pattern::Union {
                            name: name.clone(),
                            variant: name.clone(),
                            pattern: Some(Box::new(self.expr_to_pattern(&args[0]))),
                        }
                    } else {
                        // Multiple args -> tuple pattern inside union
                        let tuple_pattern =
                            Pattern::Tuple(args.iter().map(|e| self.expr_to_pattern(e)).collect());
                        Pattern::Union {
                            name: name.clone(),
                            variant: name.clone(),
                            pattern: Some(Box::new(tuple_pattern)),
                        }
                    }
                } else {
                    Pattern::Wildcard
                }
            }
            Expr::Block(block) => {
                // Check if this is a struct pattern: { x, mut y, ... }
                // by looking for identifier patterns separated by commas
                let fields = self.parse_struct_pattern_fields(block);
                Pattern::Struct {
                    name: String::new(), // Anonymous struct
                    fields,
                }
            }
            // 处理字段访问模式: `obj.field` 作为字段名
            Expr::FieldAccess { expr, field, .. } => {
                // 将字段访问转换为模式
                if let Expr::Var(name, _) = expr.as_ref() {
                    // 可能是一个简单的字段模式
                    Pattern::Identifier(format!("{}.{}", name, field))
                } else {
                    Pattern::Wildcard
                }
            }
            _ => Pattern::Wildcard,
        }
    }

    /// Parse fields from a block for struct pattern matching
    /// Supports: { x, mut y, x: pat, mut y: pat }
    #[allow(clippy::only_used_in_recursion)]
    fn parse_struct_pattern_fields(
        &mut self,
        block: &Block,
    ) -> Vec<(String, bool, Box<Pattern>)> {
        let mut fields = Vec::new();

        // Parse the block's last expression statement (comma-separated identifiers/patterns)
        if let Some(last_stmt) = block.stmts.last() {
            if let StmtKind::Expr(expr) = &last_stmt.kind {
                match expr.as_ref() {
                    Expr::Tuple(elements, _) => {
                        for element in elements {
                            if let Some((name, is_mut, pattern)) = self.parse_pattern_field(element)
                            {
                                fields.push((name, is_mut, Box::new(pattern)));
                            } else {
                                self.report_invalid_pattern_field(element);
                            }
                        }
                    }
                    _ => {
                        if let Some((name, is_mut, pattern)) = self.parse_pattern_field(expr) {
                            fields.push((name, is_mut, Box::new(pattern)));
                        } else {
                            self.report_invalid_pattern_field(expr);
                        }
                    }
                }
            }
        }

        fields
    }

    /// 结构模式字段非法时报错：此前静默剔除字段，模式变得比源码更宽，
    /// 会匹配到不该匹配的值（审计发现）
    fn report_invalid_pattern_field(
        &mut self,
        element: &Expr,
    ) {
        let span = match element {
            Expr::Var(_, s) => *s,
            Expr::Lit(_, s) => *s,
            Expr::Tuple(_, s) => *s,
            Expr::Call { span, .. } => *span,
            Expr::UnOp { span, .. } => *span,
            Expr::BinOp { span, .. } => *span,
            Expr::FieldAccess { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            _ => self.span(),
        };
        self.error(
            ErrorCodeDefinition::invalid_syntax(
                "invalid struct pattern field: expected `name`, `mut name` or `Variant(pattern)`",
            )
            .at(span)
            .build(),
        );
    }

    /// Parse a single field in struct pattern: `x`, `mut y`, `x: pat`, `mut y: pat`
    fn parse_pattern_field(
        &mut self,
        expr: &Expr,
    ) -> Option<(String, bool, Pattern)> {
        match expr {
            Expr::Var(name, _) => {
                // Simple identifier: `x` or `mut x`
                // Check if name starts with "mut " prefix
                if name.starts_with("mut ") {
                    let field_name = name.trim_start_matches("mut ").trim().to_string();
                    if field_name.is_empty() {
                        return None;
                    }
                    // Default pattern for bare identifier
                    Some((field_name.clone(), true, Pattern::Identifier(field_name)))
                } else {
                    Some((name.clone(), false, Pattern::Identifier(name.clone())))
                }
            }
            Expr::Call { func, args, .. } => {
                // Named field with pattern: `x: pat` or `mut x: pat`
                if let Expr::Var(func_name, _) = func.as_ref() {
                    let mut is_mut = false;
                    let field_name = if func_name.starts_with("mut ") {
                        is_mut = true;
                        func_name.trim_start_matches("mut ").trim().to_string()
                    } else {
                        func_name.clone()
                    };

                    if field_name.is_empty() {
                        return None;
                    }

                    // Parse the pattern from the argument
                    let pattern = if !args.is_empty() {
                        self.expr_to_pattern(&args[0])
                    } else {
                        Pattern::Identifier(field_name.clone())
                    };

                    return Some((field_name, is_mut, pattern));
                }
                None
            }
            _ => None,
        }
    }
}

/// 范围模式下界端点：probe 表达式 → 端点字面量。
/// 接受字面量与「负号前缀 + 字面量」（归一化为负值）；其余形态（标识符/
/// 调用/表达式）返回 None——模式位标识符是绑定，非常量端点无判定语义，
/// 调用点发射 E0010。
fn range_bound_from_expr(expr: &Expr) -> Option<Literal> {
    match expr {
        Expr::Lit(lit, _) => Some(lit.clone()),
        Expr::UnOp {
            op: UnOp::Neg,
            expr,
            ..
        } => match expr.as_ref() {
            // 字面量经词法器恒非负（i128 域），checked_neg 只在 i128::MIN
            // 手工形态失败——该极端值无端点语义，返回 None 交调用点拒绝
            Expr::Lit(Literal::Int(n), _) => Some(Literal::Int((*n).checked_neg()?)),
            Expr::Lit(Literal::Float(f), _) => Some(Literal::Float(-*f)),
            _ => None,
        },
        _ => None,
    }
}
