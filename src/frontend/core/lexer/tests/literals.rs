//! 字面量测试 - 整数、浮点、字符串、字符

use crate::frontend::core::lexer::{tokenize, LexError, TokenKind};

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
            assert_eq!(s, "\nline 1\nline 2\nline 3\n");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_multi_line_string_with_quotes() {
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
    fn test_multi_line_string_in_code() {
        let source = "\nlet json = \"\"\"\n{\n    \"key\": \"value\"\n}\n\"\"\"\nprintln(json)\n";
        let tokens = tokenize(source).unwrap();
        assert!(tokens.len() > 5);
        let has_string = tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::StringLiteral(_)));
        assert!(has_string);
    }

    #[test]
    fn test_float_edge_cases() {
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
        let cases = ["0", "0x0", "0o0", "0b0", "00", "007"];
        for input in cases {
            let result = tokenize(input);
            assert!(result.is_ok(), "Should parse: {}", input);
        }
    }

    #[test]
    fn test_negative_zero() {
        let tokens = tokenize("-0").unwrap();
        assert_eq!(tokens.len(), 3);
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
    fn test_float_parse_failure() {
        let result = tokenize("1e999999999999999999999999999");
        let _ = result;
    }

    #[test]
    fn test_multiple_underscores_in_float() {
        let tokens = tokenize("1_2_3.4_5_6").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_scientific_notation_uppercase_e() {
        let tokens = tokenize("1E10").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_scientific_notation_with_underscores() {
        let tokens = tokenize("1_000e1_0").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_bool_and_void_literals() {
        let tokens = tokenize("true").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::BoolLiteral(true)));

        let tokens = tokenize("false").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::BoolLiteral(false)));

        let tokens = tokenize("void").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::VoidLiteral));
    }

    #[test]
    fn test_string_empty() {
        let tokens = tokenize("\"\"").unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s.is_empty()));
    }

    #[test]
    fn test_char_with_space() {
        let tokens = tokenize("' '").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::CharLiteral(' ')));
    }

    #[test]
    fn test_string_with_unicode_char() {
        let tokens = tokenize("\"中文\"").unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert!(s.contains("中"));
        }
    }

    #[test]
    fn test_string_with_tabs() {
        let tokens = tokenize("\"hello\tworld\"").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::StringLiteral(_)));
    }

    #[test]
    fn test_float_zero() {
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
        let cases = ["-1", "-1.5", "-1e10", "-0xFF", "-0o77", "-0b1111"];
        for case in &cases {
            let tokens = tokenize(case).unwrap();
            assert!(tokens.len() >= 2);
        }
    }

    #[test]
    fn test_hex_uppercase_digits() {
        let tokens = tokenize("0xABCDEF").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(11259375)));
    }

    #[test]
    fn test_hex_mixed_case() {
        let tokens = tokenize("0xAbCdEf").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(_)));
    }

    #[test]
    fn test_octal_with_all_digits() {
        let tokens = tokenize("0o01234567").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(_)));
    }

    #[test]
    fn test_binary_with_all_ones() {
        let tokens = tokenize("0b1111").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(15)));
    }

    #[test]
    fn test_float_with_leading_zeros() {
        let tokens = tokenize("001.002").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_float_scientific_with_leading_zeros() {
        let tokens = tokenize("001e002").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(_)));
    }

    #[test]
    fn test_string_with_all_whitespace() {
        let tokens = tokenize("\"hello world\"").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::StringLiteral(_)));
    }

    #[test]
    fn test_very_long_string() {
        let long_string = "\"".to_string() + &"x".repeat(998) + "\"";
        let tokens = tokenize(&long_string).unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::StringLiteral(_)));
    }

    #[test]
    fn test_string_with_special_chars() {
        let tokens = tokenize("\"!@#$%^&*()_+{}|:<>?\"").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::StringLiteral(_)));
    }

    #[test]
    fn test_hex_with_single_underscore() {
        let tokens = tokenize("0x1_2").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(18)));
    }

    #[test]
    fn test_octal_with_single_underscore() {
        let tokens = tokenize("0o1_2").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(10)));
    }

    #[test]
    fn test_binary_with_single_underscore() {
        let tokens = tokenize("0b1_0").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(2)));
    }

    #[test]
    fn test_hex_with_non_hex_char_terminates() {
        let tokens = tokenize("0x1G").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(1)));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_float_exponent_with_only_underscore_terminates() {
        let result = tokenize("1e_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_float_parse_failure() {
        let result = tokenize("1e99999999999999999999999999999999999999999999999999");
        let _ = result;
    }

    #[test]
    fn test_leading_dot_exponent_only_underscore() {
        let tokens = tokenize(".e_").unwrap();
        assert!(tokens.len() >= 2);
    }

    #[test]
    fn test_float_leading_dot_with_exponent() {
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
    fn test_string_with_backslash_n() {
        let tokens = tokenize(r#""hello\n""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert!(s.contains('\n'));
        }
    }

    #[test]
    fn test_string_with_backslash_r() {
        let tokens = tokenize(r#""hello\r""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert!(s.contains('\r'));
        }
    }

    #[test]
    fn test_string_with_backslash_zero() {
        let tokens = tokenize(r#""hello\0""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert!(s.contains('\0'));
        }
    }

    #[test]
    fn test_all_whitespace_in_string() {
        let tokens = tokenize("\" \\t\"").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::StringLiteral(_)));
    }

    #[test]
    fn test_string_with_all_escapes() {
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
        let result = tokenize(r"'\n'");
        assert!(result.is_ok());

        let result = tokenize(r"'\t'");
        assert!(result.is_ok());

        let result = tokenize(r"'\r'");
        assert!(result.is_ok());

        let result = tokenize(r"'\\'");
        assert!(result.is_ok());

        let tokens = tokenize("'\"'").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::CharLiteral('"')));

        let result = tokenize(r"'\''");
        assert!(result.is_ok());

        let result = tokenize(r"'\0'");
        assert!(result.is_ok());
    }

    #[test]
    fn test_hex_underscore_then_non_digit() {
        let tokens = tokenize("0x123_abc").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(0x123ABC)));
    }

    #[test]
    fn test_octal_underscore_then_non_digit() {
        let tokens = tokenize("0o123_abc").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(0o123)));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_binary_underscore_then_non_digit() {
        let tokens = tokenize("0b101_abc").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(5)));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
    }

    #[test]
    fn test_float_underscore_after_exp_then_non_digit() {
        let result = tokenize("1e_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_float_underscore_after_exp_then_non_digit() {
        let result = tokenize(".5e_");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_with_embedded_quotes() {
        let tokens = tokenize(r#""hello \"world\"""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, r#"hello "world""#);
        }
    }

    #[test]
    fn test_char_with_embedded_quote() {
        let tokens = tokenize(r"'\''").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::CharLiteral('\'')));
    }

    #[test]
    fn test_hex_valid_with_trailing_underscore() {
        let tokens = tokenize("0x123_").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(0x123)));
    }

    #[test]
    fn test_octal_valid_with_trailing_underscore() {
        let tokens = tokenize("0o123_").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(0o123)));
    }

    #[test]
    fn test_binary_valid_with_trailing_underscore() {
        let tokens = tokenize("0b101_").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(5)));
    }

    #[test]
    fn test_decimal_with_trailing_underscore() {
        let tokens = tokenize("123_").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(123)));
    }

    #[test]
    fn test_unicode_escape_valid_chars() {
        let tests = [
            (r#"\u{00A9}"#, '©'),
            (r#"\u{00AE}"#, '®'),
            (r#"\u{2600}"#, '☀'),
            (r#"\u{2601}"#, '☁'),
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
        let tokens = tokenize(r#""\x7F""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            let expected = "\x7F" as &str;
            assert_eq!(s.as_str(), expected);
        }
    }
}
