//! Pratt Parser expression parsing

use super::ast::*;
use super::state::*;

impl<'a> ParserState<'a> {
    /// Parse an expression using Pratt parser
    ///
    /// # Arguments
    /// * `bp` - Minimum binding power to parse
    ///
    /// # Returns
    /// Parsed expression or None if parsing fails
    ///
    /// # Algorithm
    /// 1. Parse prefix expression (nud)
    /// 2. While next token is infix operator with binding power >= bp:
    ///    - Get infix binding powers
    ///    - Parse infix expression (led) with higher right binding power
    /// 3. Return expression
    #[inline]
    pub fn parse_expression(
        &mut self,
        min_bp: u8,
    ) -> Option<Expr> {
        self.parse_expression_inner(min_bp, BP_HIGHEST)
    }

    /// Inner Pratt parser implementation
    fn parse_expression_inner(
        &mut self,
        mut min_bp: u8,
        max_bp: u8,
    ) -> Option<Expr> {
        // Parse prefix expression (nud)
        let (left_bp, prefix_fn) = match self.prefix_info() {
            Some(info) => info,
            None => {
                self.error(super::ParseError::InvalidExpression);
                return None;
            },
        };

        // Only parse if prefix binding power meets minimum
        if left_bp < min_bp {
            self.error(super::ParseError::InvalidExpression);
            return None;
        }

        // Call prefix parser
        let mut lhs = (prefix_fn)(self)?;

        // Parse infix expressions (led) with loop
        loop {
            // Check for end of input or operators that can't continue
            if self.at_end() {
                break;
            }

            // Get infix info
            let (left_bp_infix, right_bp, infix_fn) = match self.infix_info() {
                Some(info) => info,
                None => break,
            };

            // Check if binding power is high enough
            if left_bp_infix < min_bp {
                break;
            }

            // Check if binding power exceeds maximum (for right-associative)
            if left_bp_infix > max_bp {
                break;
            }

            // Call infix parser with left binding power for the next expression
            lhs = (infix_fn)(self, lhs, left_bp_infix)?;

            // Update minimum binding power for next iteration using right binding power
            // This handles associativity correctly
            min_bp = right_bp;
        }

        Some(lhs)
    }

    /// Parse expression with right-associative operators
    fn parse_expression_right(
        &mut self,
        min_bp: u8,
    ) -> Option<Expr> {
        self.parse_expression_inner(min_bp, BP_LOWEST)
    }
}
