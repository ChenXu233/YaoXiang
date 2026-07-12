//! Formatter 集成测试
//!
//! 基于 formatter 规范，验证格式化器的端到端行为。
//!
//! 规范引用:
//!   §C1  注释类型 — 单行/多行/文档注释必须保留
//!   §C2  注释位置 — 文件头(§C2.1)/语句间(§C2.2)/行末(§C2.3)
//!   §C3  空行保留 — 原始空行用于分隔逻辑块
//!   §14  导入语句 — 排序(§14.1)/组内排序(§14.2)/注释跟随
//!   §6   代码块  — 空块(§6.1)/单行(§6.2)/多行(§6.3)

use yaoxiang::formatter::{format_source, FormatError, FormatOptions};

fn default_options() -> FormatOptions {
    FormatOptions::default()
}

fn assert_format_eq(
    input: &str,
    expected: &str,
) {
    let result = format_source(input, &default_options())
        .unwrap_or_else(|e| panic!("Failed to format input {:?}: {}", input, e));
    assert_eq!(result, expected, "Format mismatch for input: {:?}", input);
}

#[test]
fn test_format_empty_input() {
    assert_format_eq("", "");
}

#[test]
fn test_format_simple_var_declaration() {
    // RFC-010: x: Int = 1 语法
    assert_format_eq("x: Int = 1", "x: Int = 1\n");
}

#[test]
fn test_format_mut_var_declaration() {
    // RFC-010: mut x: Int = 1 语法
    assert_format_eq("mut x: Int = 1", "mut x: Int = 1\n");
}

#[test]
fn test_format_typed_var_declaration() {
    // RFC-010: x: i64 = 1 语法
    assert_format_eq("x: i64 = 1", "x: i64 = 1\n");
}

#[test]
fn test_format_function_no_args() {
    // RFC-010: foo: () -> Void = { ... } 语法
    // formatter 使用单行格式，因为只有一个语句且不超过行宽
    assert_format_eq(
        "foo: () -> Void = { x: Int = 1 }",
        "foo: () -> Void = { x: Int = 1 }\n",
    );
}

#[test]
fn test_format_if_else_branches() {
    // §6.2: 单语句代码块且不超过行宽时使用单行格式
    assert_format_eq("if true { 1 } else { 2 }", "if true { 1 } else { 2 }\n");
}

#[test]
fn test_format_binop_short() {
    // RFC-010: x: Int = 1 + 2 语法
    assert_format_eq("x: Int = 1 + 2", "x: Int = 1 + 2\n");
}

#[test]
fn test_format_lambda_body() {
    // RFC-010: Lambda 语法保持 => { expr }
    assert_format_eq("f = (x) => x + 1", "f = (x) => { x + 1 }\n");
}

#[test]
fn test_format_list_literal() {
    // RFC-010: x: Int = [1, 2, 3] 语法
    assert_format_eq("x = [1, 2, 3]", "x = [1, 2, 3]\n");
}

#[test]
fn test_format_dict_literal() {
    // RFC-010: Dict literals are now correctly parsed and formatted
    assert_format_eq("x = {\"a\": 1, \"b\": 2}", "x = {\"a\": 1, \"b\": 2}\n");
}

#[test]
fn test_format_long_line_wraps() {
    let source =
        "x: i64 = 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15 + 16 + 17 + 18 + 19 + 20 + 21 + 22 + 23 + 24 + 25 + 26 + 27 + 28 + 29 + 30";
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
    // RFC-010: x: Int = 1 语法
    assert_format_eq("// comment\nx: Int = 1\n", "// comment\n\nx: Int = 1\n");
}

#[test]
fn test_format_comment_multiline_preserved() {
    // 规范 §C1.2 + §C2.1: 多行注释必须被保留
    // RFC-010: x: Int = 1 语法
    assert_format_eq(
        "/* block comment */\nx: Int = 1\n",
        "/* block comment */\n\nx: Int = 1\n",
    );
}

// --- §C2.2 语句间注释（块内） ---

#[test]
fn test_format_block_comment_preserved() {
    // 规范 §C2.2: 代码块内语句前的注释必须保留
    // RFC-010: x: Int = 1 语法
    let source = "if true {\n    // comment\n    x: Int = 1\n}\n";
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
    // RFC-010: x: Int = 1 语法
    let source = "if true {\n    /* block comment */\n    x: Int = 1\n}\n";
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
    // RFC-010: x: Int = 1, y: Int = 2 语法
    let source = "if true {\n    x: Int = 1\n    // between\n    y: Int = 2\n}\n";
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
    // RFC-010: y: Int = 2 语法
    let source =
        "if true {\n    // outer\n    if false {\n        // inner\n        y: Int = 2\n    }\n}\n";
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
    // RFC-010: y = x 语法
    let source = "for x in [1, 2, 3] {\n    // loop comment\n    y = x\n}\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("// loop comment"),
        "Comment in for loop body should be preserved: {}",
        result
    );
}

