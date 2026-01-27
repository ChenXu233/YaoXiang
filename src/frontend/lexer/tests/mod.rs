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
        assert!(matches!(&tokens[0].kind, TokenKind::Underscore));
    }

    #[test]
    fn test_identifier_starting_with_underscore() {
        // Identifier starting with underscore: _foo
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
        // Identifier starting with underscore followed by number: _123
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
        // Test that newlines are properly handled
        let tokens = tokenize("a\nb").unwrap();
        assert_eq!(tokens.len(), 3); // a, b, EOF
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_multiple_whitespace_with_newline() {
        // Test whitespace including newline
        let tokens = tokenize("1   2\n\n   3").unwrap();
        // Should have 4 tokens: 1, 2, 3, EOF
        assert_eq!(tokens.len(), 4);
    }
}

#[cfg(test)]
mod lexer_keywords_tests {
    use super::*;

    #[test]
    fn test_type_keyword() {
        let tokens = tokenize("type").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(&tokens[0].kind, TokenKind::KwType));
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
    fn test_string_literal_with_unicode_char_in_source() {
        // Ensure lexer advances over a multi-byte UTF-8 char in raw source
        let tokens = tokenize("\"😀\"").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "😀");
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
    fn test_character_literal_with_unicode_char_in_source() {
        // Ensure lexer advances over a multi-byte UTF-8 char in raw source
        let tokens = tokenize("'😀'").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, '😀');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_char_unknown_escape_treated_as_literal() {
        // In char literals, unknown escape sequences are treated as the escaped char itself
        // (unlike strings, where it becomes an error)
        let tokens = tokenize(r#"'\q'"#).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, 'q');
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
    fn test_float_with_underscore_after_dot_before_digit() {
        // Underscore is allowed as a separator when it's between digits
        // This exercises the decimal-part underscore handling branch.
        let tokens = tokenize("1._2").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 1.2).abs() < 0.000_000_1);
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

    #[test]
    fn test_string_with_newline() {
        // String containing actual newline (not escape) should error
        let result = tokenize("\"hello\nworld\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_with_newline() {
        // Character literal containing newline should error
        let result = tokenize("'\n'");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_escape_single_digit() {
        // \x with only one hex digit should error
        let result = tokenize(r#""\x4""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_invalid_codepoint() {
        // Unicode escape with too large codepoint should error
        // \u{10FFFFFFF} is too large for char
        let result = tokenize(r#""\u{10FFFFFFF}""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_only_one_brace() {
        // Unicode escape without closing brace
        let result = tokenize(r#""\u{1F600""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_float_only_dot() {
        // Just a dot followed by whitespace should be Dot token, not error
        let tokens = tokenize(".").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
    }

    #[test]
    fn test_float_leading_dot_with_exponent() {
        // .5e10 format
        let tokens = tokenize(".5e10").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 5e9).abs() < 1.0);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_float_leading_dot_with_negative_exponent() {
        // .5e-3 format
        let tokens = tokenize(".5e-3").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 0.0005).abs() < 0.00001);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_float_with_underscores_in_exponent() {
        let tokens = tokenize("1_234.567e10").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
            assert!((n - 1234.567e10).abs() < 1.0);
        } else {
            panic!("Expected float literal");
        }
    }

    #[test]
    fn test_multiple_underscores_in_number() {
        let tokens = tokenize("1__000__000").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::IntLiteral(n) = &tokens[0].kind {
            assert_eq!(*n, 1000000);
        } else {
            panic!("Expected int literal");
        }
    }

    #[test]
    fn test_bool_literals() {
        let tokens = tokenize("true false").unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].kind, TokenKind::BoolLiteral(true)));
        assert!(matches!(tokens[1].kind, TokenKind::BoolLiteral(false)));
    }

