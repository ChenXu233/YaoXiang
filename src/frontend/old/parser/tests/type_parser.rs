//! Type parser tests

use crate::frontend::lexer::tokenize;
use crate::frontend::parser::ParserState;
use crate::util::span::Span;

fn create_dummy_span() -> Span {
    Span::dummy()
}

fn parse_type_anno(
    tokens: &[crate::frontend::lexer::tokens::Token]
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
    let tokens = tokenize("Void").unwrap();
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
        matches!(ty, crate::frontend::parser::ast::Type::Fn { params, return_type: _, } if params.is_empty())
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
        matches!(ty, crate::frontend::parser::ast::Type::Fn { params, return_type: _, }
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

// =========================================================================
// RFC-010: 统一类型语法测试
// =========================================================================

#[test]
fn test_parse_struct_type_rfc010() {
    // 结构体：type Point = { x: Float, y: Float }
    let tokens = tokenize("{ x: Float, y: Float }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    match result.unwrap() {
        crate::frontend::parser::ast::Type::Struct(fields) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].0, "x");
            assert_eq!(fields[1].0, "y");
        }
        _ => panic!("Expected Struct type"),
    }
}

#[test]
fn test_parse_enum_simple_variants_rfc010() {
    // 简单枚举：type Color = { red | green | blue }
    let tokens = tokenize("{ red | green | blue }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    match result.unwrap() {
        crate::frontend::parser::ast::Type::Variant(variants) => {
            assert_eq!(variants.len(), 3);
            assert_eq!(variants[0].name, "red");
            assert_eq!(variants[1].name, "green");
            assert_eq!(variants[2].name, "blue");
            // 简单变体没有参数
            assert_eq!(variants[0].params.len(), 0);
        }
        _ => panic!("Expected Variant type"),
    }
}

#[test]
fn test_parse_enum_with_payload_rfc010() {
    // 带载荷的枚举：type Result[T, E] = { ok(T) | err(E) }
    let tokens = tokenize("{ ok(Int) | err(String) }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    match result.unwrap() {
        crate::frontend::parser::ast::Type::Variant(variants) => {
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].name, "ok");
            assert_eq!(variants[1].name, "err");
            // ok 变体有载荷
            assert_eq!(variants[0].params.len(), 1);
        }
        _ => panic!("Expected Variant type"),
    }
}

#[test]
fn test_parse_interface_rfc010() {
    // 接口：type Drawable = { draw: (Surface) -> Void }
    let tokens = tokenize("{ draw: (Surface) -> Void }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    match result.unwrap() {
        crate::frontend::parser::ast::Type::Struct(fields) => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].0, "draw");
            // 检查字段类型是函数类型
            match &fields[0].1 {
                crate::frontend::parser::ast::Type::Fn {
                    params,
                    return_type,
                    ..
                } => {
                    assert_eq!(params.len(), 1);
                    assert!(matches!(
                        **return_type,
                        crate::frontend::parser::ast::Type::Void
                    ));
                }
                _ => panic!("Expected Fn type for interface method"),
            }
        }
        _ => panic!("Expected Struct type for interface"),
    }
}

#[test]
fn test_parse_nested_struct_rfc010() {
    // 嵌套结构体
    let tokens = tokenize("{ x: { a: Int, b: Int }, y: Float }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    match result.unwrap() {
        crate::frontend::parser::ast::Type::Struct(fields) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].0, "x");
            // x 字段也是结构体
            match &fields[0].1 {
                crate::frontend::parser::ast::Type::Struct(nested_fields) => {
                    assert_eq!(nested_fields.len(), 2);
                }
                _ => panic!("Expected nested Struct"),
            }
        }
        _ => panic!("Expected Struct type"),
    }
}

#[test]
fn test_parse_mixed_enum_variants_rfc010() {
    // 混合枚举：既有载荷变体，也有无载荷变体
    let tokens = tokenize("{ some(Int) | none }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    match result.unwrap() {
        crate::frontend::parser::ast::Type::Variant(variants) => {
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].name, "some");
            assert_eq!(variants[1].name, "none");
            // some 有载荷，none 没有
            assert_eq!(variants[0].params.len(), 1);
            assert_eq!(variants[1].params.len(), 0);
        }
        _ => panic!("Expected Variant type"),
    }
}

#[test]
fn test_parse_generic_struct_rfc010() {
    // 泛型结构体：type Box[T] = { value: T }
    let tokens = tokenize("{ value: Int }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    match result.unwrap() {
        crate::frontend::parser::ast::Type::Struct(fields) => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].0, "value");
        }
        _ => panic!("Expected Struct type"),
    }
}

#[test]
fn test_parse_enum_single_variant_rfc010() {
    // 单变体枚举
    let tokens = tokenize("{ single }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    match result.unwrap() {
        crate::frontend::parser::ast::Type::Variant(variants) => {
            assert_eq!(variants.len(), 1);
            assert_eq!(variants[0].name, "single");
        }
        _ => panic!("Expected Variant type"),
    }
}

#[test]
fn test_parse_struct_empty_rfc010() {
    // 空结构体
    let tokens = tokenize("{}").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    match result.unwrap() {
        crate::frontend::parser::ast::Type::Struct(fields) => {
            assert_eq!(fields.len(), 0);
        }
        _ => panic!("Expected Struct type"),
    }
}

#[test]
fn test_parse_enum_empty_rfc010() {
    // 空枚举（虽然不太实用，但应该能解析）
    let tokens = tokenize("{}").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
    match result.unwrap() {
        crate::frontend::parser::ast::Type::Struct(fields) => {
            // 空的花括号解析为结构体，不是枚举
            assert_eq!(fields.len(), 0);
        }
        _ => panic!("Expected Struct type"),
    }
}
