//! 运算符测试

use crate::frontend::core::lexer::{tokenize, LexError, TokenKind};

#[cfg(test)]
mod lexer_operators_tests {
    use super::*;

    #[test]
    fn test_plus_operator() {
        let tokens = tokenize("+").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Plus));
    }

    #[test]
    fn test_minus_operator() {
        let tokens = tokenize("-").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Minus));
    }

    #[test]
    fn test_star_operator() {
        let tokens = tokenize("*").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Star));
    }

    #[test]
    fn test_slash_operator() {
        let tokens = tokenize("/").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Slash));
    }

    #[test]
    fn test_percent_operator() {
        let tokens = tokenize("%").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Percent));
    }

    #[test]
    fn test_eq_eq_operator() {
        let tokens = tokenize("==").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::EqEq));
    }

    #[test]
    fn test_neq_operator() {
        let tokens = tokenize("!=").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Neq));
    }

    #[test]
    fn test_lt_operator() {
        let tokens = tokenize("<").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Lt));
    }

    #[test]
    fn test_le_operator() {
        let tokens = tokenize("<=").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Le));
    }

    #[test]
    fn test_gt_operator() {
        let tokens = tokenize(">").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Gt));
    }

    #[test]
    fn test_ge_operator() {
        let tokens = tokenize(">=").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Ge));
    }

    #[test]
    fn test_and_operator() {
        let tokens = tokenize("&&").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::And));
    }

    #[test]
    fn test_or_operator() {
        let tokens = tokenize("||").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Or));
    }

    #[test]
    fn test_not_operator() {
        let tokens = tokenize("!").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Not));
    }

    #[test]
    fn test_arrow_operator() {
        let tokens = tokenize("->").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Arrow));
    }

    #[test]
    fn test_fat_arrow_operator() {
        let tokens = tokenize("=>").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::FatArrow));
    }

    #[test]
    fn test_all_single_char_operators() {
        let source = "+-*/%";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 6);
        assert!(matches!(tokens[0].kind, TokenKind::Plus));
        assert!(matches!(tokens[1].kind, TokenKind::Minus));
        assert!(matches!(tokens[2].kind, TokenKind::Star));
        assert!(matches!(tokens[3].kind, TokenKind::Slash));
        assert!(matches!(tokens[4].kind, TokenKind::Percent));
    }

    #[test]
    fn test_comparison_operators() {
        let source = "< <= > >= == !=";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 7);
    }

    #[test]
    fn test_logical_operators() {
        let source = "! && ||";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 4);
    }

    #[test]
    fn test_mixed_operators() {
        let source = "+ - * / % = == != < <= > >= ! && ||";
        let tokens = tokenize(source).unwrap();
        assert!(tokens.len() > 15);
    }

    #[test]
    fn test_arrow_operator_usage() {
        let tokens = tokenize("->").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Arrow));
    }

    #[test]
    fn test_fat_arrow_usage() {
        let tokens = tokenize("=>").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FatArrow));
    }
}