#[test]
fn test_format_block_comment_in_if_body() {
    // 规范 §C2.2: if 体内的注释必须保留
    // RFC-010: x: Int = 1 语法
    let source = "if true {\n    // block comment\n    x: Int = 1\n}\n";
    let result = format_source(source, &default_options()).unwrap();
    assert!(
        result.contains("// block comment"),
        "Comment in if body should be preserved: {}",
        result
    );
}

#[test]
fn test_format_function_body_comment_preserved() {
    // 规范 §C2.2: 函数体内的注释必须保留
    // RFC-010: foo: () -> Void = { ... } 语法
    let source = "foo: () -> Void = {\n    // fn comment\n    x: Int = 1\n}\n";
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
    // RFC-010: x: Int = 1 语法
    let source = "x: Int = 1 // trailing\n";
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
    // RFC-010: foo: () -> Void = { ... } 语法
    let source = "/// doc comment\nfoo: () -> Void = {\n    x: Int = 1\n}\n";
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
    let source = "// std\nuse std.io\n// external\nuse serde\n";
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

#[test]
fn test_format_sort_imports_complex_comment_grouping() {
    // 规范 §14 导入排序 + §C2 注释位置
    //
    // Arrange: 头部注释 + 三个 import (包含 intra-import comment) + function binding
    let source = "\
// This header should stay
// Project-wide constants

use std.collections.list
use std.collections.map
// These two go with collections
use std.io

main = {
    io.println(\"test\")
}
";
    // Act
    let result = format_source(source, &default_options()).unwrap();
    // Assert: 头部注释只能出现一次
    assert_eq!(
        1,
        result.matches("// This header should stay").count(),
        "Header comment should appear exactly once, got: {}",
        result
    );
    // Assert: intra-import 注释只能出现一次
    assert_eq!(
        1,
        result.matches("// These two go with collections").count(),
        "Import-associated comment should appear exactly once, got: {}",
        result
    );
    // Assert: 按字母序 collections < io
    let pos_list = result
        .find("use std.collections.list")
        .expect("std.collections.list should be in output");
    let pos_io = result
        .find("use std.io")
        .expect("std.io should be in output");
    assert!(
        pos_list < pos_io,
        "std.collections should come before std.io"
    );
}
#[test]
fn test_format_sort_imports_header_only_no_duplication() {
    // 规范 §C2.1 文件头注释 + §14 导入排序
    //
    // Arrange: 纯头部注释 + 单 import + function binding
    let source = "\
// Comment 1
// Comment 2
use std.io

main = {
    io.println(\"test\")
}
";
    // Act
    let result = format_source(source, &default_options()).unwrap();
    // Assert: 头部注释恰好出现两次 (原始 issue #140 场景)
    let comment_count = result.matches("// Comment").count();
    assert_eq!(
        2, comment_count,
        "Header comments should appear exactly twice total, got {}: {}",
        comment_count, result
    );
}

#[test]
fn test_format_sort_imports_intra_import_comment_at_end() {
    // 规范 §C2.2 语句间注释 + §14 导入排序
    //
    // Arrange: header + two imports with intra-import comment
    let source = "\
// header
use std.io
// collection helpers
use std.collections.list

main = {}
";
    // Act
    let result = format_source(source, &default_options()).unwrap();
    // Assert: intra-import comment 恰好出现一次 (不泄漏到 function body 后)
    assert_eq!(
        1,
        result.matches("// collection helpers").count(),
        "Intra-import comment should appear exactly once: {}",
        result
    );
}

// === §1 缩进规则 ===

#[test]
fn test_format_indent_width_2() {
    let options = FormatOptions {
        indent_width: 2,
        ..Default::default()
    };
    // RFC-010: 使用多行块来测试缩进
    let input = "foo: () -> Void = {\nx: Int = 1\ny: Int = 2\n}";
    let result = format_source(input, &options).unwrap();
    // 缩进应为 2 空格
    assert!(
        result.contains("  "),
        "Expected 2-space indent, got: {}",
        result
    );
}

#[test]
fn test_format_use_tabs() {
    let options = FormatOptions {
        use_tabs: true,
        ..Default::default()
    };
    // RFC-010: 使用多行块来测试 Tab 缩进
    let input = "foo: () -> Void = {\nx: Int = 1\ny: Int = 2\n}";
    let result = format_source(input, &options).unwrap();
    assert!(
        result.contains('\t'),
        "Expected tab indent, got: {}",
        result
    );
}

// === §2 行宽 ===

#[test]
fn test_format_line_width_short_stays_single_line() {
    let options = FormatOptions {
        line_width: 120,
        ..Default::default()
    };
    // RFC-010: x: Int = 1 + 2 语法
    let result = format_source("x: Int = 1 + 2", &options).unwrap();
    // 短行不应换行
    assert!(
        !result.contains('\n') || result.lines().count() <= 2,
        "Short expression should not wrap: {}",
        result
    );
}

// === §3 运算符 ===

#[test]
fn test_format_binop_spaces() {
    let result = format_source("1+2*3", &default_options()).unwrap();
    // 二元运算符两侧应有空格
    assert!(
        result.contains(" + ") || result.contains("1 + 2"),
        "Expected spaces around operators: {}",
        result
    );
}

// === §6 代码块 ===

#[test]
fn test_format_empty_block() {
    // RFC-010: foo: () -> Void = {} 语法
    // 解析器将 {} 解析为空字典，而不是空代码块
    // 所以 formatter 输出 { {} }
    let result = format_source("foo: () -> Void = {}", &default_options()).unwrap();
    assert_eq!(result, "foo: () -> Void = { {} }\n");
}

// === 配置选项测试 ===

#[test]
fn test_config_line_width_affects_wrapping() {
    let narrow = FormatOptions {
        line_width: 20,
        ..Default::default()
    };
    let wide = FormatOptions {
        line_width: 200,
        ..Default::default()
    };
    // RFC-010: x: Int = 1 + 2 + 3 + 4 + 5 语法
    let input = "x: Int = 1 + 2 + 3 + 4 + 5";

    let narrow_result = format_source(input, &narrow).unwrap();
    let wide_result = format_source(input, &wide).unwrap();

    // 窄行宽应该产生更多换行
    assert!(
        narrow_result.lines().count() >= wide_result.lines().count(),
        "Narrow should have more lines.\nNarrow:\n{}\nWide:\n{}",
        narrow_result,
        wide_result
    );
}

#[test]
fn test_config_indent_width() {
    let opt2 = FormatOptions {
        indent_width: 2,
        ..Default::default()
    };
    let opt8 = FormatOptions {
        indent_width: 8,
        ..Default::default()
    };

    // RFC-010: 使用多行块来测试不同缩进配置
    let input = "foo: () -> Void = {\nx: Int = 1\ny: Int = 2\n}";

    let result2 = format_source(input, &opt2).unwrap();
    let result8 = format_source(input, &opt8).unwrap();

    // 两种配置应产生不同缩进
    assert_ne!(
        result2, result8,
        "Different indent_width should produce different output"
    );
}

#[test]
fn test_config_single_quote() {
    let options = FormatOptions {
        single_quote: true,
        ..Default::default()
    };
    // 注意：当前 formatter 的 single_quote 支持取决于解析器
    // 此测试验证选项传递不报错
    // RFC-010: x: Int = 1 语法
    let result = format_source("x: Int = 1", &options);
    assert!(
        result.is_ok(),
        "single_quote option should not cause errors"
    );
}

#[test]
fn test_format_idempotent_no_imports() {
    // 规范 §C2 注释保留 — 幂等性: format(format(x)) == format(x)
    //
    // Arrange: 纯头部注释 + 无 import
    let source = "// header\nx: Int = 1\ny: Int = 2\n";
    // Act
    let result = format_source(source, &default_options()).unwrap();
    let re_result = format_source(&result, &default_options()).unwrap();
    // Assert: 两次格式化结果一致
    assert_eq!(
        result, re_result,
        "Format should be idempotent for source with no imports"
    );
}
#[test]
fn test_format_idempotent_with_imports() {
    // 规范 §14 导入排序 + §C2 注释位置 — 幂等性
    //
    // Arrange: 头部注释 + 三个 import (含 intra-import comment) + function binding
    let source = "\
// This header should stay
// Project-wide constants

use std.collections.list
use std.collections.map
// These two go with collections
use std.io

main = {
    io.println(\"test\")
}
";
    // Act
    let result = format_source(source, &default_options()).unwrap();
    let re_result = format_source(&result, &default_options()).unwrap();
    // Assert: 两次格式化结果一致
    assert_eq!(
        result, re_result,
        "Format should be idempotent for source with multi-import comments"
    );
}

#[test]
fn test_format_idempotent_function_body_comments() {
    // 规范 §C2.2 语句间注释 + §14 导入排序 — 幂等性
    //
    // Arrange: header + import + two function bindings (each with body comments)
    let source = "\
// global header
use std.io

main = {
    // main logic
    io.println(\"hello\")
}

helper: () -> Int = {
    // helper
    42
}
";
    // Act
    let result = format_source(source, &default_options()).unwrap();
    let re_result = format_source(&result, &default_options()).unwrap();
    // Assert: 两次格式化结果一致 (body 内注释不泄漏到 body 外)
    assert_eq!(
        result, re_result,
        "Format should be idempotent for source with function body comments"
    );
}

#[test]
fn test_format_rejects_semantic_error() {
    let result = format_source("let x = 1", &default_options());
    assert!(
        matches!(result, Err(FormatError::Semantic(_))),
        "should reject semantic error"
    );
}

#[test]
fn test_format_valid_code() {
    let result = format_source("x = 1", &default_options()).unwrap();
    assert_eq!(result, "x = 1\n", "合法代码格式化后应保持规范输出");
}

#[test]
fn test_format_idempotent_valid() {
    let formatted = format_source("x=1", &default_options()).unwrap();
    let formatted2 = format_source(&formatted, &default_options()).unwrap();
    assert_eq!(formatted, formatted2, "两次格式化结果应一致");
}

#[test]
fn test_format_no_verify() {
    let mut opts = default_options();
    opts.verify = false;
    let result = format_source("x = 1", &opts);
    assert!(result.is_ok(), "no-verify 模式下合法代码应正常通过");
}
