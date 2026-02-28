//! RFC-012: F-string 类型检查测试

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::typecheck::{check_module, MonoType, PolyType, TypeEnvironment};

/// 检查类型推断是否成功
fn check_fstring_type_inference(input: &str) -> Result<(), String> {
    let tokens = tokenize(input).map_err(|e| format!("Lexer error: {:?}", e))?;
    let ast = parse(&tokens).map_err(|e| format!("Parse error: {:?}", e))?;

    let mut env = TypeEnvironment::new();
    env.add_var(
        "print".to_string(),
        PolyType::mono(MonoType::Fn {
            params: vec![MonoType::String],
            return_type: Box::new(MonoType::Void),
            is_async: false,
        }),
    );

    check_module(&ast, &mut Some(env))
        .map(|_| ())
        .map_err(|e| format!("Type error: {:?}", e))
}

#[cfg(test)]
mod fstring_typecheck_tests {
    use super::*;

    #[test]
    fn test_fstring_type_is_string() {
        let result = check_fstring_type_inference(
            r#"
            x = f"hello"
        "#,
        );
        assert!(result.is_ok(), "F-string should type-check: {:?}", result);
    }

    #[test]
    fn test_fstring_with_int_interpolation() {
        let result = check_fstring_type_inference(
            r#"
            x = 42
            s = f"value: {x}"
        "#,
        );
        assert!(
            result.is_ok(),
            "F-string with int interpolation should type-check: {:?}",
            result
        );
    }

    #[test]
    fn test_fstring_with_string_interpolation() {
        let result = check_fstring_type_inference(
            r#"
            name = "Alice"
            s = f"Hello {name}"
        "#,
        );
        assert!(
            result.is_ok(),
            "F-string with string interpolation should type-check: {:?}",
            result
        );
    }

    #[test]
    fn test_fstring_with_expression() {
        let result = check_fstring_type_inference(
            r#"
            x = 10
            y = 20
            s = f"sum: {x + y}"
        "#,
        );
        assert!(
            result.is_ok(),
            "F-string with expression should type-check: {:?}",
            result
        );
    }
}
