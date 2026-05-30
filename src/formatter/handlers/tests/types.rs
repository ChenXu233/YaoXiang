//! 类型格式化处理器测试
//!
//! 对应 formatter 规范 §9 (type annotations)

use crate::formatter::handlers::types::format_type;
use crate::frontend::core::parser::ast::*;
use crate::util::span::Span;

#[test]
fn test_format_type_int() {
    assert_eq!(format_type(&Type::Int(32)), "i32");
    assert_eq!(format_type(&Type::Int(64)), "i64");
}

#[test]
fn test_format_type_float() {
    assert_eq!(format_type(&Type::Float(32)), "f32");
    assert_eq!(format_type(&Type::Float(64)), "f64");
}

#[test]
fn test_format_type_bool() {
    assert_eq!(format_type(&Type::Bool), "Bool");
}

#[test]
fn test_format_type_string() {
    assert_eq!(format_type(&Type::String), "String");
}

#[test]
fn test_format_type_char() {
    assert_eq!(format_type(&Type::Char), "Char");
}

#[test]
fn test_format_type_void() {
    assert_eq!(format_type(&Type::Void), "()");
}

#[test]
fn test_format_type_tuple() {
    let ty = Type::Tuple(vec![Type::Int(32), Type::Bool]);
    assert_eq!(format_type(&ty), "(i32, Bool)");
}

#[test]
fn test_format_type_option() {
    let ty = Type::Option(Box::new(Type::Int(32)));
    assert_eq!(format_type(&ty), "i32?");
}

#[test]
fn test_format_type_fn() {
    let ty = Type::Fn {
        params: vec![Type::Int(32), Type::Bool],
        return_type: Box::new(Type::String),
    };
    assert_eq!(format_type(&ty), "(i32, Bool) -> String");
}

#[test]
fn test_format_type_ref() {
    let ty = Type::Ref {
        mutable: false,
        inner: Box::new(Type::Int(32)),
        span: Span::dummy(),
    };
    assert_eq!(format_type(&ty), "&i32");
}

#[test]
fn test_format_type_mut_ref() {
    let ty = Type::Ref {
        mutable: true,
        inner: Box::new(Type::Int(32)),
        span: Span::dummy(),
    };
    assert_eq!(format_type(&ty), "&mut i32");
}

#[test]
fn test_format_type_ptr() {
    let ty = Type::Ptr(Box::new(Type::Int(32)));
    assert_eq!(format_type(&ty), "*i32");
}

#[test]
fn test_format_type_name() {
    let ty = Type::Name {
        name: "MyType".to_string(),
        span: Span::dummy(),
    };
    assert_eq!(format_type(&ty), "MyType");
}

#[test]
fn test_format_type_enum() {
    let ty = Type::Enum(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
    assert_eq!(format_type(&ty), "A | B | C");
}

#[test]
fn test_format_type_sum() {
    let ty = Type::Sum(vec![Type::Int(32), Type::Bool]);
    assert_eq!(format_type(&ty), "i32 + Bool");
}
