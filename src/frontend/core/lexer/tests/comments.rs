//! 注释测试

use crate::frontend::core::lexer::{tokenize, LexError, TokenKind};

#[cfg(test)]
mod lexer_comments_tests {
    use super::*;

    #[test]
    fn test_single_line_comment_skips_content() {
        let tokens = tokenize("42 // comment\n99").unwrap();
        let has_42 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        let has_99 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(99)));
        assert!(has_42, "Should have integer 42");
        assert!(has_99, "Should have integer 99");
    }

    #[test]
    fn test_multi_line_comment_skips_content() {
        let tokens = tokenize("42 /* comment */ 99").unwrap();
        let has_42 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        let has_99 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(99)));
        assert!(has_42, "Should have integer 42");
        assert!(has_99, "Should have integer 99");
    }

    #[test]
    fn test_multi_line_comment_with_text_after() {
        let tokens = tokenize("/* comment */ let").unwrap();
        let has_let = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(_s)));
        assert!(has_let, "Should have identifier 'let'");
    }

    #[test]
    fn test_comment_with_asterisks() {
        let tokens = tokenize("42 /* ** */ 99").unwrap();
        let has_42 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        let has_99 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(99)));
        assert!(has_42, "Should have integer 42");
        assert!(has_99, "Should have integer 99");
    }

    #[test]
    fn test_nested_comments() {
        let tokens = tokenize("/* outer /* inner */ outer2 */ 42").unwrap();
        let has_42 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        assert!(has_42, "Should have integer 42 after nested comments");
    }

    #[test]
    fn test_deeply_nested_comments() {
        let tokens = tokenize("/* a /* b /* c */ b2 */ a2 */ 42").unwrap();
        let has_42 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        assert!(
            has_42,
            "Should have integer 42 after deeply nested comments"
        );
    }

    #[test]
    fn test_comment_at_start_of_file() {
        let tokens = tokenize("/* comment at start */ let").unwrap();
        let has_let = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(_s)));
        assert!(
            has_let,
            "Should have identifier 'let' after comment at start"
        );
    }

    #[test]
    fn test_multiple_single_line_comments() {
        let tokens = tokenize("// first comment\n// second comment\n42").unwrap();
        let has_42 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        assert!(
            has_42,
            "Should have integer 42 after multiple single-line comments"
        );
    }

    #[test]
    fn test_comment_before_and_after_code() {
        let tokens = tokenize("/* before */ 42 /* after */").unwrap();
        let has_42 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        assert!(has_42, "Should have integer 42 with comments on both sides");
    }

    #[test]
    fn test_comment_between_tokens() {
        let tokens = tokenize("1 /* comment */ + 2").unwrap();
        assert!(tokens.len() >= 4);
    }

    #[test]
    fn test_nested_comments() {
        let tokens = tokenize("1 /* outer /* inner */ */ + 2").unwrap();
        assert!(tokens.len() >= 4);
    }

    #[test]
    fn test_comment_at_file_start() {
        let tokens = tokenize("// comment\n1").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(1)));
    }

    #[test]
    fn test_comment_at_file_end() {
        let tokens = tokenize("1 // comment").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(1)));
    }

    #[test]
    fn test_only_comment() {
        let tokens = tokenize("// this is a comment").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_only_multi_line_comment() {
        let tokens = tokenize("/* comment */").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }
}
