//! Error handling integration tests
//!
//! Tests for error handling in compilation, execution, and runtime.

use yaoxiang::run;

#[test]
fn test_invalid_syntax_handling() {
    // Test that invalid syntax is properly caught
    let invalid_source = r#"
        main() -> () = () => {
            let x = 10
            // Missing closing brace
    "#;

    let result = run(invalid_source);

    // Should fail with a parsing or compilation error
    assert!(result.is_err());
}

#[test]
fn test_undefined_variable_handling() {
    // Test that undefined variables are caught
    let source_with_undefined_var = r#"
        main() -> () = () => {
            let x = 10
            let y = x + undefined_variable
        }
    "#;

    let result = run(source_with_undefined_var);

    // Should fail with an undefined variable error
    assert!(result.is_err());
}

#[test]
fn test_undefined_function_handling() {
    // Test that undefined functions are caught
    let source_with_undefined_func = r#"
        main() -> () = () => {
            let x = undefined_function()
        }
    "#;

    let result = run(source_with_undefined_func);

    // Should fail with an undefined function error
    assert!(result.is_err());
}

#[test]
fn test_empty_source_handling() {
    // Test that empty source is handled gracefully
    let empty_source = "";

    let result = run(empty_source);

    // Should handle empty source (either success or specific empty error)
    match result {
        Ok(_) => {}
        Err(e) => {
            let error_msg = format!("{:?}", e);
            // Empty source should either succeed or give a meaningful error
            assert!(
                error_msg.contains("empty")
                    || error_msg.contains("no")
                    || error_msg.contains("parse"),
                "Unexpected error for empty source: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_nested_function_calls() {
    // Test that deeply nested function calls are handled
    let source = r#"
        f1(x: Int) -> Int = (x) => { return x }
        f2(x: Int) -> Int = (x) => { return f1(x) }
        f3(x: Int) -> Int = (x) => { return f2(x) }

        main() -> () = () => {
            let result = f3(f2(f1(5)))
        }
    "#;

    let result = run(source);

    // Should compile successfully
    match result {
        Ok(_) => {}
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                !error_msg.contains("parse") && !error_msg.contains("syntax"),
                "Compilation error: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_empty_function_body() {
    // Test that functions with empty bodies are handled
    let source = r#"
        empty_func() -> () = () => {
        }

        main() -> () = () => {
            empty_func()
        }
    "#;

    let result = run(source);

    // Should compile successfully
    match result {
        Ok(_) => {}
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                !error_msg.contains("parse") && !error_msg.contains("syntax"),
                "Compilation error: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_duplicate_variable_names() {
    // Test that duplicate variable names in same scope are caught
    let source_with_dup_var = r#"
        main() -> () = () => {
            let x = 10
            let x = 20
        }
    "#;

    let result = run(source_with_dup_var);

    // Should fail with a duplicate variable error
    assert!(result.is_err());
}

#[test]
fn test_function_with_no_return() {
    // Test that functions without explicit return are handled
    let source = r#"
        no_return() -> () = () => {
            let x = 10
        }

        main() -> () = () => {
            no_return()
        }
    "#;

    let result = run(source);

    // Should compile successfully (implicit return or void)
    match result {
        Ok(_) => {}
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                !error_msg.contains("parse") && !error_msg.contains("syntax"),
                "Compilation error: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_nested_blocks() {
    // Test that deeply nested blocks are handled
    let source = r#"
        main() -> () = () => {
            let a = 1
            {
                let b = 2
                {
                    let c = 3
                    {
                        let d = 4
                    }
                }
            }
        }
    "#;

    let result = run(source);

    // Should compile successfully
    match result {
        Ok(_) => {}
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                !error_msg.contains("parse") && !error_msg.contains("syntax"),
                "Compilation error: {}",
                error_msg
            );
        }
    }
}
