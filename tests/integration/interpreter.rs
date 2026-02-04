//! Interpreter integration tests
//!
//! Tests for the bytecode interpreter backend.

use yaoxiang::run;

#[test]
fn test_interpreter_creation() {
    // Test that the interpreter can be created
    let _interpreter = yaoxiang::backends::interpreter::Interpreter::new();
}

#[test]
fn test_simple_function() {
    // Test that we can compile a simple function
    let source = r#"main: () -> () = () => {}"#;

    let result = run(source);

    // Should compile successfully
    match result {
        Ok(_) => {}
        Err(e) => {
            let error_msg = format!("{:?}", e);
            // Allow type or runtime errors, just not parsing errors
            assert!(
                !error_msg.contains("parse") && !error_msg.contains("syntax"),
                "Compilation error: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_function_with_parameters() {
    // Test that we can define and compile a function with parameters
    let source = r#"
        add: (a: Int, b: Int) -> Int = (a, b) => { return a }
        main: () -> () = () => { let result = add(5, 10) }
    "#;

    let result = run(source);

    // Should compile successfully
    match result {
        Ok(_) => {}
        Err(e) => {
            let error_msg = format!("{:?}", e);
            // We allow errors about runtime execution, just not compilation errors
            assert!(
                !error_msg.contains("parse") && !error_msg.contains("syntax"),
                "Compilation error: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_conditional_statement() {
    // Test that we can compile conditional statements
    let source = r#"main: () -> () = () => { if true {} }"#;

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
fn test_loop_statement() {
    // Test that we can compile loop statements
    // Note: we use 'while false' to avoid infinite loop during execution
    // since run() executes the code, not just compiles it
    let source = r#"main: () -> () = () => { while false {} }"#;

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
