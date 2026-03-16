//! Regression tests for block/function scoped `use` statements.

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
fn test_use_module_inside_function_scope() {
    let code = r#"
main = {
    use std.string
    pos = string.index_of("hello", "l")
}
"#;

    assert!(typecheck(code).is_ok());
}

#[test]
fn test_use_module_alias_inside_function_scope() {
    let code = r#"
main = {
    use std.string as str
    pos = str.index_of("hello", "l")
}
"#;

    assert!(typecheck(code).is_ok());
}
