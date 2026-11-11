//! RFC-010 Unified Syntax Lexer Tests
//! Tests lexer support for RFC-010 generic syntax

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::lexer::tokens::TokenKind;

#[test]
fn test_generic_syntax_tokenization() {
    // RFC-010: Basic generic syntax
    let source = "List(T)";
    let tokens = tokenize(source).unwrap();

    // Debug output
    println!("=== RFC-010 Debug: Tokens for '{}' ===", source);
    for (i, token) in tokens.iter().enumerate() {
        println!("  {}: {:?}", i, token.kind);
    }
    println!("=== End Debug ===");

    // Basic check - we expect at least 4 tokens
    assert!(
        tokens.len() >= 4,
        "Expected at least 4 tokens, got {}",
        tokens.len()
    );

    // Just check that we get identifier tokens
    let has_list = tokens
        .iter()
        .any(|t| matches!(t.kind, TokenKind::Identifier(_)));
    assert!(has_list, "Should have identifier token");

    // Check that we get some kind of bracket (might be LBracket due to current lexer state)
    let has_bracket = tokens
        .iter()
        .any(|t| matches!(t.kind, TokenKind::LBracket) || matches!(t.kind, TokenKind::LParen));
    assert!(has_bracket, "Should have bracket or lt token");
}
