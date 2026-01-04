//! Type parser tests

use crate::frontend::lexer::tokenize;
use crate::frontend::parser::ParserState;
use crate::util::span::Span;

fn create_dummy_span() -> Span {
    Span::dummy()
}

fn parse_type_anno(
    tokens: &[crate::frontend::lexer::tokens::Token],
) -> Option<crate::frontend::parser::ast::Type> {
    let mut state = ParserState::new(tokens);
    state.parse_type_anno()
}

// =========================================================================
// 空类型测试
// =========================================================================

#[test]
fn test_parse_empty_tuple_type() {
    let tokens = tokenize("()").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Tuple(t) if t.is_empty()));
}

#[test]
fn test_parse_void_type() {
    let tokens = tokenize("void").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Void));
}

// =========================================================================
// 基本类型测试
// =========================================================================

#[test]
fn test_parse_bool_type() {
    let tokens = tokenize("bool").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Bool));
}

#[test]
fn test_parse_char_type() {
    let tokens = tokenize("char").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Char));
}

#[test]
#[ignore]
fn test_parse_string_type() {
    let tokens = tokenize("string").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::String));
}

#[test]
fn test_parse_bytes_type() {
    let tokens = tokenize("bytes").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Bytes));
}

#[test]
fn test_parse_int_type() {
    let tokens = tokenize("int").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Int(64)));
}

#[test]
fn test_parse_float_type() {
    let tokens = tokenize("float").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Float(64)));
}

// =========================================================================
// 元组类型测试
// =========================================================================

#[test]
fn test_parse_single_element_tuple() {
    let tokens = tokenize("(int)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Tuple(t) if t.len() == 1));
}

#[test]
fn test_parse_two_element_tuple() {
    let tokens = tokenize("(int, string)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Tuple(t) if t.len() == 2));
}

#[test]
fn test_parse_multi_element_tuple() {
    let tokens = tokenize("(int, string, bool)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Tuple(t) if t.len() == 3));
}

// =========================================================================
// 函数类型测试
// =========================================================================

#[test]
fn test_parse_fn_type_no_params() {
    let tokens = tokenize("() -> int").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(
        matches!(ty, crate::frontend::parser::ast::Type::Fn { params, return_type, } if params.is_empty())
    );
}

#[test]
fn test_parse_fn_type_single_param() {
    let tokens = tokenize("(int) -> string").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(
        matches!(ty, crate::frontend::parser::ast::Type::Fn { params, return_type, }
        if params.len() == 1 && matches!(*return_type, crate::frontend::parser::ast::Type::String))
    );
}

#[test]
fn test_parse_fn_type_multiple_params() {
    let tokens = tokenize("(int, string, bool) -> float").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(
        matches!(ty, crate::frontend::parser::ast::Type::Fn { params, return_type, }
        if params.len() == 3)
    );
}

#[test]
fn test_parse_fn_type_no_return_type() {
    // Function with () return type should be parsed as returning void
    let tokens = tokenize("(int) -> ()").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_fn_type_nested() {
    let tokens = tokenize("(int) -> (string) -> bool").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

// =========================================================================
// 列表类型测试
// =========================================================================

#[test]
fn test_parse_list_type() {
    let tokens = tokenize("[int]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::List(_)));
}

#[test]
fn test_parse_list_of_list() {
    let tokens = tokenize("[[int]]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::List(inner)
        if matches!(*inner, crate::frontend::parser::ast::Type::List(_))));
}

#[test]
fn test_parse_list_of_tuple() {
    let tokens = tokenize("[(int, string)]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

// =========================================================================
// 命名类型测试
// =========================================================================

#[test]
fn test_parse_named_type() {
    let tokens = tokenize("MyType").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Name(name) if name == "MyType"));
}

#[test]
fn test_parse_qualified_name() {
    let tokens = tokenize("std.io.Reader").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(
        matches!(ty, crate::frontend::parser::ast::Type::Name(name) if name == "std.io.Reader")
    );
}

// =========================================================================
// 泛型类型测试
// =========================================================================

#[test]
fn test_parse_generic_type_single_arg() {
    let tokens = tokenize("List<int>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(
        matches!(ty, crate::frontend::parser::ast::Type::Generic { name, args }
        if name == "List" && args.len() == 1)
    );
}

#[test]
fn test_parse_generic_type_multiple_args() {
    let tokens = tokenize("Dict<int, string>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(
        matches!(ty, crate::frontend::parser::ast::Type::Generic { name, args }
        if name == "Dict" && args.len() == 2)
    );
}

#[test]
fn test_parse_nested_generic() {
    let tokens = tokenize("Dict<string, List<int>>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(
        matches!(ty, crate::frontend::parser::ast::Type::Generic { name, args }
        if name == "Dict" && args.len() == 2)
    );
}

#[test]
fn test_parse_generic_type_in_list() {
    let tokens = tokenize("[List<int>]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_option_type() {
    let tokens = tokenize("Option<int>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(
        matches!(ty, crate::frontend::parser::ast::Type::Generic { name, args }
        if name == "Option" && args.len() == 1)
    );
}

#[test]
fn test_parse_result_type() {
    let tokens = tokenize("Result<int, string>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(
        matches!(ty, crate::frontend::parser::ast::Type::Generic { name, args }
        if name == "Result" && args.len() == 2)
    );
}

// =========================================================================
// Set 类型测试
// =========================================================================

#[test]
fn test_parse_set_type() {
    let tokens = tokenize("Set<int>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(
        matches!(ty, crate::frontend::parser::ast::Type::Generic { name, args }
        if name == "Set" && args.len() == 1)
    );
}

// =========================================================================
// 复杂嵌套类型测试
// =========================================================================

#[test]
fn test_parse_complex_type() {
    let tokens = tokenize("Dict<string, List<Dict<int, float>>>").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_fn_with_fn_param() {
    // (int) -> (string) -> bool
    let tokens = tokenize("(int) -> fn(string) -> bool").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_tuple_of_generics() {
    let tokens = tokenize("(List<int>, Dict<string, bool>, Option<float>)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    let ty = result.unwrap();
    assert!(matches!(ty, crate::frontend::parser::ast::Type::Tuple(t) if t.len() == 3));
}

// =========================================================================
// 结构体类型测试
// =========================================================================

#[test]
fn test_parse_struct_type() {
    let tokens = tokenize("{ x: int, y: string }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_empty_struct() {
    let tokens = tokenize("{}").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

// =========================================================================
// 联合类型测试
// #[test]
// fn test_parse_union_type() {
//     let tokens = tokenize("int | string | bool").unwrap();
//     let result = parse_type_anno(&tokens);
//     assert!(result.is_some());
// }

// =========================================================================
// 边界情况测试
// =========================================================================

#[test]
fn test_parse_nested_parens() {
    let tokens = tokenize("(((int)))").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_list_with_generic() {
    let tokens = tokenize("[List<int>]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_with_spaces() {
    let tokens = tokenize("  List   <   int   >   ").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}
