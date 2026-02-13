//! 关键字测试

use crate::frontend::core::lexer::{tokenize, LexError, TokenKind};

#[cfg(test)]
mod lexer_keywords_tests {
    use super::*;

    #[test]
    fn test_type_is_not_keyword() {
        // RFC-010: 'type' is no longer a keyword, it's just an identifier
        let tokens = tokenize("type").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0].kind, TokenKind::Identifier(name) if name == "type"));
    }

    #[test]
    fn test_pub_keyword() {
        let tokens = tokenize("pub").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0].kind, TokenKind::KwPub));
    }

    #[test]
    fn test_use_keyword() {
        let tokens = tokenize("use").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0].kind, TokenKind::KwUse));
    }

    #[test]
    fn test_spawn_keyword() {
        let tokens = tokenize("spawn").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0].kind, TokenKind::KwSpawn));
    }

    #[test]
    fn test_ref_keyword() {
        let tokens = tokenize("ref").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0].kind, TokenKind::KwRef));
    }

    #[test]
    fn test_mut_keyword() {
        let tokens = tokenize("mut").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0].kind, TokenKind::KwMut));
    }

    #[test]
    fn test_if_else_keywords() {
        let tokens = tokenize("if else").unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(&tokens[0].kind, TokenKind::KwIf));
        assert!(matches!(tokens[1].kind, TokenKind::KwElse));
    }

    #[test]
    fn test_elif_keyword() {
        let tokens = tokenize("elif").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwElif));
    }

    #[test]
    fn test_match_keyword() {
        let tokens = tokenize("match").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwMatch));
    }

    #[test]
    fn test_while_for_keywords() {
        let tokens = tokenize("while for").unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].kind, TokenKind::KwWhile));
        assert!(matches!(tokens[1].kind, TokenKind::KwFor));
    }

    #[test]
    fn test_in_keyword() {
        let tokens = tokenize("in").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwIn));
    }

    #[test]
    fn test_return_keyword() {
        let tokens = tokenize("return").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwReturn));
    }

    #[test]
    fn test_break_keyword() {
        let tokens = tokenize("break").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwBreak));
    }

    #[test]
    fn test_continue_keyword() {
        let tokens = tokenize("continue").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwContinue));
    }

    #[test]
    fn test_as_keyword() {
        let tokens = tokenize("as").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwAs));
    }

    #[test]
    fn test_all_keywords() {
        let source =
            "type pub use spawn ref mut if elif else match while for in return break continue as";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 18);
    }

    #[test]
    fn test_keywords_are_not_identifiers() {
        let keywords = [
            "type", "pub", "use", "spawn", "ref", "mut", "if", "else", "while", "for", "in",
            "return", "break", "continue", "as",
        ];
        for kw in &keywords {
            let tokens = tokenize(kw).unwrap();
            assert_ne!(
                tokens[0].kind,
                TokenKind::Identifier(kw.to_string()),
                "{} should be keyword",
                kw
            );
        }
    }

    #[test]
    fn test_keywords_with_underscores() {
        let result = tokenize("while_");
        assert!(result.is_ok());
        if let TokenKind::Identifier(_) = result.unwrap()[0].kind {
            // expected
        }
    }
}
