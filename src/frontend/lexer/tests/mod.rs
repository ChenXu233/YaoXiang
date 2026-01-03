//! Lexer 单元测试
//!
//! 测试词法分析器的分词功能

use crate::frontend::lexer::{tokenize, LexError, TokenKind};
use crate::util::span::Span;

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
        assert!(matches!(tokens[0].kind, TokenKind::Underscore));
    }
}

#[cfg(test)]
mod lexer_keywords_tests {
    use super::*;

    #[test]
    fn test_type_keyword() {
        let tokens = tokenize("type").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwType));
    }

    #[test]
    fn test_pub_keyword() {
        let tokens = tokenize("pub").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwPub));
    }

    #[test]
    fn test_use_keyword() {
        let tokens = tokenize("use").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwUse));
    }

    #[test]
    fn test_if_else_keywords() {
        let tokens = tokenize("if else").unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].kind, TokenKind::KwIf));
        assert!(matches!(tokens[1].kind, TokenKind::KwElse));
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
    fn test_return_keyword() {
        let tokens = tokenize("return").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwReturn));
    }

    #[test]
    fn test_all_keywords() {
        let source = "type pub use spawn ref mut if elif else match while for in return break continue as";
        let tokens = tokenize(source).unwrap();
        // Each keyword + EOF = 17 + 1 = 18
        assert_eq!(tokens.len(), 18);
    }
}

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
}

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
        assert!(matches!(tokens[0].kind, TokenKind::DotDot));
    }

    #[test]
    fn test_dot_dot_dot() {
        let tokens = tokenize("...").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::DotDotDot));
    }
}

#[cfg(test)]
mod lexer_literals_tests {
    use super::*;

    #[test]
    fn test_integer_literal() {
        let tokens = tokenize("42").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::IntLiteral(n) = &tokens[0].kind {
            assert_eq!(*n, 42);
        } else {
            panic!("Expected int literal");
        }
    }

    #[test]
    fn test_negative_integer() {
        let tokens = tokenize("-42").unwrap();
        // -42 is tokenized as Minus followed by IntLiteral(42)
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].kind, TokenKind::Minus));
        if let TokenKind::IntLiteral(n) = &tokens[1].kind {
            assert_eq!(*n, 42);
        }
    }

    #[test]
    fn test_float_literal() {
        let tokens = tokenize("3.14").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 3.14).abs() < 0.001);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_string_literal() {
        let tokens = tokenize(r#""hello world""#).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello world");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_string_with_escape() {
        let tokens = tokenize(r#""hello\nworld""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello\nworld");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_character_literal() {
        let tokens = tokenize("'a'").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, 'a');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_escaped_char() {
        let tokens = tokenize(r"'\\'").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, '\\');
        } else {
            panic!("Expected char literal");
        }
    }
}

#[cfg(test)]
mod lexer_comments_tests {
    use super::*;

    #[test]
    fn test_single_line_comment_skips_content() {
        // Comments should be skipped
        let tokens = tokenize("42 // comment\n99").unwrap();
        let has_42 = tokens.iter().any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        let has_99 = tokens.iter().any(|t| matches!(&t.kind, TokenKind::IntLiteral(99)));
        assert!(has_42, "Should have integer 42");
        assert!(has_99, "Should have integer 99");
    }

    #[test]
    fn test_multi_line_comment_skips_content() {
        // Multi-line comments should be skipped
        let tokens = tokenize("42 /* comment */ 99").unwrap();
        let has_42 = tokens.iter().any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        let has_99 = tokens.iter().any(|t| matches!(&t.kind, TokenKind::IntLiteral(99)));
        assert!(has_42, "Should have integer 42");
        assert!(has_99, "Should have integer 99");
    }

    #[test]
    fn test_multi_line_comment_with_text_after() {
        // Test that text after multi-line comment is parsed correctly
        let tokens = tokenize("/* comment */ let").unwrap();
        // Should have identifier "let" and EOF
        let has_let = tokens.iter().any(|t| matches!(&t.kind, TokenKind::Identifier(s)));
        assert!(has_let, "Should have identifier 'let'");
    }

    #[test]
    fn test_comment_with_asterisks() {
        // Test comment with multiple asterisks
        let tokens = tokenize("42 /* ** */ 99").unwrap();
        let has_42 = tokens.iter().any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        let has_99 = tokens.iter().any(|t| matches!(&t.kind, TokenKind::IntLiteral(99)));
        assert!(has_42, "Should have integer 42");
        assert!(has_99, "Should have integer 99");
    }
}

#[cfg(test)]
mod lexer_error_tests {
    use super::*;

    #[test]
    fn test_unterminated_string() {
        let result = tokenize("\"hello");
        assert!(result.is_err());
        if let Err(LexError::UnterminatedString { .. }) = result {
            // Expected error
        } else {
            panic!("Expected unterminated string error");
        }
    }

    #[test]
    fn test_unterminated_char() {
        let result = tokenize("'a");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_escape() {
        let result = tokenize(r#""\q""#);
        assert!(result.is_err());
        if let Err(LexError::InvalidEscape { .. }) = result {
            // Expected error
        } else {
            panic!("Expected invalid escape error");
        }
    }

    #[test]
    fn test_unknown_char() {
        let result = tokenize("@");
        assert!(result.is_err());
        if let Err(LexError::UnexpectedChar { .. }) = result {
            // Expected error
        } else {
            panic!("Expected unexpected char error");
        }
    }
}
