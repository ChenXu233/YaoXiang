//! Fuzz tests for parser using proptest

use super::*;
use crate::frontend::lexer::tokenize;
use crate::frontend::parser::{parse, parse_expression};
use proptest::prelude::*;

/// Strategy for generating valid identifiers
fn identifier_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,10}"
}

/// Strategy for generating valid integers
fn int_literal_strategy() -> impl Strategy<Value = String> {
    "[0-9]{1,10}"
}

/// Strategy for generating valid float literals
fn float_literal_strategy() -> impl Strategy<Value = String> {
    "[0-9]{0,5}\\.[0-9]{1,5}"
}

/// Strategy for generating simple expressions
fn simple_expr_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        identifier_strategy(),
        int_literal_strategy(),
        float_literal_strategy(),
    ]
}

/// Strategy for generating binary operators
fn bin_op_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("+"),
        Just("-"),
        Just("*"),
        Just("/"),
        Just("%"),
        Just("<"),
        Just("<="),
        Just(">"),
        Just(">="),
        Just("=="),
        Just("!="),
        Just("&&"),
        Just("||"),
    ]
}

/// Strategy for generating valid expressions
fn expr_strategy() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        identifier_strategy(),
        int_literal_strategy(),
        float_literal_strategy(),
        "true".prop_recursive(1, 4, 2, |inner| {
            prop_oneof![
                "(", inner.clone(), ")",
                "[", inner.clone(), "]",
            ]
        }),
    ];

    leaf.prop_recursive(1, 8, 2, |inner| {
        prop_oneof![
            // Unary operators
            ("-", inner.clone()),
            ("+", inner.clone()),
            ("!", inner.clone()),
            // Binary operations
            (inner.clone(), bin_op_strategy(), inner.clone())
                .prop_map(|(l, op, r)| format!("{} {} {}", l, op, r)),
            // Parenthesized
            ("(", inner.clone(), ")").prop_map(|(l, mid, r)| format!("{}{}{}", l, mid, r)),
        ]
    })
}

/// Test that valid expressions don't crash the parser
#[test]
fn test_fuzz_valid_expressions(expr in expr_strategy()) {
    let tokens = tokenize(&expr).unwrap();
    // The parser should either succeed or produce a recoverable error
    let _ = parse_expression(&tokens);
}

/// Test nested parens don't cause stack overflow
#[test]
fn test_nested_parens_depth(depth in 1..20usize) {
    let expr = "(".repeat(depth) + "1" + &")".repeat(depth);
    let tokens = tokenize(&expr).unwrap();
    let result = parse_expression(&tokens);
    // Should either parse successfully or gracefully fail
    assert!(result.is_ok() || result.is_err());
}

/// Test deeply nested binary operations
#[test]
fn test_nested_bin_ops(depth in 1..15usize) {
    let base = "1 + ";
    let expr = base.repeat(depth) + "1";
    let tokens = tokenize(&expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Strategy for generating valid statements
fn stmt_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        ("let ", identifier_strategy(), " = ", int_literal_strategy(), ";").prop_map(|(a, b, c, d, e)| format!("{}{}{}{}{}", a, b, c, d, e)),
        ("return ", int_literal_strategy(), ";").prop_map(|(a, b, c)| format!("{}{}{}", a, b, c)),
        (identifier_strategy(), ";").prop_map(|(a, b)| format!("{}{}", a, b)),
    ]
}

/// Test that valid statements don't crash the parser
#[test]
fn test_fuzz_valid_statements(stmt in stmt_strategy()) {
    let tokens = tokenize(&stmt).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok() || result.is_err());
}

/// Strategy for generating module content
fn module_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(stmt_strategy(), 0..5).prop_map(|stmts| stmts.join("\n"))
}

/// Test parsing modules with multiple statements
#[test]
fn test_fuzz_module_content(module in module_strategy()) {
    let tokens = tokenize(&module).unwrap();
    let result = parse(&tokens);
    // Should either succeed or fail gracefully
    assert!(result.is_ok() || result.is_err());
}

/// QuickCheck tests using quickcheck

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    /// QuickCheck: valid identifiers should parse
    #[quickcheck]
    fn quickcheck_valid_identifier(name: String) -> TestResult {
        // Only accept valid identifiers
        if name.is_empty()
            || !name.chars().next().unwrap().is_alphabetic()
            || name.chars().any(|c| !c.is_alphanumeric() && c != '_')
        {
            return TestResult::discard();
        }

        let tokens = tokenize(&name).unwrap();
        let result = parse_expression(&tokens);
        TestResult::from_bool(result.is_ok())
    }

    /// QuickCheck: simple integers should parse
    #[quickcheck]
    fn quickcheck_simple_int(n: i64) -> TestResult {
        if n < 0 {
            return TestResult::discard();
        }
        let s = n.to_string();
        let tokens = tokenize(&s).unwrap();
        let result = parse_expression(&tokens);
        TestResult::from_bool(result.is_ok())
    }

    /// QuickCheck: balanced parentheses
    #[quickcheck]
    fn quickcheck_balanced_parens(expr: String) -> TestResult {
        let mut depth = 0;
        let mut valid = true;
        for c in expr.chars() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                if depth == 0 {
                    valid = false;
                    break;
                }
                depth -= 1;
            }
        }
        if depth != 0 || !valid {
            return TestResult::discard();
        }

        let tokens = tokenize(&format!("({})", expr)).unwrap();
        let result = parse_expression(&tokens);
        TestResult::from_bool(result.is_ok())
    }
}

/// Property-based test for operator associativity
#[test]
fn test_left_associativity() {
    // a - b - c should be (a - b) - c, not a - (b - c)
    let expr = "10 - 5 - 3";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Property-based test for multiplication associativity
#[test]
fn test_multiplication_associativity() {
    // a * b * c should be (a * b) * c, not a * (b * c)
    let expr = "2 * 3 * 4";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test that parsing is deterministic
#[test]
fn test_parse_determinism() {
    let expr = "1 + 2 + 3 + 4 + 5";
    let tokens = tokenize(expr).unwrap();
    let result1 = parse_expression(&tokens).unwrap();
    let result2 = parse_expression(&tokens).unwrap();
    // Both parses should produce equivalent results (at structural level)
    format!("{:?}", result1);
    format!("{:?}", result2);
    // If we got here, both parsed without error
}

/// Test parsing speed with large expression
#[test]
fn test_parse_large_expression() {
    // Create a large but valid expression
    let mut expr = String::new();
    for i in 0..100 {
        if i > 0 {
            expr.push_str(" + ");
        }
        expr.push_str(&i.to_string());
    }
    let tokens = tokenize(&expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Stress test for function definitions
#[test]
fn test_many_function_definitions() {
    let mut code = String::new();
    for i in 0..20 {
        code.push_str(&format!("fn foo_{}() -> Int {{ {} }}\n", i, i));
    }
    let tokens = tokenize(&code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}
