//! Lexer 单元测试
//!
//! 测试词法分析器的分词功能
#![allow(unused_imports)]
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
    fn test_spawn_keyword() {
        let tokens = tokenize("spawn").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwSpawn));
    }

    #[test]
    fn test_ref_keyword() {
        let tokens = tokenize("ref").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwRef));
    }

    #[test]
    fn test_mut_keyword() {
        let tokens = tokenize("mut").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::KwMut));
    }

    #[test]
    fn test_if_else_keywords() {
        let tokens = tokenize("if else").unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].kind, TokenKind::KwIf));
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

    #[test]
    fn test_hex_literal() {
        let tokens = tokenize("0xFF").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::IntLiteral(n) = &tokens[0].kind {
            assert_eq!(*n, 255);
        } else {
            panic!("Expected int literal");
        }
    }

    #[test]
    fn test_hex_literal_uppercase() {
        let tokens = tokenize("0XDEADBEEF").unwrap();
        if let TokenKind::IntLiteral(n) = &tokens[0].kind {
            assert_eq!(*n, 0xDEADBEEF);
        } else {
            panic!("Expected int literal");
        }
    }

    #[test]
    fn test_hex_literal_with_underscore() {
        let tokens = tokenize("0xDEAD_BEEF").unwrap();
        if let TokenKind::IntLiteral(n) = &tokens[0].kind {
            assert_eq!(*n, 0xDEADBEEF);
        } else {
            panic!("Expected int literal");
        }
    }

    #[test]
    fn test_octal_literal() {
        let tokens = tokenize("0o755").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::IntLiteral(n) = &tokens[0].kind {
            assert_eq!(*n, 493);
        } else {
            panic!("Expected int literal");
        }
    }

    #[test]
    fn test_octal_literal_with_underscore() {
        let tokens = tokenize("0o123_456").unwrap();
        if let TokenKind::IntLiteral(n) = &tokens[0].kind {
            assert_eq!(*n, 0o123456);
        } else {
            panic!("Expected int literal");
        }
    }

    #[test]
    fn test_binary_literal() {
        let tokens = tokenize("0b1010").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::IntLiteral(n) = &tokens[0].kind {
            assert_eq!(*n, 10);
        } else {
            panic!("Expected int literal");
        }
    }

    #[test]
    fn test_binary_literal_with_underscore() {
        let tokens = tokenize("0b1111_0000").unwrap();
        if let TokenKind::IntLiteral(n) = &tokens[0].kind {
            assert_eq!(*n, 0b11110000);
        } else {
            panic!("Expected int literal");
        }
    }

    #[test]
    fn test_void_literal() {
        let tokens = tokenize("void").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::VoidLiteral));
    }

    #[test]
    fn test_string_with_hex_escape() {
        let tokens = tokenize(r#""\x41""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "A");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_string_with_unicode_escape() {
        let tokens = tokenize(r#""\u{1F600}""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "😀");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_char_with_hex_escape() {
        let tokens = tokenize(r"'\x41'").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, 'A');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_char_with_unicode_escape() {
        let tokens = tokenize(r"'\u{1F600}'").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, '😀');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_float_leading_dot() {
        // Test .5 format (equivalent to 0.5)
        let tokens = tokenize(".5").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 0.5).abs() < 0.001);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_float_scientific_notation() {
        let tokens = tokenize("1e10").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 1e10).abs() < 1.0);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_float_scientific_notation_negative_exp() {
        let tokens = tokenize("1.5e-5").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 0.000015).abs() < 0.000001);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_float_scientific_notation_uppercase() {
        let tokens = tokenize("3.14E10").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 3.14e10).abs() < 1.0);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_float_scientific_notation_with_plus() {
        let tokens = tokenize("2e+5").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 200000.0).abs() < 1.0);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_integer_with_underscores() {
        let tokens = tokenize("1_000_000").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::IntLiteral(n) = &tokens[0].kind {
            assert_eq!(*n, 1000000);
        } else {
            panic!("Expected int literal");
        }
    }

    #[test]
    fn test_float_with_underscores() {
        let tokens = tokenize("1_000.5").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 1000.5).abs() < 0.001);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_empty_string() {
        let tokens = tokenize(r#""""#).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_string_with_tab_escape() {
        let tokens = tokenize(r#""hello\tworld""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello\tworld");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_string_with_cr_escape() {
        let tokens = tokenize(r#""hello\rworld""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello\rworld");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_string_with_null_escape() {
        let tokens = tokenize(r#""hello\0world""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello\0world");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_string_with_quote_escape() {
        let tokens = tokenize(r#""hello\"world""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello\"world");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_char_with_tab_escape() {
        let tokens = tokenize(r"'\t'").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, '\t');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_char_with_cr_escape() {
        let tokens = tokenize(r"'\r'").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, '\r');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_char_with_null_escape() {
        let tokens = tokenize(r"'\0'").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, '\0');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_char_with_quote_escape() {
        let tokens = tokenize(r"'\''").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, '\'');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_char_with_double_quote_escape() {
        let tokens = tokenize(r"'\x22'").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, '"');
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
        // Multi-line comments should be skipped
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
        // Test that text after multi-line comment is parsed correctly
        let tokens = tokenize("/* comment */ let").unwrap();
        // Should have identifier "let" and EOF
        let has_let = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(_s)));
        assert!(has_let, "Should have identifier 'let'");
    }

    #[test]
    fn test_comment_with_asterisks() {
        // Test comment with multiple asterisks
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
        // Test nested multi-line comments
        let tokens = tokenize("/* outer /* inner */ outer2 */ 42").unwrap();
        let has_42 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        assert!(has_42, "Should have integer 42 after nested comments");
    }

    #[test]
    fn test_deeply_nested_comments() {
        // Test deeply nested comments
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
        // Test comment at the very start
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
        // Test multiple single-line comments
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
        // Test comments on both sides of code
        let tokens = tokenize("/* before */ 42 /* after */").unwrap();
        let has_42 = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::IntLiteral(42)));
        assert!(has_42, "Should have integer 42 with comments on both sides");
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

    #[test]
    fn test_standalone_ampersand_error() {
        // Single & without second & should be an error
        let result = tokenize("&");
        assert!(result.is_err());
        if let Err(LexError::UnexpectedChar { .. }) = result {
            // Expected error
        } else {
            panic!("Expected unexpected char error for &");
        }
    }

    #[test]
    fn test_invalid_hex_escape() {
        // \x with only one hex digit
        let result = tokenize(r#""\x4""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_number_no_digits() {
        // 0x without any hex digits
        let result = tokenize("0x");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_octal_number_no_digits() {
        // 0o without any octal digits
        let result = tokenize("0o");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_binary_number_no_digits() {
        // 0b without any binary digits
        let result = tokenize("0b");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_char_literal() {
        let result = tokenize("''");
        assert!(result.is_err());
    }

    #[test]
    fn test_unterminated_multi_line_comment() {
        // Unterminated multi-line comments are skipped by lexer
        // (Parser will catch unmatched /* later)
        let tokens = tokenize("/* unterminated comment").unwrap();
        // Should have EOF token (comment is skipped)
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_invalid_unicode_escape() {
        // \u without braces
        let result = tokenize(r#""\u1234""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_unicode_escape_empty_braces() {
        // \u{} without hex digits
        let result = tokenize(r#""\u{}""#);
        assert!(result.is_err());
    }
}
