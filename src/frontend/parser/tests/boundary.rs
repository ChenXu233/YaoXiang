//! Boundary case parser tests

use super::*;
use crate::frontend::lexer::tokenize;
use crate::frontend::parser::{parse, parse_expression};

/// Test deeply nested parentheses
#[test]
fn test_deeply_nested_parens() {
    let expr = "((((((1))))))";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test many nested additions
#[test]
fn test_many_additions() {
    let expr = "1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test deeply nested blocks
#[test]
fn test_deeply_nested_blocks() {
    let expr = "{ { { { 1 } } } }";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test many function parameters
#[test]
fn test_many_params() {
    let expr = "foo(Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) -> Int = (a, b, c, d, e, f, g, h, i, j) => { 0 }";
    let tokens = tokenize(expr).unwrap();
    let result = parse(&tokens);
    if let Err(e) = &result {
        println!("Parse error: {:?}", e);
    }
    assert!(result.is_ok());
}

/// Test deeply nested if statements
#[test]
fn test_deeply_nested_if() {
    let expr = "if a { if b { if c { if d { 1 } } } }";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test many match arms
#[test]
fn test_many_match_arms() {
    let expr = "match x {
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        6 => 6,
        7 => 7,
        8 => 8,
        9 => 9,
        10 => 10
    }";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test complex expression with all operators
#[test]
fn test_all_operators() {
    let expr = "-a + b * c / d % e < f > g <= h >= i == j != k && l || m";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test chained method calls
#[test]
fn test_chained_calls() {
    let expr = "foo().bar().baz()";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test complex indexing
#[test]
fn test_complex_indexing() {
    let expr = "arr[0][1][2][3]";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test complex field access chain
#[test]
fn test_complex_field_chain() {
    let expr = "a.b.c.d.e.f.g";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test deeply nested lambdas
#[test]
fn test_deeply_nested_lambdas() {
    let expr = "(x) => (y) => (z) => x + y + z";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test lambda with many parameters
#[test]
fn test_lambda_many_params() {
    let expr = "(a, b, c, d, e) => a + b + c + d + e";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test empty module
#[test]
fn test_empty_module_items() {
    let tokens = tokenize("").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().items.len(), 0);
}

/// Test module with only whitespace
#[test]
fn test_whitespace_only() {
    let tokens = tokenize("   \n\n   \t   ").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().items.len(), 0);
}

/// Test tuple with single element
#[test]
fn test_single_element_tuple() {
    let tokens = tokenize("(42,)").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test type with tuple
#[test]
fn test_tuple_type() {
    let expr = "x: (Int, String, Bool) = (1, \"hello\", true);";
    let tokens = tokenize(expr).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test optional type parameters
#[test]
fn test_optional_type_params() {
    let expr = "foo(Int,) -> Int = (x) => { x }";
    let tokens = tokenize(expr).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test type cast
#[test]
fn test_type_cast() {
    let expr = "x as Int";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test complex destructuring in let
#[test]
fn test_tuple_destructuring() {
    let expr = "(a, b, c) = (1, 2, 3);";
    let tokens = tokenize(expr).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test match with tuple patterns
#[test]
fn test_match_tuple_pattern() {
    let expr = "match (1, 2) { (a, b) => a + b }";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test string with escape sequences
#[test]
fn test_escaped_string() {
    let expr = "\"hello\\nworld\\t!\"";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test char with escape sequence
#[test]
fn test_escaped_char() {
    let expr = "'\\n'";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test large integer literal
#[test]
fn test_large_int_literal() {
    let expr = "123456789012345678901234567890";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test large float literal
#[test]
fn test_large_float_literal() {
    let expr = "3.141592653589793238462643383279";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test block with trailing expression
#[test]
fn test_block_trailing_expression() {
    let expr = "{ 1; 2; 3; 4; 5 }";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test named arguments in function call
#[test]
fn test_named_arguments() {
    let expr = "foo(x=1, y=2, z=3)";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test wildcard pattern in match
#[test]
fn test_wildcard_pattern() {
    let expr = "match x { _ => \"default\" }";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test for loop
#[test]
fn test_for_loop() {
    let expr = "for i in 0..10 { print(i); }";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

// === 类型定义测试 (Type Definition Tests) ===

/// Test simple type definition without parameters -> Type::Name
#[test]
fn test_parse_simple_type_no_param() {
    let code = "type Color = red";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(
        result.is_ok(),
        "Failed to parse: {}\nError: {:?}",
        code,
        result.err()
    );
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test union type definition with two variants -> Type::Variant
#[test]
fn test_parse_union_type_two_variants() {
    let code = "type Color = red | green";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test union type definition with three variants -> Type::Variant
#[test]
fn test_parse_union_type_three_variants() {
    let code = "type Color = red | green | blue";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test struct type definition with parameters -> Type::Struct
#[test]
fn test_parse_struct_type_with_params() {
    let code = "type Point = Point(x: Float, y: Float)";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test generic union type definition
#[test]
fn test_parse_generic_union_type() {
    let code = "type Result[T, E] = ok(T) | err(E)";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test generic type with angle brackets
#[test]
fn test_parse_generic_type_with_angle_brackets() {
    let code = "type Result<T, E> = ok(T) | err(E)";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test single parameter constructor
#[test]
fn test_parse_single_param_constructor() {
    let code = "type Box[T] = Box(value: T)";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test type with builtin type parameter
#[test]
fn test_parse_type_with_builtin_type() {
    let code = "type IntBox = Box(Int)";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test type definition with semicolon
#[test]
fn test_parse_type_with_semicolon() {
    let code = "type Color = red;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test multiple type definitions
#[test]
fn test_parse_multiple_type_definitions() {
    let code = "type Color = red | green | blue; type Point = Point(x: Float, y: Float)";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 2);
}

/// Test enum-like type definition
#[test]
fn test_parse_enum_like_type() {
    let code = "type Day = Monday | Tuesday | Wednesday | Thursday | Friday | Saturday | Sunday";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test mixed constructor types (with and without params)
#[test]
fn test_parse_mixed_constructor_types() {
    let code = "type Shape = circle(Float) | rect(Float, Float) | point";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok(), "Failed to parse: {}", code);
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}
