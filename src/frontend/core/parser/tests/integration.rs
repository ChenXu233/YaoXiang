//! Integration tests: full programs with mixed statement types

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

#[test]
fn test_empty_module() {
    let tokens = tokenize("").unwrap();
    let module = parse(&tokens).unwrap();
    assert!(module.items.is_empty());
}

#[test]
fn test_multiple_statements() {
    let source = "x = 1\ny = 2\nz = x + y";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();
    assert_eq!(module.items.len(), 3);
}

#[test]
fn test_function_with_body() {
    // RFC-010 完整函数
    let source = "add: (a: Int, b: Int) -> Int = { return a + b }";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();
    assert_eq!(module.items.len(), 1);
}

#[test]
fn test_type_def_and_use() {
    let source = "use std.io\nPoint: Type = { x: Float, y: Float }";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();
    assert_eq!(module.items.len(), 2);
}

#[test]
fn test_variable_and_binding() {
    let source = "x = 42\nPoint.distance = distance[0]";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();
    assert_eq!(module.items.len(), 2);
}
