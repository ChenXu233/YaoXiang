//! Typecheck tests for RFC-001/008 concurrency constraints.

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
fn test_spawn_outside_block_is_compile_error() {
    let code = "main: () -> Int = () => { spawn { 1 } 2 }";
    let errors = typecheck(code).unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == "E1080"),
        "expected E1080, got: {errors:?}"
    );
}

#[test]
fn test_spawn_allowed_in_block_function() {
    let code = "main: () -> Int @block = () => { spawn { 1 } 2 }";
    assert!(typecheck(code).is_ok());
}

#[test]
fn test_spawn_allowed_in_block_expr() {
    let code = "main: () -> Int = () => { @block { spawn { 1 } } 2 }";
    assert!(typecheck(code).is_ok());
}

#[test]
fn test_spawn_rejected_in_auto_block_even_with_block_fn() {
    let code = "main: () -> Int @block = () => { @auto { spawn { 1 } } 2 }";
    let errors = typecheck(code).unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == "E1080"),
        "expected E1080, got: {errors:?}"
    );
}
