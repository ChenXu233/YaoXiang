//! RFC-012: F-string parser 测试

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::{parse_expression, ast};

#[cfg(test)]
mod fstring_parser_tests {
    use super::*;

    #[test]
    fn test_parse_fstring_text_only() {
        let tokens = tokenize(r#"f"hello""#).unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
        let expr = result.unwrap();
        if let ast::Expr::FString { segments, .. } = &expr {
            assert_eq!(segments.len(), 1);
            assert!(matches!(&segments[0], ast::FStringSegment::Text(s) if s == "hello"));
        } else {
            panic!("Expected FString, got {:?}", expr);
        }
    }

    #[test]
    fn test_parse_fstring_with_variable() {
        let tokens = tokenize(r#"f"hello {name}""#).unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
        let expr = result.unwrap();
        if let ast::Expr::FString { segments, .. } = &expr {
            assert_eq!(segments.len(), 2);
            assert!(matches!(&segments[0], ast::FStringSegment::Text(s) if s == "hello "));
            assert!(matches!(
                &segments[1],
                ast::FStringSegment::Interpolation { .. }
            ));
        } else {
            panic!("Expected FString, got {:?}", expr);
        }
    }

    #[test]
    fn test_parse_fstring_with_expression() {
        let tokens = tokenize(r#"f"sum: {x + y}""#).unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
        let expr = result.unwrap();
        if let ast::Expr::FString { segments, .. } = &expr {
            assert_eq!(segments.len(), 2);
            assert!(matches!(&segments[0], ast::FStringSegment::Text(s) if s == "sum: "));
            if let ast::FStringSegment::Interpolation { expr, format_spec } = &segments[1] {
                assert!(format_spec.is_none());
                // The expression should be a BinOp
                assert!(matches!(expr.as_ref(), ast::Expr::BinOp { .. }));
            } else {
                panic!("Expected Interpolation segment");
            }
        } else {
            panic!("Expected FString, got {:?}", expr);
        }
    }

    #[test]
    fn test_parse_fstring_with_format_spec() {
        let tokens = tokenize(r#"f"Pi: {pi:.2f}""#).unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
        let expr = result.unwrap();
        if let ast::Expr::FString { segments, .. } = &expr {
            assert_eq!(segments.len(), 2);
            if let ast::FStringSegment::Interpolation { format_spec, .. } = &segments[1] {
                assert_eq!(format_spec.as_deref(), Some(".2f"));
            } else {
                panic!("Expected Interpolation segment");
            }
        } else {
            panic!("Expected FString, got {:?}", expr);
        }
    }

    #[test]
    fn test_parse_fstring_multiple_interpolations() {
        let tokens = tokenize(r#"f"{x} + {y} = {z}""#).unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
        let expr = result.unwrap();
        if let ast::Expr::FString { segments, .. } = &expr {
            // Segments: Interpolation(x), Text(" + "), Interpolation(y), Text(" = "), Interpolation(z)
            assert_eq!(segments.len(), 5);
        } else {
            panic!("Expected FString, got {:?}", expr);
        }
    }

    #[test]
    fn test_parse_fstring_empty() {
        let tokens = tokenize(r#"f"""#).unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
        let expr = result.unwrap();
        if let ast::Expr::FString { segments, .. } = &expr {
            assert_eq!(segments.len(), 1);
            assert!(matches!(&segments[0], ast::FStringSegment::Text(s) if s.is_empty()));
        } else {
            panic!("Expected FString, got {:?}", expr);
        }
    }
}
