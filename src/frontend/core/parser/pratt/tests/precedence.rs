//! Precedence / binding power tests

// The BP_* constants and Precedence enum are tested in pratt/precedence.rs.
// This file provides additional validation from the test suite side.

#[test]
fn test_precedence_ordering() {
    // Binding power should follow: Lowest < Assign < LogicalOr < LogicalAnd
    //   < Equality < Comparison < Term < Factor < Unary < Call < Highest
    let bp_lowest = crate::frontend::core::parser::BP_LOWEST;
    let bp_highest = crate::frontend::core::parser::BP_HIGHEST;
    assert!(bp_lowest < bp_highest);
}
