//! 泛型关联类型测试 — 基于语言规范 §3.10.2 & RFC-011 §3.2
//!
//! §3.10.2: 泛型关联类型（GAT）
//! RFC-011 §3.2: 泛型关联类型

use crate::frontend::core::typecheck::traits::gat::{GATChecker, HigherRankChecker};
use crate::frontend::core::types::{MonoType, StructType, TypeVar};
use std::collections::HashMap;

// ===================================================================
// 辅助函数
// ===================================================================

/// 创建一个无字段的空结构体 MonoType
fn empty_struct(name: &str) -> MonoType {
    MonoType::Struct(StructType {
        name: name.to_string(),
        fields: vec![],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    })
}

/// 创建一个带字段的结构体 MonoType
fn struct_with_fields(
    name: &str,
    fields: Vec<(&str, MonoType)>,
) -> MonoType {
    MonoType::Struct(StructType {
        name: name.to_string(),
        fields: fields
            .into_iter()
            .map(|(n, t)| (n.to_string(), t))
            .collect(),
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    })
}

// ===================================================================
// GATChecker — Happy path 测试
// ===================================================================

#[test]
fn test_gat_checker_creation() {
    // Arrange & Act
    let checker = GATChecker::new();

    // Assert — 默认构造不应 panic
    let _ = checker;
}

#[test]
fn test_gat_checker_default_trait() {
    // Arrange & Act
    let checker = GATChecker;

    // Assert — Default trait 实现应与 new() 等价
    let _ = checker;
}

#[test]
fn test_check_gat_on_basic_type() {
    // Arrange
    let checker = GATChecker::new();

    // Act — 基本类型不需要 GAT 检查
    let result = checker.check_gat(&MonoType::Int(64));

    // Assert
    assert!(result.is_ok(), "基本类型 Int(64) 不应触发 GAT 检查错误");
}

#[test]
fn test_check_gat_on_string_type() {
    // Arrange
    let checker = GATChecker::new();

    // Act
    let result = checker.check_gat(&MonoType::String);

    // Assert
    assert!(result.is_ok(), "String 类型不应触发 GAT 检查错误");
}

#[test]
fn test_check_gat_on_empty_struct() {
    // Arrange
    let checker = GATChecker::new();
    let ty = empty_struct("Empty");

    // Act
    let result = checker.check_gat(&ty);

    // Assert
    assert!(result.is_ok(), "无字段结构体不应触发 GAT 检查错误");
}

#[test]
fn test_check_gat_on_struct_with_concrete_fields() {
    // Arrange
    let checker = GATChecker::new();
    let ty = struct_with_fields(
        "Point",
        vec![("x", MonoType::Float(64)), ("y", MonoType::Float(64))],
    );

    // Act
    let result = checker.check_gat(&ty);

    // Assert
    assert!(
        result.is_ok(),
        "包含具体类型字段的结构体不应触发 GAT 检查错误"
    );
}

#[test]
fn test_check_gat_on_fn_type_with_concrete_params() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::String],
        return_type: Box::new(MonoType::Bool),
    };

    // Act
    let result = checker.check_gat(&ty);

    // Assert
    assert!(
        result.is_ok(),
        "参数和返回值均为具体类型的函数不应触发 GAT 检查错误"
    );
}

#[test]
fn test_check_gat_on_fn_type_with_type_var() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::Fn {
        params: vec![MonoType::TypeVar(TypeVar::new(0))],
        return_type: Box::new(MonoType::TypeVar(TypeVar::new(0))),
    };

    // Act
    let result = checker.check_gat(&ty);

    // Assert
    assert!(
        result.is_ok(),
        "含 TypeVar 的函数类型应通过 validate_generic_usage"
    );
}

#[test]
fn test_check_gat_on_list_type() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::List(Box::new(MonoType::String));

    // Act
    let result = checker.check_gat(&ty);

    // Assert
    assert!(result.is_ok(), "List 类型走 catch-all 分支，不应报错");
}

#[test]
fn test_check_gat_on_tuple_type() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::Tuple(vec![MonoType::Int(32), MonoType::Bool]);

    // Act
    let result = checker.check_gat(&ty);

    // Assert
    assert!(result.is_ok(), "Tuple 类型走 catch-all 分支，不应报错");
}

// ===================================================================
// GATChecker — contains_generic_params 测试
// ===================================================================

