//! RFC Integration Tests
//! Tests end-to-end RFC-004, RFC-010, and RFC-011 functionality

#[cfg(test)]
mod rfc_integration_tests {
    use crate::frontend::core::tokenize;
    use crate::frontend::core::lexer::tokens::TokenKind;

    /// RFC-004 Integration Test: Basic Binding Syntax
    #[test]
    fn test_rfc004_basic_binding() {
        // Test basic binding syntax: function[0, 1, 2]
        let source = "func[0, 1, 2]";
        let tokens = tokenize(source).unwrap();

        // Verify tokenization
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
        assert_eq!(tokens[1].kind, TokenKind::LBracket);
        assert!(matches!(tokens[2].kind, TokenKind::IntLiteral(0)));
        assert_eq!(tokens[3].kind, TokenKind::Comma);
    }

    /// RFC-010 Integration Test: Generic Type Syntax
    #[test]
    fn test_rfc010_generic_types() {
        // Test generic type syntax: List[T], Map[K, V]
        let sources = vec!["List[T]", "Map[K, V]", "Option<Vec<T>>"];

        for source in sources {
            let tokens = tokenize(source).unwrap();
            assert!(
                tokens.len() >= 3,
                "Should have at least 3 tokens for {}",
                source
            );

            // Should have identifier, bracket/angle bracket, identifier pattern
            assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
        }
    }

    /// Cross-RFC Integration Test: Combining Features
    #[test]
    fn test_cross_rfc_features() {
        // Test combining RFC-004, RFC-010, and RFC-011 features
        let source = "func<T: Clone>[0, 1].method()";
        let tokens = tokenize(source).unwrap();

        // Should successfully tokenize combined syntax
        assert!(tokens.len() >= 10);

        // Should have:
        // - Identifier (func)
        // - Lt (for generic T)
        // - LBracket (for binding [0, 1])
        // - Dot (for method call)
        let has_identifier = tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Identifier(_)));
        let has_bracket = tokens.iter().any(|t| matches!(t.kind, TokenKind::LBracket));
        let has_dot = tokens.iter().any(|t| matches!(t.kind, TokenKind::Dot));

        assert!(
            has_identifier && has_bracket && has_dot,
            "Should have identifier, bracket, and dot for cross-RFC syntax"
        );
    }
}
