//! Precedence handling for Pratt parser
//!
//! This module defines binding power levels and precedence rules with RFC-010/011 support.

/// Binding power levels for Pratt parser
pub const BP_LOWEST: u8 = 0;
pub const BP_ASSIGN: u8 = 1;
pub const BP_LOGICAL_OR: u8 = 2;
pub const BP_LOGICAL_AND: u8 = 3;
pub const BP_EQUALITY: u8 = 4;
pub const BP_COMPARISON: u8 = 5;
pub const BP_TERM: u8 = 6;
pub const BP_FACTOR: u8 = 7;
pub const BP_UNARY: u8 = 8;
pub const BP_CAST: u8 = 8;
pub const BP_CALL: u8 = 9;
pub const BP_HIGHEST: u8 = 10;

/// Additional binding power levels for infix operators
pub const BP_RANGE: u8 = 1;
pub const BP_OR: u8 = 2;
pub const BP_AND: u8 = 3;
pub const BP_EQ: u8 = 4;
pub const BP_CMP: u8 = 5;
pub const BP_ADD: u8 = 6;
pub const BP_MUL: u8 = 7;

/// Precedence rules for the Pratt parser
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Precedence {
    /// Lowest precedence
    Lowest,
    /// Assignment (=)
    Assignment,
    /// Logical OR (||)
    LogicalOr,
    /// Logical AND (&&)
    LogicalAnd,
    /// Equality (==, !=)
    Equality,
    /// Comparison (<, <=, >, >=)
    Comparison,
    /// Term (+, -)
    Term,
    /// Factor (*, /, %)
    Factor,
    /// Unary operators (!, -, +)
    Unary,
    /// Function calls, field access, indexing
    Call,
    /// Highest precedence
    Highest,
}

impl Precedence {
    /// Convert precedence to binding power
    pub fn to_bp(self) -> u8 {
        match self {
            Precedence::Lowest => BP_LOWEST,
            Precedence::Assignment => BP_ASSIGN,
            Precedence::LogicalOr => BP_LOGICAL_OR,
            Precedence::LogicalAnd => BP_LOGICAL_AND,
            Precedence::Equality => BP_EQUALITY,
            Precedence::Comparison => BP_COMPARISON,
            Precedence::Term => BP_TERM,
            Precedence::Factor => BP_FACTOR,
            Precedence::Unary => BP_UNARY,
            Precedence::Call => BP_CALL,
            Precedence::Highest => BP_HIGHEST,
        }
    }

    /// Convert binding power to precedence
    pub fn from_bp(bp: u8) -> Option<Self> {
        match bp {
            BP_LOWEST => Some(Precedence::Lowest),
            BP_ASSIGN => Some(Precedence::Assignment),
            BP_LOGICAL_OR => Some(Precedence::LogicalOr),
            BP_LOGICAL_AND => Some(Precedence::LogicalAnd),
            BP_EQUALITY => Some(Precedence::Equality),
            BP_COMPARISON => Some(Precedence::Comparison),
            BP_TERM => Some(Precedence::Term),
            BP_FACTOR => Some(Precedence::Factor),
            BP_UNARY => Some(Precedence::Unary),
            BP_CALL => Some(Precedence::Call),
            BP_HIGHEST => Some(Precedence::Highest),
            _ => None,
        }
    }
}

/// Precedence parsing context for the Pratt parser
#[derive(Debug, Clone)]
pub struct PrecedenceContext {
    /// Current minimum binding power
    pub min_bp: u8,
    /// Current precedence level
    pub precedence: Precedence,
}

impl PrecedenceContext {
    /// Create a new precedence context
    pub fn new(min_bp: u8) -> Self {
        Self {
            min_bp,
            precedence: Precedence::from_bp(min_bp).unwrap_or(Precedence::Lowest),
        }
    }

    /// Check if the current precedence is higher than the minimum
    pub fn can_parse(&self) -> bool {
        self.precedence.to_bp() >= self.min_bp
    }

    /// Increase the minimum binding power
    pub fn increase_bp(
        &self,
        bp: u8,
    ) -> Self {
        Self::new(bp)
    }

