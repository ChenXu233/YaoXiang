//! Pratt parser implementation
//! Handles expression parsing with binding power

pub mod led;
pub mod nud;
pub mod precedence;

#[cfg(test)]
mod tests;

pub use nud::*;
pub use led::*;
pub use precedence::*;

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
            // 使用 led.rs 的 infix_info 分发所有中缀解析
            let (bp_left, bp_right, parser_fn) = match self.infix_info() {
                Some(info) => info,
                None => break,
            };
            if bp_left < min_bp {
                break;
            }
            left = parser_fn(self, left, bp_right)?;
            continue;
        }

        Some(left)
    }
}
