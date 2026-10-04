//! 模式匹配测试 — 基于语言规范 §4.8
//!
//! §4.8: 模式匹配

use std::collections::HashMap;

use crate::frontend::core::typecheck::inference::patterns::PatternInferrer;
use crate::frontend::core::parser::ast::Pattern;
use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::types::{MonoType, StructType};

fn new_inferrer() -> PatternInferrer {
    PatternInferrer::new()
}

/// 整数字面量模式 `n`。
fn int_pattern(n: i128) -> Pattern {
    Pattern::Literal(Literal::Int(n))
}

/// 结构体模式字段 `name: <整数字面量>`（非 ref 绑定）。
fn int_field(
    name: &str,
    n: i128,
) -> (String, bool, Box<Pattern>) {
    (name.to_string(), false, Box::new(int_pattern(n)))
}

/// `Point { x: Int(64), y: Int(64) }`（字段不可变、无默认值）。
fn point_int_struct() -> MonoType {
    MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MonoType::Int(64)),
            ("y".to_string(), MonoType::Int(64)),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![],
        interfaces: vec![],
    })
}

// Happy path 测试

#[test]
fn test_pattern_infer_wildcard() {
    // Arrange
    let mut inferrer = new_inferrer();

    // Act
    let result = inferrer.infer_pattern(&Pattern::Wildcard).unwrap();

    // Assert
    assert!(
        matches!(result, MonoType::TypeVar(_)),
        "Wildcard 模式应返回 TypeVar，实际得到: {:?}",
        result
    );
}

#[test]
fn test_pattern_infer_int_literal() {
    // Arrange
    let mut inferrer = new_inferrer();

    // Act
    let result = inferrer.infer_pattern(&int_pattern(42)).unwrap();

    // Assert
    assert_eq!(result, MonoType::Int(64), "Int 字面量模式应返回 Int(64)");
}

#[test]
fn test_pattern_infer_float_literal() {
    // Arrange
    let mut inferrer = new_inferrer();

    // Act
    let result = inferrer
        .infer_pattern(&Pattern::Literal(Literal::Float(std::f64::consts::PI)))
        .unwrap();

    // Assert
    assert_eq!(
        result,
        MonoType::Float(64),
        "Float 字面量模式应返回 Float(64)"
    );
}

#[test]
fn test_pattern_infer_bool_literal() {
    // Arrange
    let mut inferrer = new_inferrer();

    // Act
    let result = inferrer
        .infer_pattern(&Pattern::Literal(Literal::Bool(true)))
        .unwrap();

    // Assert
    assert_eq!(result, MonoType::Bool, "Bool 字面量模式应返回 Bool");
}

#[test]
fn test_pattern_infer_string_literal() {
    // Arrange
    let mut inferrer = new_inferrer();

    // Act
    let result = inferrer
        .infer_pattern(&Pattern::Literal(Literal::String("hello".to_string())))
        .unwrap();

    // Assert
    assert_eq!(
        result,
        MonoType::make_string(),
        "String 字面量模式应返回 String"
    );
}

#[test]
fn test_pattern_infer_char_literal() {
    // Arrange
    let mut inferrer = new_inferrer();

    // Act
    let result = inferrer
        .infer_pattern(&Pattern::Literal(Literal::Char('a')))
        .unwrap();

    // Assert
    assert_eq!(result, MonoType::Char, "Char 字面量模式应返回 Char");
}

#[test]
fn test_pattern_infer_identifier() {
    // Arrange
    let mut inferrer = new_inferrer();

    // Act
    let result = inferrer
        .infer_pattern(&Pattern::Identifier("x".to_string()))
        .unwrap();

    // Assert
    assert!(
        matches!(result, MonoType::TypeVar(_)),
        "Identifier 模式应返回 TypeVar，实际得到: {:?}",
        result
    );
}

#[test]
fn test_pattern_infer_tuple() {
    // Arrange
    let mut inferrer = new_inferrer();
    let tuple_pattern = Pattern::Tuple(vec![
        int_pattern(1),
        Pattern::Literal(Literal::String("hello".to_string())),
    ]);

    // Act
    let result = inferrer.infer_pattern(&tuple_pattern).unwrap();

    // Assert
    let expected = MonoType::make_tuple(vec![MonoType::Int(64), MonoType::make_string()]);
    assert_eq!(
        result, expected,
        "Tuple([Int, String]) 应返回 Tuple([Int(64), String])"
    );
}

#[test]
fn test_pattern_infer_struct() {
    // Arrange
    let mut inferrer = new_inferrer();
    let struct_pattern = Pattern::Struct {
        name: "Point".to_string(),
        fields: vec![int_field("x", 1), int_field("y", 2)],
    };

    // Act
    let result = inferrer.infer_pattern(&struct_pattern).unwrap();

    // Assert
    let expected = point_int_struct();
    assert_eq!(result, expected, "Struct 模式应正确推断各字段类型");
}

#[test]
fn test_pattern_infer_or() {
    // Arrange
    let mut inferrer = new_inferrer();
    let or_pattern = Pattern::Or(vec![int_pattern(1), int_pattern(2)]);

    // Act
    let result = inferrer.infer_pattern(&or_pattern).unwrap();

    // Assert
    assert_eq!(
        result,
        MonoType::Int(64),
        "Or 模式应取第一个分支的类型 Int(64)"
    );
}

// Boundary 测试

#[test]
fn test_pattern_infer_empty_or() {
    // Arrange
    let mut inferrer = new_inferrer();
    let empty_or = Pattern::Or(vec![]);

    // Act
    let result = inferrer.infer_pattern(&empty_or).unwrap();

    // Assert
    assert_eq!(result, MonoType::Void, "空 Or 模式应返回 Void");
}

#[test]
fn test_pattern_infer_nested_tuple() {
    // Arrange
    let mut inferrer = new_inferrer();
    let nested = Pattern::Tuple(vec![Pattern::Tuple(vec![int_pattern(1)])]);

    // Act
    let result = inferrer.infer_pattern(&nested).unwrap();

    // Assert
    let expected = MonoType::make_tuple(vec![MonoType::make_tuple(vec![MonoType::Int(64)])]);
    assert_eq!(result, expected, "嵌套 Tuple 应递归推断内部类型");
}
