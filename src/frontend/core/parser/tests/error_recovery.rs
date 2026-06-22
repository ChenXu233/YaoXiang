//! Error recovery tests — parse_with_recovery

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

#[test]
fn test_parse_valid_input() {
    let tokens = tokenize("x = 42").unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors);
    assert_eq!(result.module.items.len(), 1);
}

#[test]
fn test_parse_empty_input() {
    let tokens = tokenize("").unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors);
    assert!(result.module.items.is_empty());
}

#[test]
fn test_recovery_continues_after_error() {
    // @ 不是有效的语句起始，但解析器应该继续解析后续有效语句
    let source = "@\nx = 42";
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    // 应该包含错误
    assert!(result.has_errors);
    // 也应该包含后续有效语句
    assert!(!result.module.items.is_empty());
}

#[test]
fn test_parse_returns_error() {
    // parse() 应该返回 has_errors = true
    let tokens = tokenize("@").unwrap();
    let result = parse(&tokens);
    assert!(result.has_errors);
}

#[test]
fn test_recovery_multiple_errors() {
    let source = "@\n@\nx = 42";
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(result.has_errors);
    assert!(!result.errors.is_empty());
}

#[test]
fn test_parse_errors_collected() {
    let tokens = tokenize("@").unwrap();
    let result = parse(&tokens);
    assert!(!result.errors.is_empty());
}
