//! Complex statement parsing tests - Increases coverage for stmt.rs, nud.rs, led.rs

use super::*;
use crate::frontend::lexer::tokenize;
use crate::frontend::parser::{parse, parse_expression, ParserState};

fn parse_type_anno(
    tokens: &[crate::frontend::lexer::tokens::Token]
) -> Option<crate::frontend::parser::ast::Type> {
    let mut state = ParserState::new(tokens);
    state.parse_type_anno()
}

/// =========================================================================
// 类型定义语句测试 (stmt.rs - parse_type_stmt)
/// =========================================================================

#[test]
fn test_parse_simple_type_definition() {
    let code = "type Color = red;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 1);
}

#[test]
fn test_parse_union_type_definition() {
    let code = "type Result = ok | err;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_generic_type_definition() {
    let code = "type Option[T] = some(T) | none;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_struct_constructor_type() {
    let code = "type Point = Point(x: Float, y: Float);";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_multi_variant_union_type() {
    // Test union type with 3+ variants
    let code = "type Status = pending | running | completed | failed;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_type_with_bracket_generics() {
    // Test [T, U] style generic params
    let code = "type Pair[T, U] = Pair(T, U);";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_struct_with_multiple_named_fields() {
    let code = "type Person = Person(name: string, age: int, city: string);";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_generic_struct_constructor() {
    let code = "type Box[T] = Box(value: T);";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_sum_type_definition() {
    // Sum type (non-variant union) - mixed types that don't all look like variants
    let code = "type Json = string | int | float;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nested_generic_union() {
    let code = "type Nested[T] = left(T) | right(T, T);";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_type_with_no_semicolon() {
    // Without trailing semicolon
    let code = "type Simple = MyType";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_variant_with_no_params() {
    let code = "type State = Active | Inactive | Deleted;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// Use 语句测试 (stmt.rs - parse_use_stmt)
/// =========================================================================

#[test]
fn test_parse_use_simple_path() {
    let code = "use std.io;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_use_with_items() {
    let code = "use std.io.{Reader, Writer};";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_use_with_alias() {
    let code = "use std.io as io;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_use_with_items_and_alias() {
    let code = "use std.io.{Reader, Writer} as io;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_use_nested_path() {
    let code = "use std.collection.list.List;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_use_single_item() {
    let code = "use std.io.{Reader};";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_use_path_no_items_no_alias() {
    let code = "use my.module.path;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// Identifier 语句测试 (stmt.rs - parse_identifier_stmt)
/// =========================================================================

#[test]
fn test_parse_identifier_assignment() {
    let code = "x = 42;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_identifier_function_call() {
    let code = "print(\"hello\");";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_identifier_with_method_call() {
    let code = "obj.method();";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_complex_assignment() {
    let code = "result = if cond { 1 } else { 2 };";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_field_access() {
    let code = "point.x;";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_index_expression_in_stmt() {
    let code = "arr[0];";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 模块定义测试 (stmt.rs - parse_module_stmt)
/// =========================================================================

#[test]
fn test_parse_module_with_multiple_items() {
    let code = "mod MyModule {
        let x = 1;
        let y = 2;
        fn foo() { x + y }
    }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nested_module() {
    let code = "mod Outer {
        mod Inner {
            let x = 1;
        }
    }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_empty_module() {
    let code = "mod Empty { }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_module_with_type_def() {
    let code = "mod Math {
        type Num = int | float;
    }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// Return/Break/Continue 语句测试 (stmt.rs)
/// =========================================================================

#[test]
fn test_parse_return_in_block() {
    let code = "fn test() { return 42; }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_return_empty_in_block() {
    let code = "fn test() { return; }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_break_in_loop() {
    let code = "while true { break; }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_break_with_label() {
    let code = "outer: while true { break ::outer; }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_continue_in_loop() {
    let code = "while cond { continue; }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_continue_with_label() {
    let code = "loop: while true { continue ::loop; }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// For 循环语句测试 (stmt.rs)
/// =========================================================================

#[test]
fn test_parse_for_loop() {
    let code = "for x in items { print(x); }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_for_loop_expression_body() {
    let code = "for x in items x + 1";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// While 循环语句测试 (nud.rs - parse_while)
/// =========================================================================

#[test]
fn test_parse_while_loop() {
    let code = "while i < 10 { i = i + 1; }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_while_with_label() {
    let code = "outer: while cond { body }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// Match 表达式测试 (nud.rs - parse_match)
/// =========================================================================

#[test]
fn test_parse_match_simple() {
    let code = "match x { 1 => \"one\", 2 => \"two\" }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_with_wildcard() {
    let code = "match x { 1 => \"one\", _ => \"other\" }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_or_pattern() {
    let code = "match x { 1 | 2 => \"one or two\" }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_guard() {
    let code = "match x { n if n > 0 => \"positive\" }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_identifier_pattern() {
    let code = "match x { n => n }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_union_pattern() {
    let code = "match opt { some(v) => v, none => 0 }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_tuple_pattern() {
    let code = "match pair { (a, b) => a + b }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_semicolon_separator() {
    let code = "match x { 1 => \"one\"; 2 => \"two\"; }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_string_literal_pattern() {
    let code = "match s { \"hello\" => 1, _ => 0 }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_char_literal_pattern() {
    let code = "match c { 'a' => 1, _ => 0 }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_bool_literal_pattern() {
    let code = "match b { true => 1, false => 0 }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 列表推导测试 (nud.rs - parse_list_comprehension)
/// =========================================================================

#[test]
fn test_parse_list_comprehension() {
    let code = "[x for x in items]";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_list_comprehension_with_condition() {
    let code = "[x for x in items if x > 0]";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// If-Else 表达式测试 (nud.rs)
/// =========================================================================

#[test]
fn test_parse_if_only_then() {
    let code = "if cond { 1 }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_if_then_else() {
    let code = "if cond { 1 } else { 2 }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_if_elif_chain() {
    let code = "if a { 1 } elif b { 2 } elif c { 3 } else { 4 }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// Lambda 表达式测试 (nud.rs + led.rs)
/// =========================================================================

#[test]
fn test_parse_lambda_single_param() {
    let code = "x => x + 1";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_lambda_multi_param() {
    let code = "(a, b) => a + b";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_lambda_block_body() {
    let code = "(a, b) => { a + b }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_lambda_empty_params() {
    let code = "() => 42";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 类型转换测试 (led.rs - parse_cast)
/// =========================================================================

#[test]
fn test_parse_cast_expression() {
    let code = "value as int";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_cast_with_generic() {
    let code = "value as Option[int]";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 函数调用增强测试 (led.rs)
/// =========================================================================

#[test]
fn test_parse_field_access_with_call() {
    let code = "obj.method(arg)";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_index_expression() {
    let code = "arr[0]";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_chained_index() {
    let code = "matrix[0][1]";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 块表达式测试 (nud.rs)
/// =========================================================================

#[test]
fn test_parse_empty_block() {
    let code = "{}";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_block_with_expression() {
    let code = "{ 1 + 2 }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_block_with_statements() {
    let code = "{ let x = 1; x + 2 }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 范围表达式测试 (led.rs - binary operators)
/// =========================================================================

#[test]
fn test_parse_range_expression() {
    let code = "1..10";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 赋值表达式测试 (led.rs)
/// =========================================================================

#[test]
fn test_parse_assignment() {
    let code = "x = 42";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_complex_assignment_expr() {
    let code = "result = if cond { 1 } else { 2 }";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 变量声明测试 (stmt.rs)
/// =========================================================================

#[test]
fn test_parse_var_simple() {
    let code = "let x = 42;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_var_with_type() {
    let code = "let x: int = 42;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_var_mut() {
    let code = "let mut counter = 0;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_var_mut_with_type() {
    let code = "let mut x: int = 10;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_var_no_initializer() {
    let code = "let x: int;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 函数定义测试 (stmt.rs - parse_fn_stmt)
/// =========================================================================

#[test]
fn test_parse_fn_full_signature() {
    let code = "add(Int, Int) -> Int = (a, b) => a + b;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_fn_no_return_type() {
    let code = "add(Int, Int) = (a, b) => a + b;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_fn_no_params() {
    let code = "getValue() -> Int = () => 42;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_fn_block_body() {
    let code = "factorial(Int) -> Int = (n) => {
        if n <= 1 { 1 }
        else { n * factorial(n - 1) }
    };";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_fn_single_param_type_annot() {
    let code = "double(Int) -> Int = (x) => x * 2;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 模块定义测试 (stmt.rs - parse_module_stmt)
/// =========================================================================

#[test]
fn test_parse_module() {
    let code = "mod MyModule { let x = 1; }";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 复合语句测试
/// =========================================================================

#[test]
fn test_parse_multiple_statements() {
    let code = "let x = 1; let y = 2; let z = x + y;";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nested_expressions() {
    let code = "foo(bar(baz(1)))";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_complex_expression() {
    let code = "arr.filter(x => x > 0).map(x => x * 2)";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 列表和元组测试
/// =========================================================================

#[test]
fn test_parse_empty_list() {
    let code = "[]";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_list_with_elements() {
    let code = "[1, 2, 3]";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_single_element_tuple() {
    let code = "(42,)";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_empty_tuple() {
    let code = "()";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 一元运算符测试
/// =========================================================================

#[test]
fn test_parse_double_negation() {
    // Using variable to avoid comment-like parsing
    let code = "let x = 5; -(-x)";
    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_negation_of_expression() {
    let code = "-(a + b)";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_logical_not_of_bool() {
    let code = "!(a && b)";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 分组表达式测试
/// =========================================================================

#[test]
fn test_parse_grouped_binary_op() {
    let code = "(a + b) * c";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nested_groups() {
    let code = "((1 + 2) * (3 + 4))";
    let tokens = tokenize(code).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
// 更多二元运算符测试
/// =========================================================================

#[test]
fn test_parse_all_comparison_ops() {
    let cases = ["1 < 2", "1 <= 2", "1 > 2", "1 >= 2", "1 == 2", "1 != 2"];
    for case in cases {
        let tokens = tokenize(case).unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok(), "Failed for: {}", case);
    }
}

#[test]
fn test_parse_all_arithmetic_ops() {
    let cases = ["1 + 2", "1 - 2", "1 * 2", "1 / 2", "1 % 2"];
    for case in cases {
        let tokens = tokenize(case).unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok(), "Failed for: {}", case);
    }
}

#[test]
fn test_parse_logical_ops() {
    let cases = ["true && false", "true || false", "!(true)"];
    for case in cases {
        let tokens = tokenize(case).unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok(), "Failed for: {}", case);
    }
}

/// =========================================================================
// 类型解析器专项测试 (type_parser.rs)
/// =========================================================================

#[test]
fn test_parse_int_with_bit_width() {
    let tokens = tokenize("Int<32>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_float_with_bit_width() {
    let tokens = tokenize("Float<32>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_with_leading_generic() {
    // Testing the leading generic parameter list parsing
    let tokens = tokenize("<T> Vec<T>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_fn_type_with_fn_param() {
    // Function type with function parameter - using fn keyword directly
    let tokens = tokenize("fn(int) -> string -> bool").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_fn_type_with_multiple_fn_params() {
    // Using parentheses with multiple function types
    let tokens = tokenize("(fn(int), fn(string) -> bool)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_tuple_with_fn_type() {
    // Tuple containing function types
    let tokens = tokenize("(fn(int) -> string, fn(bool))").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_named_struct_type() {
    let tokens = tokenize("Person{name: string, age: int}").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_named_struct_with_multiple_fields() {
    let tokens = tokenize("Point{x: float, y: float, z: float}").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_qualified_generic_type() {
    let tokens = tokenize("std.collection.List<int>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_nested_generics() {
    let tokens = tokenize("Map<string, Option<int>>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_arrow_chaining() {
    // T -> R -> S (right associative)
    let tokens = tokenize("int -> string -> bool").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_list_of_fn_type() {
    let tokens = tokenize("[fn(int) -> string]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_fn_with_void_return() {
    // void as return type needs special handling
    let tokens = tokenize("(int) -> Void").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_empty_paren_then_arrow() {
    // () -> T
    let tokens = tokenize("() -> int").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_single_paren_type() {
    // (T) should be a tuple with 1 element
    let tokens = tokenize("(int)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_trailing_comma() {
    let tokens = tokenize("(int, string, bool,)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_with_spaces_in_generics() {
    let tokens = tokenize("Vec < int >").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_fn_type_no_parens_simple() {
    // Using fn keyword
    let tokens = tokenize("fn(int, string) -> bool").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_fn_type_no_parens_no_params() {
    let tokens = tokenize("fn() -> int").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_deeply_nested_generics() {
    let tokens = tokenize("Result<Option<Vec<int>>, string>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_mixed_generics_and_fn_types() {
    let tokens = tokenize("Map<int, fn(T) -> U>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_fn_type_with_generic_return() {
    let tokens = tokenize("fn(int) -> Option<T>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_with_multiple_generics() {
    let tokens = tokenize("Triple[int, string, bool]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_qualified_with_generics() {
    let tokens = tokenize("external.lib.Type[Generic]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_fn_type_multi_params() {
    let tokens = tokenize("fn(A, B, C) -> D").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_paren_grouped_type() {
    let tokens = tokenize("(int)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_arrow_with_fn_type() {
    let tokens = tokenize("fn(int) -> fn(bool) -> string").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_fn_in_tuple() {
    let tokens = tokenize("(fn(A), fn(B) -> C)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_with_lt_gt_generics() {
    let tokens = tokenize("Array[T]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_tuple_type_multi_element() {
    let tokens = tokenize("(A, B, C, D)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

/// =========================================================================
// Parser State 专项测试 (state.rs)
/// =========================================================================

#[test]
fn test_state_dummy_span_creation() {
    use crate::util::span::Span;
    let span = Span::dummy();
    assert!(span.is_dummy());
}

#[test]
fn test_state_span_operations() {
    let tokens = tokenize("42").unwrap();
    let mut state = ParserState::new(&tokens);

    // Test span at start
    let start_span = state.span();
    assert!(!start_span.is_dummy());

    // Test span_from
    state.bump();
    let _end_span = state.span();
    let combined = state.span_from(start_span);
    assert!(!combined.is_dummy());
    // Just check that the span has valid coordinates
    assert!(combined.start.line > 0);
}

#[test]
fn test_state_at_with_eof() {
    let tokens = tokenize("").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.at_end());
}

#[test]
fn test_state_peek_nth_beyond_bounds() {
    let tokens = tokenize("1").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.peek_nth(100).is_none());
}

#[test]
fn test_state_has_errors_initially_false() {
    let tokens = tokenize("").unwrap();
    let state = ParserState::new(&tokens);
    assert!(!state.has_errors());
}

#[test]
fn test_state_skip_non_matching() {
    use crate::frontend::lexer::tokens::TokenKind;
    let tokens = tokenize("+ -").unwrap();
    let mut state = ParserState::new(&tokens);
    assert!(!state.skip(&TokenKind::IntLiteral(42)));
    assert!(state.at(&TokenKind::Plus));
}

#[test]
fn test_state_expect_at_eof() {
    use crate::frontend::lexer::tokens::TokenKind;
    let tokens = tokenize("").unwrap();
    let mut state = ParserState::new(&tokens);
    assert!(!state.expect(&TokenKind::IntLiteral(42)));
    assert!(state.has_errors());
}

#[test]
fn test_state_multiple_errors() {
    use crate::frontend::lexer::tokens::TokenKind;
    use crate::frontend::parser::ParseError;

    let tokens = tokenize("42").unwrap();
    let mut state = ParserState::new(&tokens);

    state.error(ParseError::ExpectedToken(
        TokenKind::Plus,
        TokenKind::IntLiteral(42),
    ));
    state.error(ParseError::ExpectedToken(
        TokenKind::Minus,
        TokenKind::IntLiteral(42),
    ));

    assert!(state.has_errors());
    assert_eq!(state.into_errors().len(), 2);
}

#[test]
fn test_state_synchronize_skips_to_sync_point() {
    use crate::frontend::lexer::tokens::*;

    let tokens = vec![
        create_token(TokenKind::IntLiteral(42)),
        create_token(TokenKind::Error("error".to_string())),
        create_token(TokenKind::IntLiteral(10)),
        create_token(TokenKind::KwMut), // sync point
        create_token(TokenKind::IntLiteral(20)),
        create_token(TokenKind::Eof),
    ];

    let mut state = ParserState::new(&tokens);
    state.synchronize();

    // Should stop at KwMut (sync point)
    assert!(state.at(&TokenKind::KwMut));
}

#[test]
fn test_state_skip_to_sync() {
    use crate::frontend::lexer::tokens::*;

    let tokens = vec![
        create_token(TokenKind::IntLiteral(42)),
        create_token(TokenKind::Error("error".to_string())),
        create_token(TokenKind::KwType), // sync point
        create_token(TokenKind::Eof),
    ];

    let mut state = ParserState::new(&tokens);
    state.skip_to_sync();

    assert!(state.at(&TokenKind::KwType));
}

#[test]
fn test_state_can_start_stmt_with_kw_use() {
    let tokens = tokenize("use").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.can_start_stmt());
}

#[test]
fn test_state_can_start_expr_with_all_literal_types() {
    use crate::frontend::lexer::tokens::*;

    // All expression-starting tokens
    let test_cases = vec![
        TokenKind::IntLiteral(42),
        TokenKind::FloatLiteral(3.14),
        TokenKind::StringLiteral("test".to_string()),
        TokenKind::CharLiteral('a'),
        TokenKind::BoolLiteral(true),
    ];

    for kind in test_cases.iter().cloned() {
        let tokens = vec![create_token(kind.clone())];
        let state = ParserState::new(&tokens);
        assert!(
            state.can_start_expr(),
            " {:?} should start an expression",
            kind
        );
    }
}

#[test]
fn test_state_can_start_expr_with_punctuation() {
    use crate::frontend::lexer::tokens::*;

    let test_cases = vec![
        TokenKind::Minus,
        TokenKind::Plus,
        TokenKind::Not,
        TokenKind::LParen,
        TokenKind::LBrace,
        TokenKind::LBracket,
        TokenKind::Pipe,
    ];

    for kind in test_cases.iter().cloned() {
        let tokens = vec![create_token(kind.clone())];
        let state = ParserState::new(&tokens);
        assert!(
            state.can_start_expr(),
            " {:?} should start an expression",
            kind
        );
    }
}

#[test]
fn test_state_can_start_expr_with_keywords() {
    use crate::frontend::lexer::tokens::*;

    let test_cases = vec![
        TokenKind::KwIf,
        TokenKind::KwMatch,
        TokenKind::KwWhile,
        TokenKind::KwFor,
    ];

    for kind in test_cases.iter().cloned() {
        let tokens = vec![create_token(kind.clone())];
        let state = ParserState::new(&tokens);
        assert!(
            state.can_start_expr(),
            " {:?} should start an expression",
            kind
        );
    }
}

#[test]
fn test_state_can_start_stmt_with_various_keywords() {
    use crate::frontend::lexer::tokens::*;

    let test_cases = vec![TokenKind::KwMut, TokenKind::KwType, TokenKind::KwUse];

    for kind in test_cases.iter().cloned() {
        let tokens = vec![create_token(kind.clone())];
        let state = ParserState::new(&tokens);
        assert!(
            state.can_start_stmt(),
            " {:?} should start a statement",
            kind
        );
    }
}

#[test]
fn test_state_at_with_token() {
    use crate::frontend::lexer::tokens::TokenKind;
    let tokens = tokenize("42 + 10").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.at(&TokenKind::IntLiteral(42)));
    assert!(!state.at(&TokenKind::IntLiteral(10)));
}

#[test]
fn test_state_at_with_eof_after_consume() {
    let tokens = tokenize("42").unwrap();
    let mut state = ParserState::new(&tokens);
    state.bump(); // consume 42
    assert!(state.at_end());
}

#[test]
fn test_state_peek_next_token() {
    use crate::frontend::lexer::tokens::TokenKind;
    let tokens = tokenize("42 + 10").unwrap();
    let state = ParserState::new(&tokens);
    let next = state.peek();
    assert!(next.is_some());
    // Just verify peek returns a token, not None
    assert!(next.unwrap().kind != TokenKind::Eof);
}

#[test]
fn test_state_bump_consumes_token() {
    use crate::frontend::lexer::tokens::TokenKind;
    let tokens = tokenize("42 43").unwrap();
    let mut state = ParserState::new(&tokens);
    assert!(state.at(&TokenKind::IntLiteral(42)));
    state.bump();
    assert!(state.at(&TokenKind::IntLiteral(43)));
}

#[test]
fn test_state_skip_matching_token() {
    use crate::frontend::lexer::tokens::TokenKind;
    let tokens = tokenize("+ 42").unwrap();
    let mut state = ParserState::new(&tokens);
    assert!(state.at(&TokenKind::Plus));
    assert!(state.skip(&TokenKind::Plus));
    assert!(state.at(&TokenKind::IntLiteral(42)));
}

#[test]
fn test_state_synchronize_with_multiple_sync_points() {
    use crate::frontend::lexer::tokens::*;

    let tokens = vec![
        create_token(TokenKind::IntLiteral(1)),
        create_token(TokenKind::Error("err1".to_string())),
        create_token(TokenKind::IntLiteral(2)),
        create_token(TokenKind::Error("err2".to_string())),
        create_token(TokenKind::KwType), // first sync point
        create_token(TokenKind::IntLiteral(3)),
        create_token(TokenKind::Eof),
    ];

    let mut state = ParserState::new(&tokens);
    state.synchronize();

    // Should stop at KwType
    assert!(state.at(&TokenKind::KwType));
}

#[test]
fn test_state_error_adds_to_error_list() {
    use crate::frontend::parser::ParseError;
    use crate::frontend::lexer::tokens::TokenKind;

    let tokens = tokenize("test").unwrap();
    let mut state = ParserState::new(&tokens);

    assert!(!state.has_errors());
    state.error(ParseError::UnexpectedToken(TokenKind::Eof));
    assert!(state.has_errors());
}

#[test]
fn test_state_into_errors_returns_errors() {
    use crate::frontend::parser::ParseError;
    use crate::frontend::lexer::tokens::TokenKind;

    let tokens = tokenize("test").unwrap();
    let mut state = ParserState::new(&tokens);

    state.error(ParseError::ExpectedToken(TokenKind::Plus, TokenKind::Eof));

    let errors = state.into_errors();
    assert_eq!(errors.len(), 1);
}

#[test]
fn test_state_span_from_tracks_range() {
    let tokens = tokenize("42").unwrap();
    let mut state = ParserState::new(&tokens);

    let start = state.span();
    state.bump();

    let range = state.span_from(start);
    assert!(range.start.offset <= range.end.offset);
}

// Helper function to create tokens for state tests
fn create_token(
    kind: crate::frontend::lexer::tokens::TokenKind
) -> crate::frontend::lexer::tokens::Token {
    use crate::util::span::{Position, Span};
    crate::frontend::lexer::tokens::Token {
        kind,
        span: Span::new(
            Position::with_offset(1, 1, 0),
            Position::with_offset(1, 2, 1),
        ),
        literal: None,
    }
}
