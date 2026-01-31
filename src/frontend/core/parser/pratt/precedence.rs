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
    fn test_precedence_conversion() {
        assert_eq!(Precedence::Lowest.to_bp(), BP_LOWEST);
        assert_eq!(Precedence::Assignment.to_bp(), BP_ASSIGN);
        assert_eq!(Precedence::Call.to_bp(), BP_CALL);
        assert_eq!(Precedence::Highest.to_bp(), BP_HIGHEST);
    }

    #[test]
    fn test_bp_to_precedence() {
        assert_eq!(Precedence::from_bp(BP_LOWEST), Some(Precedence::Lowest));
        assert_eq!(Precedence::from_bp(BP_ASSIGN), Some(Precedence::Assignment));
        assert_eq!(Precedence::from_bp(BP_HIGHEST), Some(Precedence::Highest));
        assert_eq!(Precedence::from_bp(255), None);
    }

    #[test]
    fn test_precedence_context() {
        let ctx = PrecedenceContext::new(BP_ASSIGN);
        assert!(ctx.can_parse());
        assert_eq!(ctx.min_bp, BP_ASSIGN);
    }
}
