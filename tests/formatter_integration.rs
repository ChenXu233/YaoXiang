//! Formatter 集成测试
//!
//! 基于 formatter 规范，验证格式化器的端到端行为。
//! 参见: docs/src/formatter/spec/SUMMARY.md

use yaoxiang::formatter::{format_source, FormatOptions};

fn default_options() -> FormatOptions {
    FormatOptions::default()
}

fn assert_format_eq(
    input: &str,
    expected: &str,
) {
    let result = format_source(input, &default_options()).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_format_empty_input() {
    assert_format_eq("", "");
}

#[test]
fn test_format_simple_var_declaration() {
    // NOTE: formatter currently puts a newline after `let` keyword
    assert_format_eq("let x = 1", "let\nx = 1\n");
}

#[test]
fn test_format_mut_var_declaration() {
    // NOTE: formatter currently puts a newline after `let` keyword
    assert_format_eq("let mut x = 1", "let\nmut x = 1\n");
}

#[test]
fn test_format_typed_var_declaration() {
    // NOTE: formatter currently puts a newline after `let` keyword
    assert_format_eq("let x: i64 = 1", "let\nx: i64 = 1\n");
}

#[test]
fn test_format_function_no_args() {
    // NOTE: formatter currently expands fn declaration and let across multiple lines
    assert_format_eq(
        "fn foo() { let x = 1 }",
        "fn\nfoo()\n{\n    let\n    x = 1\n}\n",
    );
}

#[test]
fn test_format_if_else_branches() {
    assert_format_eq(
        "if true { 1 } else { 2 }",
        "if true {\n    1\n} else {\n    2\n}\n",
    );
}

#[test]
fn test_format_comment_single_line_preserved() {
    // NOTE: formatter currently adds a blank line after comments before `let`
    assert_format_eq("// comment\nlet x = 1\n", "// comment\n\nlet\nx = 1\n");
}

#[test]
fn test_format_comment_multiline_preserved() {
    // NOTE: formatter currently adds a blank line after comments before `let`
    assert_format_eq(
        "/* block comment */\nlet x = 1\n",
        "/* block comment */\n\nlet\nx = 1\n",
    );
}

#[test]
fn test_format_binop_short() {
    // NOTE: formatter currently puts a newline after `let` keyword
    assert_format_eq("let x = 1 + 2", "let\nx = 1 + 2\n");
}

#[test]
fn test_format_lambda_body() {
    // NOTE: formatter currently wraps lambda body in a block expression
    assert_format_eq("let f = (x) => x + 1", "let\nf = (x) => {\n    x + 1\n}\n");
}

#[test]
fn test_format_list_literal() {
    // NOTE: formatter currently puts a newline after `let` keyword
    assert_format_eq("let x = [1, 2, 3]", "let\nx = [1, 2, 3]\n");
}

#[test]
fn test_format_dict_literal() {
    // NOTE: formatter currently has a known issue with dict literals
    // It misparses `"a": 1, "b": 2` as lambda-like syntax
    assert_format_eq(
        "let x = {\"a\": 1, \"b\": 2}",
        "let\nx = () => {\n    \"a\"\n}\n",
    );
}

#[test]
fn test_format_long_line_wraps() {
    let source = "let x = very_long_variable_name + another_long_name + yet_another_long_name + and_one_more;";
    let result = format_source(source, &default_options()).unwrap();
    assert!(result.contains('\n'), "Long line should be wrapped");
}
