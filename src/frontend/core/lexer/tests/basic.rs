//! 基础测试 - 标识符、空白符、换行等

use crate::frontend::core::lexer::{tokenize, LexError, TokenKind};

#[cfg(test)]
mod lexer_basic_tests {
    use super::*;

    #[test]
    fn test_empty_source() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_whitespace() {
        let tokens = tokenize("   \t\n\r   ").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_single_char_identifier() {
        let tokens = tokenize("a").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_multi_char_identifier() {
        let tokens = tokenize("helloWorld").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::Identifier(name) = &tokens[0].kind {
            assert_eq!(name, "helloWorld");
        } else {
            panic!("Expected identifier");
        }
    }

    #[test]
    fn test_identifier_with_underscore() {
        let tokens = tokenize("my_variable test123").unwrap();
        assert_eq!(tokens.len(), 3);
        if let TokenKind::Identifier(name) = &tokens[0].kind {
            assert_eq!(name, "my_variable");
        }
    }

    #[test]
    fn test_standalone_underscore() {
        let tokens = tokenize("_").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0].kind, TokenKind::Underscore));
    }

    #[test]
    fn test_identifier_starting_with_underscore() {
        let tokens = tokenize("_foo").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::Identifier(name) = &tokens[0].kind {
            assert_eq!(name, "_foo");
        } else {
            panic!("Expected identifier");
        }
    }

    #[test]
    fn test_identifier_starting_with_underscore_and_number() {
        let tokens = tokenize("_123abc").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::Identifier(name) = &tokens[0].kind {
            assert_eq!(name, "_123abc");
        } else {
            panic!("Expected identifier");
        }
    }

    #[test]
    fn test_newline_handling() {
        let tokens = tokenize("a\nb").unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_multiple_whitespace_with_newline() {
        let tokens = tokenize("1   2\n\n   3").unwrap();
        assert_eq!(tokens.len(), 4);
    }

    #[test]
    fn test_long_identifier() {
        let tokens = tokenize("very_long_identifier_with_many_characters").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_mixed_code_with_newlines() {
        let source = "let x = 42\n\n// comment\n\nlet y = 99";
        let tokens = tokenize(source).unwrap();
        assert!(tokens.len() > 8);
    }

    #[test]
    fn test_whitespace_variants() {
        let source = "1\t2\r\n3 4";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 5);
    }

    #[test]
    fn test_identifier_with_numbers_and_underscores() {
        let tokens = tokenize("var_123_name").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_identifier_unicode() {
        let result = tokenize("变量");
        let _ = result;
    }

    #[test]
    fn test_very_long_identifier() {
        let long_name = "a".repeat(1000);
        let tokens = tokenize(&long_name).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_standalone_underscore_in_expression() {
        let tokens = tokenize("_ + 5").unwrap();
        assert_eq!(tokens.len(), 4);
        assert!(matches!(tokens[0].kind, TokenKind::Underscore));
    }

    #[test]
    fn test_empty_file() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_only_whitespace() {
        let tokens = tokenize("   \t\n\r   ").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_float_with_leading_underscore_error() {
        let tokens = tokenize("_123").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    }
}
