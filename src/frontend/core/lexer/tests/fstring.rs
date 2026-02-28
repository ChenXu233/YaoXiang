//! RFC-012: F-string lexer 测试

use crate::frontend::core::lexer::{tokenize, TokenKind};

#[cfg(test)]
mod fstring_lexer_tests {
    use super::*;

    #[test]
    fn test_fstring_basic() {
        let tokens = tokenize(r#"f"hello""#).unwrap();
        assert_eq!(tokens.len(), 2); // FStringLiteral + Eof
        if let TokenKind::FStringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected FStringLiteral, got {:?}", tokens[0].kind);
        }
    }

    #[test]
    fn test_fstring_with_interpolation() {
        let tokens = tokenize(r#"f"hello {name}""#).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FStringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "hello {name}");
        } else {
            panic!("Expected FStringLiteral, got {:?}", tokens[0].kind);
        }
    }

    #[test]
    fn test_fstring_multiple_interpolations() {
        let tokens = tokenize(r#"f"{x} + {y} = {z}""#).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FStringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "{x} + {y} = {z}");
        } else {
            panic!("Expected FStringLiteral, got {:?}", tokens[0].kind);
        }
    }

    #[test]
    fn test_fstring_with_format_spec() {
        let tokens = tokenize(r#"f"Pi: {pi:.2f}""#).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FStringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "Pi: {pi:.2f}");
        } else {
            panic!("Expected FStringLiteral, got {:?}", tokens[0].kind);
        }
    }

    #[test]
    fn test_fstring_expression() {
        let tokens = tokenize(r#"f"sum: {x + y}""#).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FStringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "sum: {x + y}");
        } else {
            panic!("Expected FStringLiteral, got {:?}", tokens[0].kind);
        }
    }

    #[test]
    fn test_fstring_escaped_braces() {
        let tokens = tokenize(r#"f"value: {{42}}""#).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FStringLiteral(s) = &tokens[0].kind {
            // {{ and }} are escaped to literal { and }
            assert_eq!(s, "value: {42}");
        } else {
            panic!("Expected FStringLiteral, got {:?}", tokens[0].kind);
        }
    }

    #[test]
    fn test_fstring_empty() {
        let tokens = tokenize(r#"f"""#).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FStringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "");
        } else {
            panic!("Expected FStringLiteral, got {:?}", tokens[0].kind);
        }
    }

    #[test]
    fn test_fstring_escape_sequence() {
        let tokens = tokenize(r#"f"line1\nline2""#).unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::FStringLiteral(s) = &tokens[0].kind {
            assert_eq!(s, "line1\nline2");
        } else {
            panic!("Expected FStringLiteral, got {:?}", tokens[0].kind);
        }
    }

    #[test]
    fn test_f_identifier_not_fstring() {
        // Just 'f' without a quote should be an identifier
        let tokens = tokenize("f").unwrap();
        assert_eq!(tokens.len(), 2);
        if let TokenKind::Identifier(name) = &tokens[0].kind {
            assert_eq!(name, "f");
        } else {
            panic!("Expected Identifier, got {:?}", tokens[0].kind);
        }
    }

    #[test]
    fn test_fstring_in_context() {
        // f-string in assignment context
        let tokens = tokenize(r#"x = f"hello {name}""#).unwrap();
        assert!(tokens.len() >= 4); // x, =, FStringLiteral, Eof
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(_)));
        assert!(matches!(tokens[1].kind, TokenKind::Eq));
        assert!(matches!(tokens[2].kind, TokenKind::FStringLiteral(_)));
    }
}