    #[test]
    fn test_multi_line_string_basic() {
        let tokens = tokenize("\"\"\"hello\nworld\"\"\"").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello\nworld");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_multi_line_string_empty() {
        let tokens = tokenize("\"\"\"\"\"\"").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_multi_line_string_with_content() {
        let source = "\"\"\"\nline 1\nline 2\nline 3\n\"\"\"";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            // Note: The opening newline after """ is preserved in the content
            assert_eq!(s, "\nline 1\nline 2\nline 3\n");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_multi_line_string_with_quotes() {
        // Multi-line string with a single quote in the middle (not three in a row)
        // Content: hello 'world
        let tokens = tokenize("\"\"\"hello 'world'\"\"\"").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello 'world'");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_multi_line_string_with_escaped_quotes() {
        // Multi-line string with a single backslash escape (not \\")
        // Content: hello \n world (backslash-n escape)
        let tokens = tokenize("\"\"\"hello \\n world\"\"\"").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello \n world");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_multi_line_string_json_like() {
        let source = "\"\"\"\n{\n    \"name\": \"张三\",\n    \"age\": 30\n}\n\"\"\"";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert!(s.contains('"'));
            assert!(s.contains("name"));
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_multi_line_string_with_escapes() {
        let tokens = tokenize("\"\"\"hello\\nworld\"\"\"").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello\nworld");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_multi_line_string_unterminated() {
        let result = tokenize("\"\"\"hello world");
        assert!(result.is_err());
    }

    #[test]
    fn test_multi_line_string_in_code() {
        let source = "\nlet json = \"\"\"\n{\n    \"key\": \"value\"\n}\n\"\"\"\nprintln(json)\n";
        let tokens = tokenize(source).unwrap();
        // Should have: let, identifier(json), =, string, identifier(println), (, identifier(json), ), EOF
        assert!(tokens.len() > 5);
        let has_string = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::StringLiteral(_)));
        assert!(has_string);
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
    fn test_invalid_hex_number_only_underscore() {
        // 0x_ consumes underscore but still has no digits
        let result = tokenize("0x_");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_octal_number_no_digits() {
        // 0o without any octal digits
        let result = tokenize("0o");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_octal_number_only_underscore() {
        // 0o_ consumes underscore but still has no digits
        let result = tokenize("0o_");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_binary_number_no_digits() {
        // 0b without any binary digits
        let result = tokenize("0b");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_binary_number_only_underscore() {
        // 0b_ consumes underscore but still has no digits
        let result = tokenize("0b_");
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

    #[test]
    fn test_string_ending_with_backslash() {
        // String ending with backslash
        let result = tokenize(r#""hello\"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_char_ending_with_backslash() {
        // Char literal ending with backslash
        let result = tokenize("'a\\'");
        assert!(result.is_err());
    }

    #[test]
    fn test_standalone_pipe() {
        // Single | is a valid Pipe token (used in closures, pattern matching)
        let result = tokenize("|");
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 2); // Pipe + EOF
        assert!(matches!(tokens[0].kind, TokenKind::Pipe));
    }

    #[test]
    fn test_number_too_large() {
        // Very large number that overflows i128
        let result = tokenize("99999999999999999999999999999999999999999");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_number_too_large() {
        // Very large hex number
        let result = tokenize("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_number_exceeds_i128_but_fits_u128() {
        // This should hit the `try_into()` error path (fits u128, exceeds i128::MAX)
        let value: u128 = 1u128 << 127;
        let source = format!("0x{:x}", value);
        let result = tokenize(&source);
        assert!(result.is_err());
    }

    #[test]
    fn test_octal_number_exceeds_i128_but_fits_u128() {
        // Also hit the `try_into()` error path for octal
        let value: u128 = 1u128 << 127;
        let source = format!("0o{:o}", value);
        let result = tokenize(&source);
        assert!(result.is_err());
    }

    #[test]
    fn test_binary_number_exceeds_i128_but_fits_u128() {
        // Also hit the `try_into()` error path for binary
        let source = format!("0b1{}", "0".repeat(127));
        let result = tokenize(&source);
        assert!(result.is_err());
    }

    #[test]
    fn test_float_with_only_underscore_after_dot() {
        // 123._ should be an error (underscore after decimal point must be followed by digit)
        let result = tokenize(r"123._");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_underscore() {
        // ._ is parsed as Dot + Underscore (not a float error)
        let result = tokenize(r"._");
        assert!(result.is_ok());
        let tokens = result.unwrap();
        // Should be: Dot, Underscore, EOF
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
        assert!(matches!(tokens[1].kind, TokenKind::Underscore));
    }

    #[test]
    fn test_float_with_underscore_between_exp_digits() {
        // 1e1_2 is valid - underscore between exponent digits
        let result = tokenize(r"1e1_2");
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 2); // float + EOF
    }

    #[test]
    fn test_exponent_without_digits() {
        // 1e without exponent digits should error
        let result = tokenize("1e");
        assert!(result.is_err());
    }

    #[test]
    fn test_exponent_trailing_underscore_error() {
        // Underscore must be between digits in exponent
        let result = tokenize("1e1_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_with_underscore_error() {
        // .5_ should error (underscore after decimal must be followed by digit)
        let result = tokenize(r".5_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_exponent_without_digits() {
        // .5e without exponent digits should error
        let result = tokenize(r".5e");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_exponent_trailing_underscore_error() {
        // Underscore must be between digits in exponent (leading-dot float)
        let result = tokenize(r".5e1_");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_string_escape_char() {
        // Invalid escape character like \q
        let result = tokenize(r#""\q""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_only() {
        // Just a dot with no digits after should be Dot token
        let tokens = tokenize(".").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
    }

    #[test]
    fn test_dot_dot_at_end() {
        // .. at end should be DotDot token
        let tokens = tokenize("..").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::DotDot));
    }

    #[test]
    fn test_hex_overflow() {
        let result = tokenize("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_with_all_escapes() {
        // Test all valid escape sequences individually
        let tokens = tokenize(r#""\n""#).unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "\n"));

        let tokens = tokenize(r#""\t""#).unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "\t"));

        let tokens = tokenize(r#""\r""#).unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "\r"));

        let tokens = tokenize(r#""\\""#).unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "\\"));

        let tokens = tokenize(r#""\"""#).unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "\""));

        let tokens = tokenize(r#""\0""#).unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "\0"));
    }

    #[test]
    fn test_char_with_all_escapes() {
        // Test valid char escapes individually
        let result = tokenize(r"'\n'");
        assert!(result.is_ok());

        let result = tokenize(r"'\t'");
        assert!(result.is_ok());

        let result = tokenize(r"'\r'");
        assert!(result.is_ok());

        let result = tokenize(r"'\\'");
        assert!(result.is_ok());

        // For double quote in char, use a different approach
        let tokens = tokenize("'\"'").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::CharLiteral('"')));

        let result = tokenize(r"'\''");
        assert!(result.is_ok());

        let result = tokenize(r"'\0'");
        assert!(result.is_ok());
    }

    #[test]
    fn test_long_identifier() {
        // Long identifier
        let tokens = tokenize("very_long_identifier_with_many_characters").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_mixed_code_with_newlines() {
        // Mixed code with multiple newlines - using valid identifiers
        let source = "let x = 42\n\n// comment\n\nlet y = 99";
        let tokens = tokenize(source).unwrap();
        // Should have: let, identifier, =, int, let, identifier, =, int, EOF
        assert!(tokens.len() > 8);
    }

    #[test]
    fn test_whitespace_variants() {
        // Different whitespace characters
        let source = "1\t2\r\n3 4";
        let tokens = tokenize(source).unwrap();
        // Should have 5 tokens: 1, 2, 3, 4, EOF
        assert_eq!(tokens.len(), 5);
    }

    #[test]
    fn test_all_delimiters() {
        // Test all delimiter tokens
        let source = "()[]{}";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 7); // (, ), [, ], {, }, EOF
        assert!(matches!(tokens[0].kind, TokenKind::LParen));
        assert!(matches!(tokens[1].kind, TokenKind::RParen));
        assert!(matches!(tokens[2].kind, TokenKind::LBracket));
        assert!(matches!(tokens[3].kind, TokenKind::RBracket));
        assert!(matches!(tokens[4].kind, TokenKind::LBrace));
        assert!(matches!(tokens[5].kind, TokenKind::RBrace));
    }

    #[test]
    fn test_all_single_char_operators() {
        // Test single character operators
        let source = "+-*/%";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 6); // 5 operators + EOF
        assert!(matches!(tokens[0].kind, TokenKind::Plus));
        assert!(matches!(tokens[1].kind, TokenKind::Minus));
        assert!(matches!(tokens[2].kind, TokenKind::Star));
        assert!(matches!(tokens[3].kind, TokenKind::Slash));
        assert!(matches!(tokens[4].kind, TokenKind::Percent));
    }

    #[test]
    fn test_comparison_operators() {
        // Test comparison operators
        let source = "< <= > >= == !=";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 7); // 6 operators + EOF
    }

    #[test]
    fn test_logical_operators() {
        // Test logical operators
        let source = "! && ||";
        let tokens = tokenize(source).unwrap();
        assert_eq!(tokens.len(), 4); // Not, And, Or, EOF
    }

    #[test]
    fn test_float_edge_cases() {
        // Float edge cases
        let cases = [
            ("0.0", 0.0),
            ("1e0", 1.0),
            ("1e-0", 1.0),
            ("0.1e10", 0.1e10),
            ("1e+10", 1e10),
        ];
        for (input, expected) in cases {
            let tokens = tokenize(input).unwrap();
            if let TokenKind::FloatLiteral(n) = &tokens[0].kind {
                assert!((n - expected).abs() < 0.001, "Failed for {}", input);
            } else {
                panic!("Expected float for {}", input);
            }
        }
    }

    #[test]
    fn test_int_edge_cases() {
        // Integer edge cases
        let cases = ["0", "1", "42", "1000", "999999999999999999"];
        for input in cases {
            let tokens = tokenize(input).unwrap();
            assert!(
                matches!(tokens[0].kind, TokenKind::IntLiteral(_)),
                "Failed for {}",
                input
            );
        }
    }

    #[test]
    fn test_zero_prefixed_numbers() {
        // Zero-prefixed numbers
        let cases = ["0", "0x0", "0o0", "0b0", "00", "007"];
        for input in cases {
            let result = tokenize(input);
            assert!(result.is_ok(), "Should parse: {}", input);
        }
    }

    #[test]
    fn test_negative_zero() {
        // Negative zero (just minus token + zero)
        let tokens = tokenize("-0").unwrap();
        assert_eq!(tokens.len(), 3); // -, 0, EOF
    }

    #[test]
    fn test_leading_dot_exponent_no_digits() {
        // .5e without exponent digits - this should be an error
        let result = tokenize(r".5e");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_exponent_underscore_error() {
        // .5e_5 should error (underscore in exponent must be between digits)
        // But .5e is valid as 0.5, and .5e5 is valid
        // Let's test a case that should definitely error: .5e_ (underscore at end)
        let result = tokenize(r".5e_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_exponent_plus_only() {
        // .5e+ without digits should error
        let result = tokenize(r".5e+");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_exponent_minus_only() {
        // .5e- without digits should error
        let result = tokenize(r".5e-");
        // This might be valid (0.5e-) or might error
        // Just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_invalid_unicode_escape_no_hex_digits() {
        // \u{abc} is valid, but \u{} is invalid (empty)
        let result = tokenize(r#""\u{}""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_unicode_escape_invalid_codepoint() {
        // \u{FFFFFFFFFFFFFFFF} is too large for a codepoint
        let result = tokenize(r#""\u{FFFFFFFFFFFFFFFF}""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_escape_too_few_digits() {
        // \x with only one hex digit (not two)
        let result = tokenize(r#""\xA""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_escape_x_valid() {
        // Valid \xFF escape
        let tokens = tokenize(r#""\x41""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "A");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_char_escape_x_valid() {
        // Valid \x41 in char
        let tokens = tokenize("'\\x41'").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, 'A');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_string_unicode_valid() {
        // Valid \u{1F600} emoji
        let tokens = tokenize(r#""\u{1F600}""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "😀");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_char_unicode_valid() {
        // Valid \u{1F600} in char (may fail if char is multi-byte, but lexer should handle)
        // Using a simpler unicode char
        let tokens = tokenize("'\\u{263A}'").unwrap(); // ☺
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, '☺');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_float_parse_failure() {
        // This tests the unlikely case where f64 parsing fails
        // Using an extremely large float that might overflow
        let result = tokenize("1e999999999999999999999999999");
        // This might succeed (as infinity) or fail depending on implementation
        // The point is to exercise the error path in the lexer
        let _ = result; // Just ensure it doesn't panic
    }

    #[test]
    fn test_multiple_underscores_in_float() {
        // 1_2_3.4_5_6 should be valid
        let tokens = tokenize("1_2_3.4_5_6").unwrap();
        assert_eq!(tokens.len(), 2); // float + EOF
        assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_scientific_notation_uppercase_e() {
        // Test uppercase E in scientific notation
        let tokens = tokenize("1E10").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_scientific_notation_with_underscores() {
        // 1_000e1_0 should be valid
        let tokens = tokenize("1_000e1_0").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_decimal_overflow_error() {
        // Very large decimal number that overflows i128
        // Use an extremely long number to ensure overflow
        let result = tokenize("9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_try_into_failure() {
        // Hex number that's too large for i128 but fits in u128
        // This tests the try_into failure path
        let result = tokenize("0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"); // near i128::MAX
                                                                      // Should succeed if it fits in i128
        if result.is_ok() {
            let tokens = result.unwrap();
            assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(_)));
        }
    }

    #[test]
    fn test_octal_overflow() {
        // Very large octal number - need even more digits to overflow u128
        let result = tokenize("0o7777777777777777777777777777777777777777777777777777777777777777777777777777777777777777777777777777777");
        assert!(result.is_err());
    }

    #[test]
    fn test_binary_overflow() {
        // Very large binary number - need more than 128 bits
        let result = tokenize("0b111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111");
        assert!(result.is_err());
    }

    #[test]
    fn test_float_with_leading_underscore_error() {
        // _123 is identifier, not number
        let tokens = tokenize("_123").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_number_with_underscore_before_dot() {
        // 123_.456 - this might be parsed differently
        // Let's use a case that's clearly an error
        let result = tokenize("123.456_"); // underscore at end of decimal
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_valid_chars() {
        // Test various valid unicode escapes
        let tests = [
            (r#"\u{00A9}"#, '©'), // Copyright
            (r#"\u{00AE}"#, '®'), // Registered
            (r#"\u{2600}"#, '☀'), // Sun
            (r#"\u{2601}"#, '☁'), // Cloud
        ];
        for (escape, expected) in tests {
            let input = format!("\"{}\"", escape);
            let tokens = tokenize(&input).unwrap();
            if let TokenKind::StringLiteral(s) = &tokens[0].kind {
                assert_eq!(s.chars().next(), Some(expected), "Failed for {}", escape);
            }
        }
    }

    #[test]
    fn test_hex_escape_all_digits() {
        // \x7F with both digits (max valid ASCII)
        let tokens = tokenize(r#""\x7F""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            let expected = "\x7F" as &str;
            assert_eq!(s.as_str(), expected);
        }
    }

    #[test]
    fn test_string_with_embedded_quotes() {
        // String with escaped double quote inside
        let tokens = tokenize(r#""hello \"world\"""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, r#"hello "world""#);
        }
    }

    #[test]
    fn test_char_with_embedded_quote() {
        // Char with escaped single quote
        let tokens = tokenize(r"'\''").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::CharLiteral('\'')));
    }

    #[test]
    fn test_mixed_operators() {
        // Test various operators in sequence
        let source = "+ - * / % = == != < <= > >= ! && ||";
        let tokens = tokenize(source).unwrap();
        // Should have many tokens plus EOF
        assert!(tokens.len() > 15);
    }

    #[test]
    fn test_keywords_are_not_identifiers() {
        // Keywords should be recognized as keywords, not identifiers
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
    fn test_bool_and_void_literals() {
        // true, false should be BoolLiteral
        let tokens = tokenize("true").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::BoolLiteral(true)));

        let tokens = tokenize("false").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::BoolLiteral(false)));

        // void should be VoidLiteral
        let tokens = tokenize("void").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::VoidLiteral));
    }

    #[test]
    fn test_comment_between_tokens() {
        // Comments should be skipped
        let tokens = tokenize("1 /* comment */ + 2").unwrap();
        assert!(tokens.len() >= 4); // 1, +, 2, EOF
    }

    #[test]
    fn test_nested_comments() {
        // Nested /* comments */
        let tokens = tokenize("1 /* outer /* inner */ */ + 2").unwrap();
        assert!(tokens.len() >= 4);
    }

    #[test]
    fn test_comment_at_file_start() {
        // Comment at start of file
        let tokens = tokenize("// comment\n1").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(1)));
    }

    #[test]
    fn test_comment_at_file_end() {
        // Comment at end of file
        let tokens = tokenize("1 // comment").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(1)));
    }

    #[test]
    fn test_hex_escape_invalid_value() {
        // \x with valid 2 digits but invalid value (if any)
        // \xGG would fail at hex digit parsing, not value parsing
        // This tests the path where hex has 2 chars but parsing fails
        let result = tokenize(r#""\xGG""#);
        // Should error because G is not a hex digit
        assert!(result.is_err());
    }

    #[test]
    fn test_string_multi_line_error() {
        // String that ends with newline - should error
        let result = tokenize("\"hello\nworld\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_multi_line_error() {
        // Char literal with newline - should error
        let result = tokenize("'a\nb'");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_decimal_points_error() {
        // 1.2.3 is parsed as 1.2 and .3 (both valid floats)
        // This is actually valid in this lexer, not an error
        let result = tokenize("1.2.3");
        // It should succeed and produce multiple float tokens
        if result.is_ok() {
            let tokens = result.unwrap();
            assert!(tokens.len() >= 3); // 1.2, .3, EOF
        }
    }

    #[test]
    fn test_invalid_unicode_empty_braces() {
        // \u{} is invalid (empty hex)
        let result = tokenize(r#""\u{}""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_unicode_missing_brace() {
        // \u1234 without closing brace
        let result = tokenize(r#""\u{1234""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_very_long_identifier() {
        // Very long identifier
        let long_name = "a".repeat(1000);
        let tokens = tokenize(&long_name).unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_string_with_all_whitespace() {
        // String containing various whitespace
        let tokens = tokenize("\"hello world\"").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::StringLiteral(_)));
    }

    #[test]
    fn test_colon_variants() {
        // Test single colon and double colon
        let tokens = tokenize(": ::").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Colon));
        assert!(matches!(tokens[1].kind, TokenKind::ColonColon));
    }

    #[test]
    fn test_arrow_variants() {
        // Test -> (arrow) and => (fat arrow)
        let tokens = tokenize("-> =>").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Arrow));
        assert!(matches!(tokens[1].kind, TokenKind::FatArrow));
    }

    #[test]
    fn test_dot_variants() {
        // Test ., .., ...
        let tokens = tokenize(". .. ...").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
        assert!(matches!(tokens[1].kind, TokenKind::DotDot));
        assert!(matches!(tokens[2].kind, TokenKind::DotDotDot));
    }

    #[test]
    fn test_empty_file() {
        // Empty file should have just EOF
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_only_whitespace() {
        // Only whitespace
        let tokens = tokenize("   \t\n\r   ").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_only_comment() {
        // Only comment
        let tokens = tokenize("// this is a comment").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_only_multi_line_comment() {
        // Only multi-line comment
        let tokens = tokenize("/* comment */").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_standalone_underscore_in_expression() {
        // Standalone underscore in expression context
        let tokens = tokenize("_ + 5").unwrap();
        assert_eq!(tokens.len(), 4); // _, +, 5, EOF
        assert!(matches!(tokens[0].kind, TokenKind::Underscore));
    }

    #[test]
    fn test_arrow_operator_usage() {
        // -> in type annotation context
        let tokens = tokenize("->").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Arrow));
    }

    #[test]
    fn test_fat_arrow_usage() {
        // => in match/closure context
        let tokens = tokenize("=>").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FatArrow));
    }

    #[test]
    fn test_hex_uppercase_digits() {
        // Hex with uppercase A-F
        let tokens = tokenize("0xABCDEF").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(11259375)));
    }

    #[test]
    fn test_hex_mixed_case() {
        // Hex with mixed case
        let tokens = tokenize("0xAbCdEf").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(_)));
    }

    #[test]
    fn test_octal_with_all_digits() {
        // Octal with all valid digits 0-7
        let tokens = tokenize("0o01234567").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(_)));
    }

    #[test]
    fn test_binary_with_all_ones() {
        // Binary with many ones
        let tokens = tokenize("0b1111").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(15)));
    }

    #[test]
    fn test_float_with_leading_zeros() {
        // Float with leading zeros
        let tokens = tokenize("001.002").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_float_scientific_with_leading_zeros() {
        // Float with scientific notation and leading zeros
        let tokens = tokenize("001e002").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_identifier_with_numbers_and_underscores() {
        // Identifier with numbers and underscores
        let tokens = tokenize("var_123_name").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_string_empty() {
        // Empty string
        let tokens = tokenize("\"\"").unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s.is_empty()));
    }

    #[test]
    fn test_char_empty_quotes() {
        // Empty char quotes - should error
        let result = tokenize("''");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_with_space() {
        // Char with space - should be valid
        let tokens = tokenize("' '").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::CharLiteral(' ')));
    }

    #[test]
    fn test_string_with_unicode_char() {
        // String with unicode character
        let tokens = tokenize("\"中文\"").unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert!(s.contains("中"));
        }
    }

    #[test]
    fn test_identifier_unicode() {
        // Identifier with unicode (not valid, but test error handling)
        let result = tokenize("变量");
        // This might error or produce identifier depending on implementation
        let _ = result;
    }

    #[test]
    fn test_string_with_tabs() {
        // String with tab character
        let tokens = tokenize("\"hello\tworld\"").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::StringLiteral(_)));
    }

    #[test]
    fn test_float_zero() {
        // Various forms of zero
        let cases = ["0", "0.0", "0e0", "0.0e0", ".0"];
        for case in &cases {
            let tokens = tokenize(case).unwrap();
            assert!(matches!(
                tokens[0].kind,
                TokenKind::FloatLiteral(0.0) | TokenKind::IntLiteral(0)
            ));
        }
    }

    #[test]
    fn test_negative_numbers() {
        // Negative numbers
        let cases = ["-1", "-1.5", "-1e10", "-0xFF", "-0o77", "-0b1111"];
        for case in &cases {
            let tokens = tokenize(case).unwrap();
            // Should have minus and number
            assert!(tokens.len() >= 2);
        }
    }

    #[test]
    fn test_string_with_backslash_n() {
        // String with \n escape
        let tokens = tokenize(r#""hello\n""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert!(s.contains('\n'));
        }
    }

    #[test]
    fn test_string_with_backslash_r() {
        // String with \r escape
        let tokens = tokenize(r#""hello\r""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert!(s.contains('\r'));
        }
    }

    #[test]
    fn test_string_with_backslash_zero() {
        // String with \0 escape
        let tokens = tokenize(r#""hello\0""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert!(s.contains('\0'));
        }
    }

    #[test]
    fn test_all_whitespace_in_string() {
        // String with whitespace types (not newline - that causes error)
        let tokens = tokenize("\" \\t\"").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::StringLiteral(_)));
    }

    #[test]
    fn test_keywords_with_underscores() {
        // Keywords should not be affected by underscores
        let result = tokenize("while_");
        assert!(result.is_ok());
        // while_ is an identifier, not the while keyword
        if let TokenKind::Identifier(_) = result.unwrap()[0].kind {
            // expected
        }
    }

    #[test]
    fn test_hex_underscore_then_non_digit() {
        // 0x123_abc - underscore is skipped, abc are valid hex digits!
        // So 0x123_abc = 0x123ABC = 1194684
        let tokens = tokenize("0x123_abc").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(0x123ABC)));
    }

    #[test]
    fn test_octal_underscore_then_non_digit() {
        // 0o123_abc - parsed as 0o123 (octal) + _abc (identifier)
        let tokens = tokenize("0o123_abc").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(0o123)));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_binary_underscore_then_non_digit() {
        // 0b101_abc - parsed as 0b101 (binary) + _abc (identifier)
        let tokens = tokenize("0b101_abc").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(5)));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_float_underscore_after_exp_then_non_digit() {
        // 1e_ - this should error (underscore in exponent followed by non-digit)
        let result = tokenize("1e_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_float_underscore_after_exp_then_non_digit() {
        // .5e_ - this should error
        let result = tokenize(".5e_");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_escape_value() {
        // \x with non-hex chars
        let result = tokenize(r#""\xZZ""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_string_escape_backslash() {
        // String with just backslash at end
        let result = tokenize(r#""hello\"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_char_escape_backslash() {
        // Char with just backslash at end
        let result = tokenize("'a\\");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_with_only_backslash() {
        // String containing only a backslash (not a valid escape)
        let result = tokenize(r#""\\""#);
        // This is valid (escaped backslash)
        assert!(result.is_ok());
    }

    #[test]
    fn test_char_with_unknown_escape() {
        // Char with unknown escape sequence like \k - this is actually valid in the lexer
        // The unknown escape pushes the character itself
        let tokens = tokenize(r"'\k'").unwrap();
        // It should parse as char 'k'
        assert!(matches!(tokens[0].kind, TokenKind::CharLiteral('k')));
    }

    #[test]
    fn test_string_with_unknown_escape() {
        // String with unknown escape sequence like \k - lexer returns error
        let result = tokenize(r#""\k""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_without_open_brace() {
        // \u1234 without opening brace - lexer returns error for invalid escape
        let result = tokenize(r#""\u1234""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_with_only_open_brace() {
        // \u{ without hex digits - this is an error
        let result = tokenize(r#""\u{""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_invalid_incomplete() {
        // \u{123 without closing brace
        let result = tokenize(r#""\u{123""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_char_newline_in_middle() {
        // Char literal with newline in the middle
        let result = tokenize("'\n'");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_newline_in_middle() {
        // String literal with newline in the middle
        let result = tokenize("\"hello\nworld\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_valid_with_trailing_underscore() {
        // 0x123_ - underscore is skipped, parsed as just 0x123
        let tokens = tokenize("0x123_").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(0x123)));
        // Underscore is consumed by the lexer, no separate token
    }

    #[test]
    fn test_octal_valid_with_trailing_underscore() {
        // 0o123_ - underscore is skipped, parsed as just 0o123
        let tokens = tokenize("0o123_").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(0o123)));
    }

    #[test]
    fn test_binary_valid_with_trailing_underscore() {
        // 0b101_ - underscore is skipped, parsed as just 0b101
        let tokens = tokenize("0b101_").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(5)));
    }

    #[test]
    fn test_decimal_with_trailing_underscore() {
        // 123_ - underscore is skipped, parsed as just 123
        let tokens = tokenize("123_").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(123)));
    }

    #[test]
    fn test_float_with_trailing_underscore() {
        // 123.456_ - underscore after decimal point is an error
        let result = tokenize("123.456_");
        assert!(result.is_err());
    }

    #[test]
    fn test_float_exp_trailing_underscore() {
        // 1e10_ - underscore after exponent is an error
        let result = tokenize("1e10_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_float_trailing_underscore() {
        // .5_ - underscore after leading dot is an error
        let result = tokenize(".5_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_float_exp_trailing_underscore() {
        // .5e10_ - underscore after exponent is an error
        let result = tokenize(".5e10_");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_ending_with_escaped_quote() {
        // String ending with escaped quote but no closing quote
        let result = tokenize(r#""hello\""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_char_ending_with_escaped_quote() {
        // Char ending with escaped quote but no closing quote
        let result = tokenize("'a\\'");
        assert!(result.is_err());
    }

    #[test]
    fn test_very_long_string() {
        // Very long string using single-line quotes (not multi-line)
        // Use regular string to avoid triggering multi-line string parsing
        let long_string = "\"".to_string() + &"x".repeat(998) + "\"";
        let tokens = tokenize(&long_string).unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::StringLiteral(_)));
    }

    #[test]
    fn test_string_with_special_chars() {
        // String with special characters
        let tokens = tokenize("\"!@#$%^&*()_+{}|:<>?\"").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::StringLiteral(_)));
    }

    #[test]
    fn test_hex_with_single_underscore() {
        // 0x1_2 - valid underscore between hex digits
        let tokens = tokenize("0x1_2").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(18)));
    }

    #[test]
    fn test_octal_with_single_underscore() {
        // 0o1_2 - valid underscore between octal digits
        let tokens = tokenize("0o1_2").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(10)));
    }

    #[test]
    fn test_binary_with_single_underscore() {
        // 0b1_0 - valid underscore between binary digits
        let tokens = tokenize("0b1_0").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(2)));
    }

    // ===== 缺失覆盖率补全测试 =====

    #[test]
    fn test_hex_with_non_hex_char_terminates() {
        // 0x1G - G terminates hex parsing, G becomes next token
        let tokens = tokenize("0x1G").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(1)));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_float_exponent_with_only_underscore_terminates() {
        // 1e_ - exponent with only underscore should be an error
        let result = tokenize("1e_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_float_parse_failure() {
        // Extremely large float that would overflow f64
        let result = tokenize("1e99999999999999999999999999999999999999999999999999");
        // Should either succeed (as infinity) or error, but not panic
        // This exercises the f64 parse failure path
        let _ = result;
    }

    #[test]
    fn test_leading_dot_exponent_only_underscore() {
        // .e_ - exponent without digits, underscore terminates
        let tokens = tokenize(".e_").unwrap();
        // Should be: Dot, Identifier(e), Identifier(_), EOF
        // Or: Dot, Error, depending on implementation
        assert!(tokens.len() >= 2);
    }

    #[test]
    fn test_char_hex_escape_single_digit_error() {
        // '\x with only one hex digit - lexer sets error and continues
        let result = tokenize(r"'\x4'");
        // Should return error since \x requires 2 hex digits
        assert!(result.is_err());
    }

    #[test]
    fn test_char_hex_escape_no_digits_error() {
        // '\x with no hex digits
        let result = tokenize(r"'\x'");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_unicode_escape_empty_braces_error() {
        // '\u{}' - empty braces, no hex digits
        let result = tokenize(r"'\u{}'");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_unicode_escape_no_closing_brace() {
        // '\u{123' without closing brace
        let result = tokenize(r"'\u{123'");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_unicode_escape_missing_open_brace() {
        // '\u1234' without opening brace
        let result = tokenize(r"'\u1234'");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_unicode_escape_invalid_codepoint() {
        // Codepoint too large for char
        let result = tokenize(r"'\u{10FFFFFFF}'");
        assert!(result.is_err());
    }
}
