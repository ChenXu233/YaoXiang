//! Basic parser tests

use super::*;
use crate::frontend::lexer::tokenize;
use crate::frontend::parser::{parse, parse_expression};

/// Test parsing an empty module
#[test]
fn test_parse_empty_module() {
    let tokens = tokenize("").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
    let module = result.unwrap();
    assert!(module.items.is_empty());
}

/// Test parsing a simple integer literal
#[test]
fn test_parse_int_literal() {
    let tokens = tokenize("42").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing a simple float literal
#[test]
fn test_parse_float_literal() {
    let tokens = tokenize("3.14").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing a string literal
#[test]
fn test_parse_string_literal() {
    let tokens = tokenize("\"hello\"").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing a char literal
#[test]
fn test_parse_char_literal() {
    let tokens = tokenize("'a'").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing a bool literal
#[test]
fn test_parse_bool_literal() {
    let tokens = tokenize("true").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());

    let tokens = tokenize("false").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing an identifier
#[test]
fn test_parse_identifier() {
    let tokens = tokenize("x").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing simple addition
#[test]
fn test_parse_addition() {
    let tokens = tokenize("1 + 2").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing subtraction
#[test]
fn test_parse_subtraction() {
    let tokens = tokenize("5 - 3").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing multiplication
#[test]
fn test_parse_multiplication() {
    let tokens = tokenize("4 * 5").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing division
#[test]
fn test_parse_division() {
    let tokens = tokenize("10 / 2").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing modulo
#[test]
fn test_parse_modulo() {
    let tokens = tokenize("10 % 3").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing comparison operators
#[test]
fn test_parse_comparison() {
    let cases = ["1 < 2", "1 <= 2", "1 > 2", "1 >= 2", "1 == 2", "1 != 2"];
    for case in cases {
        let tokens = tokenize(case).unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok(), "Failed for: {}", case);
    }
}

/// Test parsing logical operators
#[test]
fn test_parse_logical() {
    let tokens = tokenize("true && false").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());

    let tokens = tokenize("true || false").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing negation
#[test]
fn test_parse_negation() {
    let tokens = tokenize("-5").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing logical not
#[test]
fn test_parse_not() {
    let tokens = tokenize("!true").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing unary plus
#[test]
fn test_parse_unary_plus() {
    let tokens = tokenize("+5").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing grouped expression
#[test]
fn test_parse_grouped() {
    let tokens = tokenize("(1 + 2)").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing tuple
#[test]
fn test_parse_tuple() {
    let tokens = tokenize("(1, 2, 3)").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing empty tuple
#[test]
fn test_parse_empty_tuple() {
    let tokens = tokenize("()").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing simple function call
#[test]
fn test_parse_function_call() {
    let tokens = tokenize("foo()").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());

    let tokens = tokenize("foo(1, 2)").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing field access
#[test]
fn test_parse_field_access() {
    let tokens = tokenize("obj.field").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing index access
#[test]
fn test_parse_index() {
    let tokens = tokenize("arr[0]").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing variable declaration statement
#[test]
fn test_parse_var_statement() {
    let tokens = tokenize("x: int = 42;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test parsing variable declaration without type
#[test]
fn test_parse_var_statement_no_type() {
    let tokens = tokenize("y = 10;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test parsing mutable variable declaration
#[test]
fn test_parse_mut_var_statement() {
    let tokens = tokenize("mut z: int = 0;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test parsing function definition
#[test]
fn test_parse_function_definition() {
    let tokens = tokenize("add(Int, Int) -> Int = (a, b) => a + b").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

/// Test parsing if expression
#[test]
fn test_parse_if_expression() {
    let tokens = tokenize("if x > 0 { 1 } else { 0 }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing if-elif-else expression
#[test]
fn test_parse_if_elif_else() {
    let tokens = tokenize("if x > 0 { 1 } elif x == 0 { 0 } else { -1 }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing match expression
#[test]
fn test_parse_match_expression() {
    let tokens = tokenize("match x { 1 => \"one\", 2 => \"two\" }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing lambda expression
#[test]
fn test_parse_lambda() {
    let tokens = tokenize("(x) => x + 1").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing block expression
#[test]
fn test_parse_block() {
    let tokens = tokenize("{ 1; 2; 3 }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing while loop
#[test]
fn test_parse_while() {
    let tokens = tokenize("while i < 10 { i = i + 1 }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing return statement
#[test]
fn test_parse_return() {
    let tokens = tokenize("return 42;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test parsing break statement
#[test]
fn test_parse_break() {
    let tokens = tokenize("break;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test parsing continue statement
#[test]
fn test_parse_continue() {
    let tokens = tokenize("continue;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test parsing type definition
#[test]
fn test_parse_type_definition() {
    let tokens = tokenize("type MyInt = int;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test parsing type annotation
#[test]
fn test_parse_type_annotation() {
    let tokens = tokenize("x: int = 42;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test parsing generic type
#[test]
fn test_parse_generic_type() {
    let tokens = tokenize("list: List<Int> = [];").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test parsing function type
#[test]
fn test_parse_fn_type() {
    let tokens = tokenize("f: (Int) -> Int = (x) => x;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test parsing complex expression with operator precedence
#[test]
fn test_parse_precedence() {
    let tokens = tokenize("1 + 2 * 3 - 4 / 2").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());

    let tokens = tokenize("1 < 2 && 3 > 4 || 5 == 6").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing nested parentheses
#[test]
fn test_parse_nested_parens() {
    let tokens = tokenize("((1 + 2) * (3 + 4))").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// Test parsing labeled break
#[test]
fn test_parse_labeled_break() {
    let tokens = tokenize("break ::label;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// Test parsing use statement
#[test]
fn test_parse_use() {
    let tokens = tokenize("use std.io;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}
