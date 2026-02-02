//! 分隔符测试

use crate::frontend::core::lexer::{tokenize, LexError, TokenKind};

#[cfg(test)]
mod lexer_delimiters_tests {
    use super::*;

    #[test]
    fn test_left_paren() {
        let tokens = tokenize("(").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::LParen));
    }

    #[test]
    fn test_right_paren() {
        let tokens = tokenize(")").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::RParen));
    }

    #[test]
    fn test_left_bracket() {
        let tokens = tokenize("[").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::LBracket));
    }

    #[test]
    fn test_right_bracket() {
        let tokens = tokenize("]").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::RBracket));
    }

    #[test]
    fn test_left_brace() {
        let tokens = tokenize("{").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::LBrace));
    }

    #[test]
    fn test_right_brace() {
        let tokens = tokenize("}").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::RBrace));
    }

    #[test]
    fn test_comma() {
        let tokens = tokenize(",").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Comma));
    }

    #[test]
    fn test_semicolon() {
        let tokens = tokenize(";").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Semicolon));
    }

    #[test]
    fn test_colon() {
        let tokens = tokenize(":").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Colon));
    }

    #[test]
    fn test_colon_colon() {
        let tokens = tokenize("::").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::ColonColon));
    }

    #[test]
    fn test_pipe() {
        let tokens = tokenize("|").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Pipe));
    }

    #[test]
    fn test_dot() {
        let tokens = tokenize(".").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
    }

    #[test]
    fn test_dot_dot() {
        let tokens = tokenize("..").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::DotDot));
    }

    #[test]
    fn test_dot_dot_dot() {
        let tokens = tokenize("...").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::DotDotDot));
    }

    #[test]
    fn test_all_delimiters() {
        let source = "()[]{}";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 7);
        assert!(matches!(tokens[0].kind, TokenKind::LParen));
        assert!(matches!(tokens[1].kind, TokenKind::RParen));
        assert!(matches!(tokens[2].kind, TokenKind::LBracket));
        assert!(matches!(tokens[3].kind, TokenKind::RBracket));
        assert!(matches!(tokens[4].kind, TokenKind::LBrace));
        assert!(matches!(tokens[5].kind, TokenKind::RBrace));
    }

    #[test]
    fn test_colon_variants() {
        let tokens = tokenize(": ::").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Colon));
        assert!(matches!(tokens[1].kind, TokenKind::ColonColon));
    }

    #[test]
    fn test_dot_variants() {
        let tokens = tokenize(". .. ...").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
        assert!(matches!(tokens[1].kind, TokenKind::DotDot));
        assert!(matches!(tokens[2].kind, TokenKind::DotDotDot));
    }

    #[test]
    fn test_leading_dot_only() {
        let tokens = tokenize(".").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
    }

    #[test]
    fn test_dot_dot_at_end() {
        let tokens = tokenize("..").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::DotDot));
    }
}
