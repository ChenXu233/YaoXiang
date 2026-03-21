//! Typecheck tests for `Result[T, E]` and `?` (RFC-001).

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::typecheck::TypeChecker;

fn typecheck(code: &str) -> Result<(), Vec<crate::util::diagnostic::Diagnostic>> {
    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();
    let mut checker = TypeChecker::new("test");
    checker.check_module(&module).map(|_| ())
}

#[test]
fn test_try_only_allowed_in_result_function() {
    let code = r#"
        foo: () -> Result[Int, String] = () => { return 1 }
        main: () -> Int = () => { return foo()? }
    "#;
    let errors = typecheck(code).unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == "E1081"),
        "expected E1081, got: {errors:?}"
    );
}

#[test]
fn test_try_requires_result_value() {
    let code = r#"
        main: () -> Result[Int, String] = () => { return 1? }
    "#;
    let errors = typecheck(code).unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == "E1082"),
        "expected E1082, got: {errors:?}"
    );
}

#[test]
fn test_try_error_type_mismatch() {
    let code = r#"
        foo: () -> Result[Int, Int] = () => { return 1 }
        bar: () -> Result[Int, String] = () => { return foo()? }
    "#;
    let errors = typecheck(code).unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == "E1083"),
        "expected E1083, got: {errors:?}"
    );
}

#[test]
fn test_try_allowed_with_matching_err_type() {
    let code = r#"
        foo: () -> Result[Int, String] = () => { return 1 }
        bar: () -> Result[Int, String] = () => { return foo()? }
    "#;
    assert!(typecheck(code).is_ok());
}