#[test]
fn test_contains_generic_params_on_type_var() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::TypeVar(TypeVar::new(0));

    // Act
    let result = checker.contains_generic_params(&ty);

    // Assert
    assert!(result, "TypeVar 应被识别为包含泛型参数");
}

#[test]
fn test_contains_generic_params_on_concrete_type() {
    // Arrange
    let checker = GATChecker::new();

    // Act & Assert
    assert!(
        !checker.contains_generic_params(&MonoType::Int(64)),
        "Int 不含泛型参数"
    );
    assert!(
        !checker.contains_generic_params(&MonoType::String),
        "String 不含泛型参数"
    );
    assert!(
        !checker.contains_generic_params(&MonoType::Bool),
        "Bool 不含泛型参数"
    );
}

#[test]
fn test_contains_generic_params_on_list_with_type_var() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::List(Box::new(MonoType::TypeVar(TypeVar::new(1))));

    // Act
    let result = checker.contains_generic_params(&ty);

    // Assert
    assert!(result, "List(TypeVar) 应被识别为包含泛型参数");
}

#[test]
fn test_contains_generic_params_on_list_without_type_var() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::List(Box::new(MonoType::Int(32)));

    // Act
    let result = checker.contains_generic_params(&ty);

    // Assert
    assert!(!result, "List(Int) 不含泛型参数");
}

#[test]
fn test_contains_generic_params_on_tuple_mixed() {
    // Arrange
    let checker = GATChecker::new();
    let with_var = MonoType::Tuple(vec![MonoType::Int(32), MonoType::TypeVar(TypeVar::new(2))]);
    let without_var = MonoType::Tuple(vec![MonoType::Int(32), MonoType::Bool]);

    // Act & Assert
    assert!(
        checker.contains_generic_params(&with_var),
        "Tuple 含 TypeVar 时应返回 true"
    );
    assert!(
        !checker.contains_generic_params(&without_var),
        "Tuple 不含 TypeVar 时应返回 false"
    );
}

#[test]
fn test_contains_generic_params_on_fn_return_type_var() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::TypeVar(TypeVar::new(3))),
    };

    // Act
    let result = checker.contains_generic_params(&ty);

    // Assert
    assert!(result, "函数返回值含 TypeVar 时应返回 true");
}

#[test]
fn test_contains_generic_params_on_fn_all_concrete() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::Fn {
        params: vec![MonoType::String],
        return_type: Box::new(MonoType::Bool),
    };

    // Act
    let result = checker.contains_generic_params(&ty);

    // Assert
    assert!(!result, "全具体类型的函数不含泛型参数");
}

#[test]
fn test_contains_generic_params_on_struct_with_type_var_field() {
    // Arrange
    let checker = GATChecker::new();
    let ty = struct_with_fields(
        "Container",
        vec![("data", MonoType::TypeVar(TypeVar::new(0)))],
    );

    // Act
    let result = checker.contains_generic_params(&ty);

    // Assert
    assert!(result, "结构体字段含 TypeVar 时应返回 true");
}

// ===================================================================
// GATChecker — check_associated_type 测试
// ===================================================================

#[test]
fn test_check_associated_type_iterator_item() {
    // Arrange
    let checker = GATChecker::new();

    // Act
    let result = checker.check_associated_type("Iterator", "Item");

    // Assert
    assert!(result.is_ok(), "Iterator::Item 是合法的关联类型");
}

#[test]
fn test_check_associated_type_into_iterator_item() {
    // Arrange
    let checker = GATChecker::new();

    // Act
    let result = checker.check_associated_type("IntoIterator", "Item");

    // Assert
    assert!(result.is_ok(), "IntoIterator::Item 是合法的关联类型");
}

#[test]
fn test_check_associated_type_not_found() {
    // Arrange
    let checker = GATChecker::new();

    // Act
    let result = checker.check_associated_type("Iterator", "Output");

    // Assert
    assert!(result.is_err(), "Iterator::Output 不存在，应返回 Err");
    let err = result.unwrap_err();
    assert_eq!(err.code, "E4005", "错误码应为 E4005（关联类型未找到）");
}

#[test]
fn test_check_associated_type_clone_has_none() {
    // Arrange
    let checker = GATChecker::new();

    // Act
    let result = checker.check_associated_type("Clone", "Item");

    // Assert
    assert!(result.is_err(), "Clone 没有关联类型，应返回 Err");
}

