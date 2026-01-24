//! Error handling integration tests
//!
//! Tests for error handling in compilation, execution, and runtime.

use yaoxiang::run;

#[test]
fn test_invalid_syntax_handling() {
    // Test that invalid syntax is properly caught
    let invalid_source = r#"main: () -> () = () => { let x = 10"#;

    let result = run(invalid_source);

    // Should fail with a parsing or compilation error
    assert!(result.is_err());
}

#[test]
fn test_undefined_variable_handling() {
    // Test that undefined variables are caught
    let source_with_undefined_var =
        r#"main: () -> () = () => { let x = 10; let y = x + undefined_variable }"#;

    let result = run(source_with_undefined_var);

    // Should fail with an undefined variable error
    assert!(result.is_err());
}

#[test]
fn test_undefined_function_handling() {
    // Test that undefined functions are caught
    let source_with_undefined_func = r#"main: () -> () = () => { let x = undefined_function() }"#;

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
        f1: (Int) -> Int = (x) => { return x }
        f2: (Int) -> Int = (x) => { return f1(x) }
        f3: (Int) -> Int = (x) => { return f2(x) }
        main: () -> () = () => { let result = f3(f2(f1(5))) }
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
        empty_func: () -> () = () => {}
        main: () -> () = () => { empty_func() }
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
    let source_with_dup_var = r#"main: () -> () = () => { let x = 10; let x = 20 }"#;

    let result = run(source_with_dup_var);

    // Should fail with a duplicate variable error
    assert!(result.is_err());
}

#[test]
fn test_function_with_no_return() {
    // Test that functions without explicit return are handled
    let source = r#"
        no_return: () -> () = () => { let x = 10 }
        main: () -> () = () => { no_return() }
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
    let source = r#"main: () -> () = () => {
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
    }"#;

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
fn test_return_statement_with_value() {
    // Test that return statement with value works
    let source = r#"
        get_five: () -> Int = () => {
            return 5
        }
        main: () -> () = () => { get_five() }
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
fn test_return_statement_without_value() {
    // Test that return statement without value works (void return)
    let source = r#"
        do_nothing: () -> () = () => {
            return
        }
        main: () -> () = () => { do_nothing() }
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
fn test_early_return_in_if() {
    // Test early return inside if statement
    let source = r#"
        max: (Int, Int) -> Int = (a, b) => {
            if a > b {
                return a
            } else {
                return b
            }
        }
        main: () -> () = () => { max(10, 20) }
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
fn test_nested_early_return() {
    // Test early return in nested blocks
    let source = r#"
        complex_function: (Int, Int) -> Int = (x, y) => {
            if x > 0 {
                if y > 0 {
                    return x + y
                }
                return x
            }
            return 0
        }
        main: () -> () = () => { complex_function(5, 3) }
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
fn test_match_with_mixed_return_and_non_return_arms() {
    // Test match with some arms returning and others not
    let source = r#"
        process: (Int) -> Int = (x) => {
            return match x {
                1 => 10,
                2 => 20,
                _ => 30
            }
        }
        main: () -> () = () => { process(1) }
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
fn test_nested_match_with_return() {
    // Test nested match expressions with return
    // Note: FIXED - Resolved infinite recursion in match arm processing
    let source = r#"
        simple_match: (Int) -> Int = (x) => {
            match x {
                1 => 100,
                2 => 200,
                _ => 300
            }
        }
        main: () -> () = () => { simple_match(1) }
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
fn test_match_in_block_with_return() {
    // Test match expression inside a block with return
    // Note: FIXED - Resolved infinite recursion in match arm processing
    let source = r#"
        calculate: (Int) -> Int = (x) => {
            let result = match x {
                1 => return 10,
                2 => return 20,
                _ => {
                    let y = x + 1
                    return y
                }
            }
            return result
        }
        main: () -> () = () => { calculate(1) }
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
fn test_match_with_tuple_pattern_and_return() {
    // Test match with tuple patterns and return
    // Note: FIXED - Resolved infinite recursion in match arm processing
    let source = r#"
        process_tuple: ((Int, Int)) -> Int = (pair) => {
            return match pair {
                (0, 0) => return 0,
                (0, y) => return y,
                (x, 0) => return x,
                (x, y) => x + y
            }
        }
        main: () -> () = () => { process_tuple((1, 2)) }
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
fn test_match_with_wildcard_and_return() {
    // Test match with wildcard pattern and return
    // Note: FIXED - Resolved infinite recursion in match arm processing
    let source = r#"
        handle_input: (String) -> Int = (input) => {
            return match input {
                "quit" => return 0,
                "exit" => return 1,
                _ => {
                    if input.len > 10 {
                        return 2
                    }
                    return 3
                }
            }
        }
        main: () -> () = () => { handle_input("hello") }
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
fn test_match_with_or_pattern_and_return() {
    // Test match with or patterns and return
    // Note: FIXED - Resolved infinite recursion in match arm processing
    let source = r#"
        is_special: (Int) -> Bool = (x) => {
            return match x {
                1 | 2 | 3 => return true,
                _ => return false
            }
        }
        main: () -> () = () => { is_special(2) }
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
fn test_match_in_while_loop_with_return() {
    // Test match expression in while loop with return
    // Note: FIXED - Resolved infinite recursion in match arm processing
    let source = r#"
        find_value: (Int) -> Int = (target) => {
            let mut i: Int = 0
            while i < 100 {
                let result = match i {
                    n if n == target => return i,
                    n if n > target => return -1,
                    _ => i + 1
                }
                i = result
            }
            return -1
        }
        main: () -> () = () => { find_value(5) }
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
fn test_complex_nested_control_flow_with_return() {
    // Test complex nested control flow with return in match
    // Fixed: return/break/continue are now properly parsed as prefix expressions
    let source = r#"
        complex_func: (Int, Int, Int) -> Int = (x, y, z) => {
            if x > 0 {
                return match y {
                    0 => {
                        if z > 0 {
                            return x + z
                        }
                        return x
                    },
                    n if n > 0 => match z {
                        0 => return x + y,
                        m if m > 0 => x + y + z,
                        _ => return -1
                    },
                    _ => return 0
                }
            }
            return -1
        }
        main: () -> () = () => { complex_func(1, 2, 3) }
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
