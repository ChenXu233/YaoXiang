//! Parser state and error handling

use crate::frontend::core::lexer::tokens::*;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;
use std::collections::HashSet;

/// Temporary: wrap string as E0012 diagnostic（#324：挂当前 token 位置）
pub fn parse_msg(
    state: &ParserState<'_>,
    msg: impl Into<String>,
) -> Diagnostic {
    ErrorCodeDefinition::invalid_syntax(&msg.into())
        .at(state.span())
        .build()
}

pub struct ParserState<'a> {
    tokens: &'a [Token],
    pos: usize,
    errors: Vec<Diagnostic>,
    /// match 臂守卫模式（RFC-010b）：为真时 `=>` 不作为 lambda 中缀被
    /// 表达式循环消费——守卫表达式 `n if n > 3 => ...` 要完整解析 `n > 3`
    /// 并停在 `=>`。save/restore 语义，嵌套（守卫里再写 match）安全。
    pub(crate) no_fat_arrow: bool,
    /// RFC-027：本模块已解析的**谓词名**（返回 `Type` 的声明）。
    ///
    /// 用于判定类型标注里的类型应用是「编译期谓词应用」（实参位是**值**）
    /// 还是「const 泛型应用」（实参位是类型引用）。parser 不持有类型环境，
    /// 只能靠本遍已解析的声明累积——**谓词须先声明后使用**。
    predicate_names: HashSet<String>,
}

impl<'a> ParserState<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
            no_fat_arrow: false,
            predicate_names: HashSet::new(),
        }
    }

    /// RFC-027：登记一个谓词名（返回 `Type` 的声明）。
    pub(crate) fn declare_predicate(
        &mut self,
        name: &str,
    ) {
        self.predicate_names.insert(name.to_string());
    }

    /// RFC-027：本条声明是否引用已登记的谓词名。
    pub(crate) fn is_predicate_name(
        &self,
        name: &str,
    ) -> bool {
        self.predicate_names.contains(name)
    }
    pub fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
            || matches!(self.current().map(|t| &t.kind), Some(TokenKind::Eof))
    }
    pub fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }
    pub fn peek_nth(
        &self,
        n: usize,
    ) -> Option<&Token> {
        self.tokens.get(self.pos + n)
    }
    pub fn span(&self) -> Span {
        self.current().map(|t| t.span).unwrap_or(Span::dummy())
    }
    pub fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned()?;
        self.pos += 1;
        Some(t)
    }
    pub fn at(
        &self,
        kind: &TokenKind,
    ) -> bool {
        self.current().is_some_and(|t| &t.kind == kind)
    }
    pub fn skip(
        &mut self,
        kind: &TokenKind,
    ) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }
    pub fn expect(
        &mut self,
        kind: &TokenKind,
    ) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            let found = self
                .current()
                .map(|t| t.kind.clone())
                .unwrap_or(TokenKind::Eof);
            self.errors.push(
                ErrorCodeDefinition::expected_token(
                    &format!("{:?}", kind),
                    &format!("{:?}", found),
                )
                .at(self.span())
                .build(),
            );
            false
        }
    }
    pub fn error(
        &mut self,
        mut error: Diagnostic,
    ) {
        // #324：parser 错误缺 span 时自动挂当前 token 位置（咽喉点，覆盖全部站点）
        if error.span.is_none() {
            error.span = Some(self.span());
        }
        self.errors.push(error);
    }
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    pub fn first_error(&self) -> Option<&Diagnostic> {
        self.errors.first()
    }
    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }
    pub fn take_errors(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.errors)
    }
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }
    pub fn save_position(&self) -> usize {
        self.pos
    }
    pub fn restore_position(
        &mut self,
        pos: usize,
    ) {
        self.pos = pos;
    }
    pub fn truncate_errors(
        &mut self,
        len: usize,
    ) {
        self.errors.truncate(len);
    }
    pub fn can_start_stmt(&self) -> bool {
        !self.at_end() && !self.at(&TokenKind::Semicolon)
    }
    pub fn synchronize(&mut self) {
        while !self.at_end() {
            if self.at(&TokenKind::Semicolon) {
                self.bump();
                break;
            }
            if let Some(TokenKind::Identifier(_)) = self.current().map(|t| &t.kind) {
                if matches!(
                    self.peek().map(|t| &t.kind),
                    Some(TokenKind::Colon | TokenKind::Eq | TokenKind::LParen | TokenKind::LBrace)
                ) {
                    break;
                }
            }
            self.bump();
        }
    }
    pub fn parse_statement(&mut self) -> Option<crate::frontend::core::parser::ast::Stmt> {
        use crate::frontend::core::parser::statements::*;
        let ss = self.span();
        match self.current().map(|t| &t.kind) {
            Some(TokenKind::KwUse) => parse_use_stmt(self, ss),
            Some(TokenKind::KwReturn) => parse_return_stmt(self, ss),
            Some(TokenKind::KwBreak) => parse_break_stmt(self, ss),
            Some(TokenKind::KwContinue) => parse_continue_stmt(self, ss),
            Some(TokenKind::KwFor) => parse_for_stmt(self, ss),
            Some(TokenKind::KwWhile) => parse_while_stmt(self, ss),
            Some(TokenKind::KwIf) => parse_if_stmt(self, ss),
            Some(TokenKind::LBrace) => parse_block_stmt(self, ss),
            // `unsafe { ... }` 是表达式（nud.rs:parse_unsafe）。语句位置必须
            // 走表达式语句，否则落入下方「关键字误用」分支报 E0018（#347）。
            Some(TokenKind::KwUnsafe) => parse_expr_stmt(self, ss),
            Some(TokenKind::KwMut) => parse_var_stmt(self, ss),
            Some(TokenKind::KwPub) => parse_identifier_stmt(self, ss),
            Some(TokenKind::Identifier(_)) => parse_identifier_stmt(self, ss),
            Some(TokenKind::LParen) => parse_paren_destructure_stmt(self, ss),
            Some(TokenKind::Eof) | None => None,
            Some(TokenKind::At) => {
                self.error(ErrorCodeDefinition::unexpected_token("@").at(ss).build());
                None
            }
            Some(kw @ TokenKind::KwRef)
            | Some(kw @ TokenKind::KwElse)
            | Some(kw @ TokenKind::KwIn)
            | Some(kw @ TokenKind::KwAs) => {
                let kw = match kw {
                    TokenKind::KwRef => "ref",
                    TokenKind::KwElse => "else",
                    TokenKind::KwIn => "in",
                    TokenKind::KwAs => "as",
                    _ => "keyword",
                };
                self.error(ErrorCodeDefinition::keyword_as_name(kw).at(ss).build());
                self.bump();
                None
            }
            Some(_) => parse_expr_stmt(self, ss),
        }
    }
    pub fn parse_expression(
        &mut self,
        min_bp: u8,
    ) -> Option<crate::frontend::core::parser::ast::Expr> {
        crate::frontend::core::parser::pratt::parse_expression_impl(self, min_bp)
    }
}