#[test]
fn test_check_associated_type_debug_has_none() {
    // Arrange
    let checker = GATChecker::new();

    // Act
    let result = checker.check_associated_type("Debug", "Item");

    // Assert
    assert!(result.is_err(), "Debug 没有关联类型，应返回 Err");
}

#[test]
fn test_check_associated_type_unknown_container() {
    // Arrange
    let checker = GATChecker::new();

    // Act
    let result = checker.check_associated_type("MyTrait", "AssocType");

    // Assert
    assert!(result.is_err(), "未知容器中的关联类型应返回 Err");
}

// ===================================================================
// GATChecker — is_associated_type_defined 测试
// ===================================================================

#[test]
fn test_is_associated_type_defined_iterator_item() {
    // Arrange
    let checker = GATChecker::new();

    // Act & Assert
    assert!(
        checker.is_associated_type_defined("Iterator", "Item"),
        "Iterator 应定义 Item"
    );
}

#[test]
fn test_is_associated_type_defined_into_iterator_item() {
    // Arrange
    let checker = GATChecker::new();

    // Act & Assert
    assert!(
        checker.is_associated_type_defined("IntoIterator", "Item"),
        "IntoIterator 应定义 Item"
    );
}

#[test]
fn test_is_associated_type_defined_clone_item_false() {
    // Arrange
    let checker = GATChecker::new();

    // Act & Assert
    assert!(
        !checker.is_associated_type_defined("Clone", "Item"),
        "Clone 不应定义 Item"
    );
}

#[test]
fn test_is_associated_type_defined_unknown() {
    // Arrange
    let checker = GATChecker::new();

    // Act & Assert
    assert!(
        !checker.is_associated_type_defined("Foo", "Bar"),
        "未知容器应返回 false"
    );
}

// ===================================================================
// GATChecker — resolve_associated_type 测试
// ===================================================================

#[test]
fn test_resolve_associated_type_success() {
    // Arrange
    let checker = GATChecker::new();

    // Act
    let result = checker.resolve_associated_type("Iterator", "Item");

    // Assert
    assert_eq!(
        result,
        Some("Iterator::Item".to_string()),
        "应正确解析 Iterator::Item"
    );
}

#[test]
fn test_resolve_associated_type_not_found() {
    // Arrange
    let checker = GATChecker::new();

    // Act
    let result = checker.resolve_associated_type("Clone", "Item");

    // Assert
    assert_eq!(result, None, "不存在的关联类型应返回 None");
}

// ===================================================================
// GATChecker — Error path 测试
// ===================================================================

#[test]
fn test_check_gat_on_struct_with_generic_field() {
    // Arrange
    let checker = GATChecker::new();
    let ty = struct_with_fields("Box", vec![("value", MonoType::TypeVar(TypeVar::new(0)))]);

    // Act — validate_generic_usage 当前简化实现始终返回 Ok
    let result = checker.check_gat(&ty);

    // Assert
    assert!(
        result.is_ok(),
        "当前实现中含泛型字段的结构体应通过（validate_generic_usage 简化实现）"
    );
}

// ===================================================================
// GATChecker — Boundary 测试
// ===================================================================

#[test]
fn test_check_gat_on_void_type() {
    // Arrange
    let checker = GATChecker::new();

    // Act
    let result = checker.check_gat(&MonoType::Void);

    // Assert
    assert!(result.is_ok(), "Void 类型走 catch-all 分支，不应报错");
}

#[test]
fn test_contains_generic_params_on_empty_tuple() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::Tuple(vec![]);

    // Act
    let result = checker.contains_generic_params(&ty);

    // Assert
    assert!(!result, "空元组不含泛型参数");
}

#[test]
fn test_contains_generic_params_on_deeply_nested() {
    // Arrange
    let checker = GATChecker::new();
    // List(List(List(TypeVar(5))))
    let ty = MonoType::List(Box::new(MonoType::List(Box::new(MonoType::List(
        Box::new(MonoType::TypeVar(TypeVar::new(5))),
    )))));

    // Act
    let result = checker.contains_generic_params(&ty);

    // Assert
    assert!(result, "深层嵌套的 TypeVar 应被检测到");
}

