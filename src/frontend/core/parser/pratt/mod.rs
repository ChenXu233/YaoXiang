//! Pratt parser implementation
//! Handles expression parsing with binding power

pub mod led;
pub mod nud;
pub mod precedence;

#[cfg(test)]
mod tests;

pub use precedence::*;

use crate::frontend::core::parser::ast::*;
use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ParserState;

/// Public entry point for expression parsing
pub fn parse_expression_impl(
    state: &mut ParserState<'_>,
    min_bp: u8,
) -> Option<Expr> {
    state.parse_expression_internal(min_bp)
}

/// 表达式是否以 Identifier/`)`/`]` 结尾（RFC-038 行首 `.` 链式续行的前提）。
fn expr_ends_chainable(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Var(_, _)
            | Expr::Call { .. }
            | Expr::Index { .. }
            | Expr::FieldAccess { .. }
            | Expr::List(_, _)
            | Expr::Tuple(_, _)
    )
}

/// 表达式结束行号（RFC-038 换行规则用）。
/// 所有 Expr 变体都携带 span，这里统一提取。
fn expr_end_line(expr: &Expr) -> usize {
    let span = match expr {
        Expr::Lit(_, s) => *s,
        Expr::Var(_, s) => *s,
        Expr::BinOp { span, .. } => *span,
        Expr::UnOp { span, .. } => *span,
        Expr::Call { span, .. } => *span,
        Expr::FnDef { span, .. } => *span,
        Expr::If { span, .. } => *span,
        Expr::Match { span, .. } => *span,
        Expr::While { span, .. } => *span,
        Expr::For { span, .. } => *span,
        Expr::SpawnFor { span, .. } => *span,
        Expr::Block(b) => b.span,
        Expr::Return(_, s) => *s,
        Expr::Break(s) => *s,
        Expr::Continue(s) => *s,
        Expr::Cast { span, .. } => *span,
        Expr::Tuple(_, s) => *s,
        Expr::List(_, s) => *s,
        Expr::ListComp { span, .. } => *span,
        Expr::Dict(_, s) => *s,
        Expr::In { span, .. } => *span,
        Expr::Index { span, .. } => *span,
        Expr::FieldAccess { span, .. } => *span,
        Expr::Try { span, .. } => *span,
        Expr::Ref { span, .. } => *span,
        Expr::Borrow { span, .. } => *span,
        Expr::Unsafe { span, .. } => *span,
        Expr::Spawn { span, .. } => *span,
        Expr::Lambda { span, .. } => *span,
        Expr::FString { span, .. } => *span,
        Expr::Error(s) => *s,
    };
    span.end.line
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
            // 使用 led.rs 的 infix_info 分发所有中缀解析
            let (bp_left, bp_right, parser_fn) = match self.infix_info() {
                Some(info) => info,
                None => break,
            };
            if bp_left < min_bp {
                break;
            }
            // RFC-038：换行终止语句。后缀运算符（调用/索引/字段访问/try）跨行时
            // 视为新语句开头，不再并入上一表达式；行首 `.` 链式续行例外（Swift 式）。
            if !self.can_continue_expression(&left) {
                break;
            }
            left = parser_fn(self, left, bp_right)?;
            continue;
        }

        Some(left)
    }

    /// RFC-038：判断当前 token 能否延续 left 表达式（换行规则）。
    ///
    /// - 同行：后缀/中缀都继续（`a(b)`、`a + b`）
    /// - 跨行后缀（`(` `[` `.` `?`）：不继续（行首开新语句）
    /// - 跨行行首 `.`：若 left 以 Identifier/`)`/`]` 结尾，则继续（链式续行，Swift 式）
    /// - 跨行中缀（二元运算符/`=`/`as` 等）：继续（行尾运算符续行，RFC-038 例外 2）
    /// - 括号深度 > 0 时换行不终止（隐式续行，RFC-038 例外 1）——由调用方括号内解析天然保证
    fn can_continue_expression(
        &self,
        left: &Expr,
    ) -> bool {
        let Some(cur) = self.current() else {
            return false;
        };
        // 同一行：无条件继续（保持现有行为）
        if expr_end_line(left) == cur.span.start.line {
            return true;
        }
        // 跨行：按 token 类别判断
        match &cur.kind {
            // 后缀运算符：跨行不继续（行首开新语句）
            TokenKind::LParen | TokenKind::LBracket | TokenKind::Question => false,
            // 行首 `.` 链式续行（RFC-038 例外 3，Swift 式）：
            // 仅当 left 以 Identifier/`)`/`]` 结尾才继续
            TokenKind::Dot => expr_ends_chainable(left),
            // 中缀运算符等其它 token：跨行继续（行尾运算符续行 / 未闭合括号由括号内解析处理）
            _ => true,
        }
    }
}
