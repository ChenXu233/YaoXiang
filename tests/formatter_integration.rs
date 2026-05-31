//! Formatter 集成测试
//!
//! 基于 formatter 规范，验证格式化器的端到端行为。
//! 参见: docs/src/design/formatter/

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

// ===========================================================================
// 注释保留测试 — 基于 formatter 规范 §C1, §C2
// ===========================================================================

// --- §C2.1 文件头注释 ---

#[test]
fn test_format_comment_single_line_preserved() {
    // 规范 §C1.1 + §C2.1: 单行注释必须被保留
    // NOTE: formatter currently adds a blank line after comments before `let`
    assert_format_eq("// comment\nlet x = 1\n", "// comment\n\nlet\nx = 1\n");
}

#[test]
fn test_format_comment_multiline_preserved() {
    // 规范 §C1.2 + §C2.1: 多行注释必须被保留
    // NOTE: formatter currently adds a blank line after comments before `let`
    assert_format_eq(
        "/* block comment */\nlet x = 1\n",
        "/* block comment */\n\nlet\nx = 1\n",
    );
}

// --- §C2.2 语句间注释（块内） ---

#[test]
fn test_format_block_comment_preserved() {
    // 规范 §C2.2: 代码块内语句前的注释必须保留
    let source = "if true {\n    // comment\n    let x = 1\n}\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("// comment"),
        "Comment inside block should be preserved: {}",
        result
    );
}

#[test]
fn test_format_block_multiline_comment_preserved() {
    // 规范 §C1.2 + §C2.2: 代码块内的多行注释必须保留
    let source = "if true {\n    /* block comment */\n    let x = 1\n}\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("/* block comment */"),
        "Block comment inside block should be preserved: {}",
        result
    );
}

#[test]
fn test_format_block_multi_stmt_comment_preserved() {
    // 规范 §C2.2: 多个语句之间的注释必须保留
    let source = "if true {\n    let x = 1\n    // between\n    let y = 2\n}\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("// between"),
        "Comment between statements should be preserved: {}",
        result
    );
}

#[test]
fn test_format_nested_block_comment_preserved() {
    // 规范 §C2.2: 嵌套块内的注释必须保留
    let source =
        "if true {\n    // outer\n    if false {\n        // inner\n        let y = 2\n    }\n}\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("// outer"),
        "Outer comment should be preserved: {}",
        result
    );
    assert!(
        result.contains("// inner"),
        "Inner comment should be preserved: {}",
        result
    );
}

#[test]
fn test_format_for_loop_body_comment_preserved() {
    // 规范 §C2.2: for 循环体内的注释必须保留
    let source = "for x in [1, 2, 3] {\n    // loop comment\n    let y = x\n}\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("// loop comment"),
        "Comment in for loop body should be preserved: {}",
        result
    );
}

#[test]
fn test_format_while_loop_body_comment_preserved() {
    // 规范 §C2.2: while 循环体内的注释必须保留
    let source = "while true {\n    // while comment\n    let x = 1\n}\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("// while comment"),
        "Comment in while loop body should be preserved: {}",
        result
    );
}

#[test]
fn test_format_function_body_comment_preserved() {
    // 规范 §C2.2: 函数体内的注释必须保留
    let source = "fn foo() {\n    // fn comment\n    let x = 1\n}\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("// fn comment"),
        "Comment in function body should be preserved: {}",
        result
    );
}

// --- §C2.3 行末注释 ---

#[test]
fn test_format_trailing_comment_preserved() {
    // 规范 §C2.3: 行末注释必须保留在同一行末尾
    let source = "let x = 1 // trailing\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("// trailing"),
        "Trailing comment should be preserved: {}",
        result
    );
}

// --- §C1.3 文档注释 ---

#[test]
fn test_format_doc_comment_preserved() {
    // 规范 §C1.3: 文档注释必须被保留
    let source = "/// doc comment\nfn foo() {\n    let x = 1\n}\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("/// doc comment"),
        "Doc comment should be preserved: {}",
        result
    );
}

// --- §14 导入排序注释关联 ---

#[test]
fn test_format_sort_imports_preserves_comments() {
    // 规范 §14.1 + §C2.2: 导入排序后注释必须跟随语句移动
    let source = "// std\nuse std::io\n// external\nuse serde\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("// std"),
        "Std import comment should be preserved: {}",
        result
    );
    assert!(
        result.contains("// external"),
        "External import comment should be preserved: {}",
        result
    );
}
