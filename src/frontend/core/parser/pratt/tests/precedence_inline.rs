//! Precedence 模块测试
//!
//! 测试优先级相关功能，包括：
//! - 优先级枚举转换
//! - 绑定常量验证
//! - 优先级上下文操作

use crate::frontend::core::parser::pratt::precedence::{
    Precedence, PrecedenceContext, BP_ASSIGN, BP_COMPARISON, BP_EQUALITY, BP_FACTOR,
    BP_HIGHEST, BP_LOGICAL_AND, BP_LOGICAL_OR, BP_LOWEST, BP_TERM, BP_UNARY, BP_CALL,
};

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
