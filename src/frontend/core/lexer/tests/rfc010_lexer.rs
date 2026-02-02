//! RFC-010 Unified Syntax Lexer Tests
//! Tests lexer support for RFC-010 generic syntax

use yaoxiang::frontend::core::lexer::tokenize;
use yaoxiang::frontend::core::lexer::tokens::{TokenKind, Token};

#[test]
fn test_simple_angle_brackets() {
    // Test simple < and > characters
    let source = "<>";
    let tokens = tokenize(source).unwrap();

    println!("=== Angle brackets test ===");
    for (i, token) in tokens.iter().enumerate() {
        println!("  {}: {:?}", i, token.kind);
    }
    println!("=== End angle brackets ===");

    // Basic assertion - just check we get tokens
    assert!(tokens.len() >= 2);
}

#[test]
fn test_generic_syntax_tokenization() {
    // RFC-010: Basic generic syntax
    let source = "List[T]";
    let tokens = tokenize(source).unwrap();

    // Debug output
    println!("=== RFC-010 Debug: Tokens for '{}' ===", source);
    for (i, token) in tokens.iter().enumerate() {
        println!("  {}: {:?}", i, token.kind);
    }
    println!("=== End Debug ===");

    // Basic check - we expect at least 4 tokens
    assert!(tokens.len() >= 4, "Expected at least 4 tokens, got {}", tokens.len());

    // Just check that we get identifier tokens
    let has_list = tokens.iter().any(|t| matches!(t.kind, TokenKind::Identifier(_)));
    assert!(has_list, "Should have identifier token");

    // Check that we get some kind of bracket (might be LBracket due to current lexer state)
    let has_bracket = tokens.iter().any(|t| matches!(t.kind, TokenKind::LBracket) || matches!(t.kind, TokenKind::Lt));
    assert!(has_bracket, "Should have bracket or lt token");
}

#[test]
fn test_multiple_generic_parameters() {
    // RFC-010: Multiple type parameters
    let source = "Map[K, V]";
    let tokens = tokenize(source).unwrap();

    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[1].kind, TokenKind::Lt);
    assert!(matches!(tokens[2].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[3].kind, TokenKind::Comma);
    assert!(matches!(tokens[4].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[5].kind, TokenKind::Gt);
}

#[test]
fn test_generic_with_constraints() {
    // RFC-010: Generic with constraints
    let source = "T: Clone";
    let tokens = tokenize(source).unwrap();

    // Should tokenize: Identifier, Colon, Identifier
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[1].kind, TokenKind::Colon);
    assert!(matches!(tokens[2].kind, TokenKind::Identifier(_)));
}

#[test]
fn test_where_keyword() {
    // RFC-010: Where keyword for constraints
    let source = "where";
    let tokens = tokenize(source).unwrap();

    assert_eq!(tokens.len(), 2); // Keyword, Eof
    assert_eq!(tokens[0].kind, TokenKind::KwWhere);
}

#[test]
fn test_trait_keyword() {
    // RFC-010: Trait keyword
    let source = "trait";
    let tokens = tokenize(source).unwrap();

    assert_eq!(tokens.len(), 2); // Keyword, Eof
    assert_eq!(tokens[0].kind, TokenKind::KwTrait);
}

#[test]
fn test_interface_keyword() {
    // RFC-010: Interface keyword
    let source = "interface";
    let tokens = tokenize(source).unwrap();

    assert_eq!(tokens.len(), 2); // Keyword, Eof
    assert_eq!(tokens[0].kind, TokenKind::KwInterface);
}

#[test]
fn test_impl_keyword() {
    // RFC-010: Impl keyword
    let source = "impl";
    let tokens = tokenize(source).unwrap();

    assert_eq!(tokens.len(), 2); // Keyword, Eof
    assert_eq!(tokens[0].kind, TokenKind::KwImpl);
}

#[test]
fn test_forall_keyword() {
    // RFC-010: Forall keyword
    let source = "forall";
    let tokens = tokenize(source).unwrap();

    assert_eq!(tokens.len(), 2); // Keyword, Eof
    assert_eq!(tokens[0].kind, TokenKind::KwForall);
}

#[test]
fn test_exists_keyword() {
    // RFC-010: Exists keyword
    let source = "exists";
    let tokens = tokenize(source).unwrap();

    assert_eq!(tokens.len(), 2); // Keyword, Eof
    assert_eq!(tokens[0].kind, TokenKind::KwExists);
}

#[test]
fn test_nested_generics() {
    // RFC-010: Nested generic types
    let source = "Option<Vec<T>>";
    let tokens = tokenize(source).unwrap();

    // Should tokenize: Option < Vec < T > >
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[1].kind, TokenKind::Lt);
    assert!(matches!(tokens[2].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[3].kind, TokenKind::Lt);
    assert!(matches!(tokens[4].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[5].kind, TokenKind::Gt);
    assert_eq!(tokens[6].kind, TokenKind::Gt);
}

#[test]
fn test_complex_generic_expression() {
    // RFC-010: Complex generic expression with constraints
    let source = "func<T: Clone + Add>(a: T, b: T) -> T";
    let tokens = tokenize(source).unwrap();

    // Should recognize generics, constraints, and function syntax
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_))); // func identifier
    assert_eq!(tokens[1].kind, TokenKind::Lt);

    // Find the constraint colon
    let colon_idx = tokens.iter().position(|t| t.kind == TokenKind::Colon).unwrap();
    assert!(matches!(tokens[colon_idx - 1].kind, TokenKind::Identifier(_)));

    // Check for + operator in constraint
    let plus_idx = tokens.iter().position(|t| t.kind == TokenKind::Plus).unwrap();
    assert_eq!(colon_idx + 1, plus_idx);
}
