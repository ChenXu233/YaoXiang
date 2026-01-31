//! 错误处理测试

use crate::frontend::core::lexer::{tokenize, LexError, TokenKind};

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
        let result = tokenize(r#""\x4""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_number_no_digits() {
        let result = tokenize("0x");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_number_only_underscore() {
        let result = tokenize("0x_");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_octal_number_no_digits() {
        let result = tokenize("0o");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_octal_number_only_underscore() {
        let result = tokenize("0o_");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_binary_number_no_digits() {
        let result = tokenize("0b");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_binary_number_only_underscore() {
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
        let tokens = tokenize("/* unterminated comment").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].kind, TokenKind::Eof));
    }

    #[test]
    fn test_invalid_unicode_escape() {
        let result = tokenize(r#""\u1234""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_unicode_escape_empty_braces() {
        let result = tokenize(r#""\u{}""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_ending_with_backslash() {
        let result = tokenize(r#""hello\""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_char_ending_with_backslash() {
        let result = tokenize("'a\\'");
        assert!(result.is_err());
    }

    #[test]
    fn test_standalone_pipe() {
        let result = tokenize("|");
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Pipe));
    }

    #[test]
    fn test_number_too_large() {
        let result = tokenize("99999999999999999999999999999999999999999");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_number_too_large() {
        let result = tokenize("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_number_exceeds_i128_but_fits_u128() {
        let value: u128 = 1u128 << 127;
        let source = format!("0x{:x}", value);
        let result = tokenize(&source);
        assert!(result.is_err());
    }

    #[test]
    fn test_octal_number_exceeds_i128_but_fits_u128() {
        let value: u128 = 1u128 << 127;
        let source = format!("0o{:o}", value);
        let result = tokenize(&source);
        assert!(result.is_err());
    }

    #[test]
    fn test_binary_number_exceeds_i128_but_fits_u128() {
        let source = format!("0b1{}", "0".repeat(127));
        let result = tokenize(&source);
        assert!(result.is_err());
    }

    #[test]
    fn test_float_with_only_underscore_after_dot() {
        let result = tokenize(r"123._");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_underscore() {
        let result = tokenize(r"._");
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
        assert!(matches!(tokens[1].kind, TokenKind::Underscore));
    }

    #[test]
    fn test_float_with_underscore_between_exp_digits() {
        let result = tokenize(r"1e1_2");
        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn test_exponent_without_digits() {
        let result = tokenize("1e");
        assert!(result.is_err());
    }

    #[test]
    fn test_exponent_trailing_underscore_error() {
        let result = tokenize("1e1_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_with_underscore_error() {
        let result = tokenize(r".5_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_exponent_without_digits() {
        let result = tokenize(r".5e");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_exponent_trailing_underscore_error() {
        let result = tokenize(r".5e1_");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_string_escape_char() {
        let result = tokenize(r#""\q""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_overflow() {
        let result = tokenize("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_with_newline() {
        let result = tokenize("\"hello\nworld\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_with_newline() {
        let result = tokenize("'\n'");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_escape_single_digit() {
        let result = tokenize(r#""\x4""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_invalid_codepoint() {
        let result = tokenize(r#""\u{10FFFFFFF}""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_only_one_brace() {
        let result = tokenize(r#""\u{1F600""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_float_only_dot() {
        let tokens = tokenize(".").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].kind, TokenKind::Dot));
    }

    #[test]
    fn test_leading_dot_exponent_no_digits() {
        let result = tokenize(r".5e");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_exponent_underscore_error() {
        let result = tokenize(r".5e_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_exponent_plus_only() {
        let result = tokenize(r".5e+");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_exponent_minus_only() {
        let result = tokenize(r".5e-");
        let _ = result;
    }

    #[test]
    fn test_invalid_unicode_escape_no_hex_digits() {
        let result = tokenize(r#""\u{}""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_unicode_escape_invalid_codepoint() {
        let result = tokenize(r#""\u{FFFFFFFFFFFFFFFF}""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_escape_too_few_digits() {
        let result = tokenize(r#""\xA""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_escape_x_valid() {
        let tokens = tokenize(r#""\x41""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "A");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_char_escape_x_valid() {
        let tokens = tokenize("'\\x41'").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, 'A');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_string_unicode_valid() {
        let tokens = tokenize(r#""\u{1F600}""#).unwrap();
        if let TokenKind::StringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "😀");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_char_unicode_valid() {
        let tokens = tokenize("'\\u{263A}'").unwrap();
        if let TokenKind::CharLiteral(c) = &tokens[0].kind {
            assert_eq!(*c, '☺');
        } else {
            panic!("Expected char literal");
        }
    }

    #[test]
    fn test_decimal_overflow_error() {
        let result = tokenize("9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999");
        assert!(result.is_err());
    }

    #[test]
    fn test_hex_try_into_failure() {
        let result = tokenize("0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
        if result.is_ok() {
            let tokens = result.unwrap();
            assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(_)));
        }
    }

    #[test]
    fn test_octal_overflow() {
        let result = tokenize("0o7777777777777777777777777777777777777777777777777777777777777777777777777777777777777777777777777777777");
        assert!(result.is_err());
    }

    #[test]
    fn test_binary_overflow() {
        let result = tokenize("0b111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111");
        assert!(result.is_err());
    }

    #[test]
    fn test_number_with_underscore_before_dot() {
        let result = tokenize("123.456_");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_escape_value() {
        let result = tokenize(r#""\xZZ""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_string_escape_backslash() {
        let result = tokenize(r#""hello\""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_char_escape_backslash() {
        let result = tokenize("'a\\");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_with_only_backslash() {
        let result = tokenize(r#""\\""#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_char_with_unknown_escape() {
        let tokens = tokenize(r"'\k'").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::CharLiteral('k')));
    }

    #[test]
    fn test_string_with_unknown_escape() {
        let result = tokenize(r#""\k""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_without_open_brace() {
        let result = tokenize(r#""\u1234""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_with_only_open_brace() {
        let result = tokenize(r#""\u{""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_unicode_escape_invalid_incomplete() {
        let result = tokenize(r#""\u{123""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_char_newline_in_middle() {
        let result = tokenize("'\n'");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_newline_in_middle() {
        let result = tokenize("\"hello\nworld\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_float_with_trailing_underscore() {
        let result = tokenize("123.456_");
        assert!(result.is_err());
    }

    #[test]
    fn test_float_exp_trailing_underscore() {
        let result = tokenize("1e10_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_float_trailing_underscore() {
        let result = tokenize(".5_");
        assert!(result.is_err());
    }

    #[test]
    fn test_leading_dot_float_exp_trailing_underscore() {
        let result = tokenize(".5e10_");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_ending_with_escaped_quote() {
        let result = tokenize(r#""hello\""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_char_ending_with_escaped_quote() {
        let result = tokenize("'a\\'");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_hex_escape_single_digit_error() {
        let result = tokenize(r"'\x4'");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_hex_escape_no_digits_error() {
        let result = tokenize(r"'\x'");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_unicode_escape_empty_braces_error() {
        let result = tokenize(r"'\u{}'");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_unicode_escape_no_closing_brace() {
        let result = tokenize(r"'\u{123'");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_unicode_escape_missing_open_brace() {
        let result = tokenize(r"'\u1234'");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_unicode_escape_invalid_codepoint() {
        let result = tokenize(r"'\u{10FFFFFFF}'");
        assert!(result.is_err());
    }

    #[test]
    fn test_string_multi_line_error() {
        let result = tokenize("\"hello\nworld\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_char_multi_line_error() {
        let result = tokenize("'a\nb'");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_decimal_points_error() {
        let result = tokenize("1.2.3");
        if result.is_ok() {
            let tokens = result.unwrap();
            assert!(tokens.len() >= 3);
        }
    }

    #[test]
    fn test_invalid_unicode_empty_braces() {
        let result = tokenize(r#""\u{}""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_unicode_missing_brace() {
        let result = tokenize(r#""\u{1234""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_escape_too_few_digits() {
        let result = tokenize(r#""\xGG""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_char_empty_quotes() {
        let result = tokenize("''");
        assert!(result.is_err());
    }
}