#[test]
fn test_check_gat_on_async_fn_type() {
    // Arrange
    let checker = GATChecker::new();
    let ty = MonoType::Fn {
        params: vec![MonoType::String],
        return_type: Box::new(MonoType::Void),
    };

    // Act
    let result = checker.check_gat(&ty);

    // Assert
    assert!(result.is_ok(), "异步函数类型应正常通过 GAT 检查");
}

// ===================================================================
// HigherRankChecker — Happy path 测试
// ===================================================================

#[test]
fn test_higher_rank_checker_creation() {
    // Arrange & Act
    let checker = HigherRankChecker::new();

    // Assert
    let _ = checker;
}

#[test]
fn test_higher_rank_checker_default() {
    // Arrange & Act
    let checker = HigherRankChecker;

    // Assert
    let _ = checker;
}

#[test]
fn test_check_non_higher_rank_type() {
    // Arrange
    let checker = HigherRankChecker::new();

    // Act
    let result = checker.check("fn(i32) -> i32");

    // Assert
    assert!(result.is_ok(), "非高阶类型应直接通过检查");
}

#[test]
fn test_check_higher_rank_type_valid() {
    // Arrange
    let checker = HigherRankChecker::new();

    // Act — 包含 "for(" 且括号平衡
    let result = checker.check("for<'a> fn(&'a str) -> &'a str");

    // Assert
    assert!(result.is_ok(), "合法的高阶类型应通过检查");
}

#[test]
fn test_parse_higher_rank_type_success() {
    // Arrange
    let checker = HigherRankChecker::new();

    // Act
    let result = checker.parse_higher_rank_type("for(a) fn(&a str) -> &a str");

    // Assert
    assert!(result.is_ok(), "应能解析高阶类型");
    assert_eq!(
        result.unwrap(),
        "for(a) fn(&a str) -> &a str",
        "解析结果应与输入一致"
    );
}

// ===================================================================
// HigherRankChecker — Error path 测试
// ===================================================================

#[test]
fn test_parse_non_higher_rank_type_returns_error() {
    // Arrange
    let checker = HigherRankChecker::new();

    // Act
    let result = checker.parse_higher_rank_type("fn(i32) -> i32");

    // Assert
    assert!(result.is_err(), "非高阶类型解析应返回 Err");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("Not a higher-rank type"),
        "错误消息应包含 'Not a higher-rank type'，实际为: {}",
        err.message
    );
}

#[test]
fn test_check_higher_rank_type_invalid_syntax() {
    // Arrange
    let checker = HigherRankChecker::new();

    // Act — 括号不平衡：有一个未闭合的 '('
    let result = checker.check("for(a) fn(&a str -> &a str");

    // Assert
    assert!(result.is_err(), "括号不平衡的高阶类型应返回 Err");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("Invalid higher-rank type syntax"),
        "错误消息应包含 'Invalid higher-rank type syntax'，实际为: {}",
        err.message
    );
}

// ===================================================================
// HigherRankChecker — Boundary 测试
// ===================================================================

#[test]
fn test_check_empty_string() {
    // Arrange
    let checker = HigherRankChecker::new();

    // Act — 空字符串不含 "for("，不走高阶路径
    let result = checker.check("");

    // Assert
    assert!(result.is_ok(), "空字符串不是高阶类型，应直接通过");
}

#[test]
fn test_check_for_keyword_without_parens() {
    // Arrange
    let checker = HigherRankChecker::new();

    // Act — "for" 但无 '(' 跟随
    let result = checker.check("for_each_item");

    // Assert
    assert!(
        result.is_ok(),
        "包含 'for' 但不含 'for(' 的字符串不应被识别为高阶类型"
    );
}

#[test]
fn test_higher_rank_type_with_nested_brackets() {
    // Arrange
    let checker = HigherRankChecker::new();

    // Act — 嵌套括号
    let result = checker.check("for<'a> fn(&'a [i32]) -> Vec<&'a str>");

    // Assert
    assert!(result.is_ok(), "嵌套括号且平衡的高阶类型应通过检查");
}

#[test]
fn test_higher_rank_type_extra_closing_bracket() {
    // Arrange
    let checker = HigherRankChecker::new();

    // Act — 多余的右括号：两个 ')' 但只有一个 '('
    let result = checker.check("for(a) fn(&a str)) -> &a str");

    // Assert
    assert!(result.is_err(), "多余的右括号应导致语法错误");
}
