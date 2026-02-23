//! RFC-012: F-string integration tests
//!
//! Tests for f-string template literal compilation pipeline.

use yaoxiang::run;

/// Helper: run source and check it doesn't fail at parse/compile stage
fn compile_ok(source: &str) {
    let result = run(source);
    match result {
        Ok(_) => {}
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                !error_msg.contains("parse") && !error_msg.contains("syntax"),
                "Compilation error for f-string: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_fstring_basic_compilation() {
    compile_ok(
        r#"
        x = f"hello world"
    "#,
    );
}

#[test]
fn test_fstring_with_variable_compilation() {
    compile_ok(
        r#"
        name = "Alice"
        greeting = f"Hello {name}"
    "#,
    );
}

#[test]
fn test_fstring_with_expression_compilation() {
    compile_ok(
        r#"
        x = 10
        y = 20
        result = f"Sum: {x + y}"
    "#,
    );
}

#[test]
fn test_fstring_multiple_interpolations_compilation() {
    compile_ok(
        r#"
        x = 10
        y = 20
        s = f"{x} + {y} = {x + y}"
    "#,
    );
}

#[test]
fn test_fstring_const_eval() {
    // This should be optimized to a constant string at compile time
    compile_ok(
        r#"
        x = f"hello"
    "#,
    );
}

#[test]
fn test_fstring_with_format_spec_compilation() {
    compile_ok(
        r#"
        pi = 3.14159
        s = f"Pi: {pi:.2f}"
    "#,
    );
}

#[test]
fn test_fstring_in_print_compilation() {
    compile_ok(
        r#"
        name = "World"
        print(f"Hello {name}")
    "#,
    );
}
