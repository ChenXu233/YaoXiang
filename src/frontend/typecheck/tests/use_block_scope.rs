//! Block scope tests for `use` statements.

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
fn test_use_inside_block_not_visible_outside() {
    let code = r#"
main = {
    if true {
        use std.string
        pos = string.index_of("hello", "l")
    }
    pos2 = string.index_of("world", "o")
    return
}
"#;

    let errors = typecheck(code).expect_err("string should be out of scope outside block");
    assert!(
        errors.iter().any(|e| e.code == "E1001"),
        "expected E1001 unknown variable, got: {errors:?}"
    );
}

#[test]
fn test_use_inside_block_stays_available_in_same_block() {
    let code = r#"
main = {
    if true {
        use std.string
        a = string.index_of("hello", "l")
        b = string.index_of("world", "o")
    }
    return
}
"#;

    assert!(typecheck(code).is_ok());
}

#[test]
fn test_use_inside_function_not_visible_in_other_function() {
    let code = r#"
f = {
    use std.string
    _ = string.index_of("hello", "l")
    return
}

g = {
    x = string.index_of("world", "o")
    return
}
"#;

    let errors =
        typecheck(code).expect_err("function-local use should not leak to another function");
    assert!(
        errors.iter().any(|e| e.code == "E1001"),
        "expected E1001 unknown variable, got: {errors:?}"
    );
}
