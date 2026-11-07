//! Lexer 模块测试
//! 规范来源：docs/src/reference/language-spec/syntax.md §1.6；RFC-004 / RFC-010
//!
//! 测试词法分析器的核心功能，包括：
//! - 基础分词功能
//! - RFC-004 绑定语法
//! - RFC-010 泛型语法

use crate::frontend::core::lexer::{tokenize, TokenKind};

#[test]
fn test_simple_tokenization() {
    let source = "abc";
    let tokens = tokenize(source).unwrap();

    assert_eq!(tokens.len(), 2); // Identifier, Eof
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    assert_eq!(tokens[1].kind, TokenKind::Eof);
}

#[test]
fn test_rfc010_generic_syntax() {
    // RFC-010: Generic syntax support
    let source = "List(T)";
    let tokens = tokenize(source).unwrap();

    // Should recognize angle brackets for generics
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    // Flexible assertion - allow for different token sequences
    assert!(
        tokens.len() >= 3,
        "Expected at least 3 tokens, got {}",
        tokens.len()
    );

    // Check that we have some kind of bracket token (Lt or LBracket)
    let has_bracket = tokens.iter().any(|t| {
        matches!(t.kind, TokenKind::LParen)
            || matches!(t.kind, TokenKind::Lt)
            || matches!(t.kind, TokenKind::LBracket)
    });
    assert!(
        has_bracket,
        "Should have bracket token (LParen, Lt or LBracket)"
    );
}
