//! RFC-004 Binding Syntax Lexer Tests
//! Tests lexer support for RFC-004 binding syntax

use yaoxiang::frontend::core::lexer::tokenize;
use yaoxiang::frontend::core::lexer::tokens::{TokenKind, Token};

#[test]
fn test_binding_syntax_tokenization() {
    // RFC-004: Basic binding syntax
    let source = "function[0, 1, 2]";
    let tokens = tokenize(source).unwrap();

    assert_eq!(tokens.len(), 8); // Identifier, LBracket, Int, Comma, Int, Comma, Int, RBracket

    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[1].kind, TokenKind::LBracket);
    assert!(matches!(tokens[2].kind, TokenKind::IntLiteral(0)));
    assert_eq!(tokens[3].kind, TokenKind::Comma);
    assert!(matches!(tokens[4].kind, TokenKind::IntLiteral(1)));
    assert_eq!(tokens[5].kind, TokenKind::Comma);
    assert!(matches!(tokens[6].kind, TokenKind::IntLiteral(2)));
    assert_eq!(tokens[7].kind, TokenKind::RBracket);
}

#[test]
fn test_binding_with_underscore() {
    // RFC-004: Position wildcard with underscore
    let source = "method[0, _]";
    let tokens = tokenize(source).unwrap();

    assert_eq!(tokens[2].kind, TokenKind::IntLiteral(0));
    assert_eq!(tokens[3].kind, TokenKind::Comma);
    assert_eq!(tokens[4].kind, TokenKind::Underscore);
}

#[test]
fn test_complex_binding_expression() {
    // RFC-004: Complex binding expression
    let source = "Type.method = function[0, 1, 2]";
    let tokens = tokenize(source).unwrap();

    // Should tokenize: Type . method = function [ 0 , 1 , 2 ]
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[1].kind, TokenKind::Dot);
    assert!(matches!(tokens[2].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[3].kind, TokenKind::Eq);
    assert!(matches!(tokens[4].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[5].kind, TokenKind::LBracket);
    assert!(matches!(tokens[6].kind, TokenKind::IntLiteral(0)));
    assert_eq!(tokens[7].kind, TokenKind::Comma);
}

#[test]
fn test_nested_binding_brackets() {
    // RFC-004: Nested brackets in expressions
    let source = "data[0][1]";
    let tokens = tokenize(source).unwrap();

    // Should tokenize: data [ 0 ] [ 1 ]
    assert_eq!(tokens[0].kind, TokenKind::Identifier("data".to_string()));
    assert_eq!(tokens[1].kind, TokenKind::LBracket);
    assert!(matches!(tokens[2].kind, TokenKind::IntLiteral(0)));
    assert_eq!(tokens[3].kind, TokenKind::RBracket);
    assert_eq!(tokens[4].kind, TokenKind::LBracket);
    assert!(matches!(tokens[5].kind, TokenKind::IntLiteral(1)));
    assert_eq!(tokens[6].kind, TokenKind::RBracket);
}

#[test]
fn test_binding_with_generic() {
    // RFC-004 + RFC-010: Binding with generic types
    let source = "List[T][0, 1]";
    let tokens = tokenize(source).unwrap();

    // Should tokenize: List < T > [ 0 , 1 ]
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[1].kind, TokenKind::Lt);
    assert!(matches!(tokens[2].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[3].kind, TokenKind::Gt);
    assert_eq!(tokens[4].kind, TokenKind::LBracket);
}