    /// Decrease the minimum binding power
    pub fn decrease_bp(
        &self,
        bp: u8,
    ) -> Self {
        Self::new(self.min_bp.saturating_sub(bp))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precedence_conversion_all() {
        assert_eq!(Precedence::Lowest.to_bp(), BP_LOWEST);
        assert_eq!(Precedence::Assignment.to_bp(), BP_ASSIGN);
        assert_eq!(Precedence::LogicalOr.to_bp(), BP_LOGICAL_OR);
        assert_eq!(Precedence::LogicalAnd.to_bp(), BP_LOGICAL_AND);
        assert_eq!(Precedence::Equality.to_bp(), BP_EQUALITY);
        assert_eq!(Precedence::Comparison.to_bp(), BP_COMPARISON);
        assert_eq!(Precedence::Term.to_bp(), BP_TERM);
        assert_eq!(Precedence::Factor.to_bp(), BP_FACTOR);
        assert_eq!(Precedence::Unary.to_bp(), BP_UNARY);
        assert_eq!(Precedence::Call.to_bp(), BP_CALL);
        assert_eq!(Precedence::Highest.to_bp(), BP_HIGHEST);
    }

    #[test]
    fn test_bp_to_precedence_all() {
        assert_eq!(Precedence::from_bp(BP_LOWEST), Some(Precedence::Lowest));
        assert_eq!(Precedence::from_bp(BP_ASSIGN), Some(Precedence::Assignment));
        assert_eq!(
            Precedence::from_bp(BP_LOGICAL_OR),
            Some(Precedence::LogicalOr)
        );
        assert_eq!(
            Precedence::from_bp(BP_LOGICAL_AND),
            Some(Precedence::LogicalAnd)
        );
        assert_eq!(Precedence::from_bp(BP_EQUALITY), Some(Precedence::Equality));
        assert_eq!(
            Precedence::from_bp(BP_COMPARISON),
            Some(Precedence::Comparison)
        );
        assert_eq!(Precedence::from_bp(BP_TERM), Some(Precedence::Term));
        assert_eq!(Precedence::from_bp(BP_FACTOR), Some(Precedence::Factor));
        assert_eq!(Precedence::from_bp(BP_UNARY), Some(Precedence::Unary));
        assert_eq!(Precedence::from_bp(BP_CALL), Some(Precedence::Call));
        assert_eq!(Precedence::from_bp(BP_HIGHEST), Some(Precedence::Highest));
        assert_eq!(Precedence::from_bp(255), None);
    }

    #[test]
    fn test_precedence_context_all() {
        let ctx = PrecedenceContext::new(BP_ASSIGN);
        assert!(ctx.can_parse());
        assert_eq!(ctx.min_bp, BP_ASSIGN);
        assert_eq!(ctx.precedence, Precedence::Assignment);

        let higher = ctx.increase_bp(BP_CALL);
        assert_eq!(higher.min_bp, BP_CALL);
        assert_eq!(higher.precedence, Precedence::Call);

        let lower = higher.decrease_bp(2);
        assert_eq!(lower.min_bp, BP_CALL - 2);

        let saturated = ctx.decrease_bp(100);
        assert_eq!(saturated.min_bp, 0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_bp_ordering() {
        assert!(BP_LOWEST < BP_ASSIGN);
        assert!(BP_ASSIGN < BP_LOGICAL_OR);
        assert!(BP_LOGICAL_AND < BP_EQUALITY);
        assert!(BP_COMPARISON < BP_TERM);
        assert!(BP_TERM < BP_FACTOR);
        assert!(BP_FACTOR <= BP_CALL);
        assert!(BP_CALL < BP_HIGHEST);
    }

    #[test]
    fn test_from_bp_invalid() {
        assert_eq!(Precedence::from_bp(11), None);
        assert_eq!(Precedence::from_bp(100), None);
    }

    #[test]
    fn test_precedence_context_can_parse() {
        let low = PrecedenceContext::new(BP_LOWEST);
        assert!(low.can_parse());
        let high = PrecedenceContext::new(BP_HIGHEST);
        assert!(high.can_parse());
    }
}
