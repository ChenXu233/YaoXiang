//! Error recovery tests — parse_with_recovery

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::{parse, parse_with_recovery};

#[test]
fn test_parse_valid_input() {
    let tokens = tokenize("x = 42").unwrap();
    let result = parse_with_recovery(&tokens);
    assert!(!result.has_errors);
    assert_eq!(result.module.items.len(), 1);
}

#[test]
fn test_parse_empty_input() {
    let tokens = tokenize("").unwrap();
    let result = parse_with_recovery(&tokens);
    assert!(!result.has_errors);
    assert!(result.module.items.is_empty());
}

#[test]
fn test_recovery_continues_after_error() {
    // @ 不是有效的语句起始，但解析器应该继续解析后续有效语句
    let source = "@\nx = 42";
    let tokens = tokenize(source).unwrap();
    let result = parse_with_recovery(&tokens);
    // 应该包含错误
    assert!(result.has_errors);
    // 也应该包含后续有效语句
    assert!(!result.module.items.is_empty());
}

#[test]
fn test_parse_returns_error() {
    // parse() 应该在第一个错误处返回 Err
    let tokens = tokenize("@").unwrap();
    let result = parse(&tokens);
    assert!(result.is_err());
}

#[test]
fn test_recovery_multiple_errors() {
    let source = "@\n@\nx = 42";
    let tokens = tokenize(source).unwrap();
    let result = parse_with_recovery(&tokens);
    assert!(result.has_errors);
    assert!(!result.errors.is_empty());
}

#[test]
fn test_parse_errors_collected() {
    let tokens = tokenize("@").unwrap();
    let result = parse_with_recovery(&tokens);
    assert!(!result.errors.is_empty());
}
