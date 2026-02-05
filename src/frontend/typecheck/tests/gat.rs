//! GAT 测试模块 (RFC-011)
//!
//! 测试 Generic Associated Types 功能：
//! - 关联类型检查
//! - 关联类型解析
//! - 关联类型约束

use crate::frontend::core::type_system::MonoType;
use crate::frontend::core::type_system::TypeVar;
use crate::frontend::typecheck::gat::checker::GATChecker;

/// 测试关联类型定义检查
#[test]
fn test_associated_type_defined() {
    let checker = GATChecker::new();

    // Iterator::Item 应该是定义的
    let result = checker.is_associated_type_defined("Iterator", "Item");
    assert!(result);

    // IntoIterator::Item 应该是定义的
    let result = checker.is_associated_type_defined("IntoIterator", "Item");
    assert!(result);
}

/// 测试未定义的关联类型
#[test]
fn test_undefined_associated_type() {
    let checker = GATChecker::new();

    // Clone 没有关联类型
    let result = checker.is_associated_type_defined("Clone", "Item");
    assert!(!result);

    // Debug 没有关联类型
    let result = checker.is_associated_type_defined("Debug", "Item");
    assert!(!result);

    // 不存在的 Trait
    let result = checker.is_associated_type_defined("UnknownTrait", "Item");
    assert!(!result);
}

/// 测试关联类型解析
#[test]
fn test_resolve_associated_type() {
    let checker = GATChecker::new();

    // 解析 Iterator::Item
    let result = checker.resolve_associated_type("Iterator", "Item");
    assert_eq!(result, Some("Iterator::Item".to_string()));

    // 解析不存在的关联类型
    let result = checker.resolve_associated_type("Clone", "Item");
    assert_eq!(result, None);
}

/// 测试 GAT 关联类型检查
#[test]
fn test_check_associated_type() {
    let checker = GATChecker::new();

    // Iterator::Item 应该通过检查
    let result = checker.check_associated_type("Iterator", "Item");
    assert!(result.is_ok());

    // 不存在的关联类型应该失败
    let result = checker.check_associated_type("UnknownTrait", "UnknownAssoc");
    assert!(result.is_err());
}

/// 测试 GAT 检查函数类型
#[test]
fn test_check_gat_fn_type() {
    let checker = GATChecker::new();

    // 函数类型应该能通过 GAT 检查
    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::Int(64)),
        is_async: false,
    };

    let result = checker.check_gat(&fn_type);
    assert!(result.is_ok());
}

/// 测试 GAT 检查结构体类型
#[test]
fn test_check_gat_struct_type() {
    let checker = GATChecker::new();

    // 结构体类型应该能通过 GAT 检查
    let struct_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "TestStruct".to_string(),
        fields: vec![
            ("field1".to_string(), MonoType::Int(32)),
            ("field2".to_string(), MonoType::String),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
    });

    let result = checker.check_gat(&struct_type);
    assert!(result.is_ok());
}

/// 测试包含泛型参数的 GAT
#[test]
fn test_check_gat_with_generic_params() {
    let checker = GATChecker::new();

    // 包含类型变量的函数
    let fn_type = MonoType::Fn {
        params: vec![MonoType::TypeVar(
            crate::frontend::core::type_system::TypeVar::new(0),
        )],
        return_type: Box::new(MonoType::TypeVar(
            crate::frontend::core::type_system::TypeVar::new(0),
        )),
        is_async: false,
    };

    let result = checker.check_gat(&fn_type);
    assert!(result.is_ok());
}

/// 测试关联类型约束检查
#[test]
fn test_check_associated_type_constraints() {
    let checker = GATChecker::new();

    // Iterator::Item 应该没有额外约束
    let result = checker.check_associated_type_constraints("Iterator", "Item");
    assert!(result.is_ok());
}

/// 测试关联类型泛型参数检查
#[test]
fn test_check_associated_type_generics() {
    let checker = GATChecker::new();

    // Iterator::Item 应该使用正确的泛型参数
    let result = checker.check_associated_type_generics("Iterator", "Item");
    assert!(result.is_ok());
}

// ============ 嵌套类型测试 ============

/// 测试 GAT 检查列表类型
#[test]
fn test_check_gat_list_type() {
    let checker = GATChecker::new();

    // List<Int> 应该能通过 GAT 检查
    let list_type = MonoType::List(Box::new(MonoType::Int(32)));
    let result = checker.check_gat(&list_type);
    assert!(result.is_ok());
}

/// 测试 GAT 检查元组类型
#[test]
fn test_check_gat_tuple_type() {
    let checker = GATChecker::new();

    // (Int, String, Bool) 应该能通过 GAT 检查
    let tuple_type = MonoType::Tuple(vec![MonoType::Int(32), MonoType::String, MonoType::Bool]);
    let result = checker.check_gat(&tuple_type);
    assert!(result.is_ok());
}

/// 测试 GAT 检查字典类型
#[test]
fn test_check_gat_dict_type() {
    let checker = GATChecker::new();

    // Dict<String, Int> 应该能通过 GAT 检查
    let dict_type = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(32)));
    let result = checker.check_gat(&dict_type);
    assert!(result.is_ok());
}

/// 测试 GAT 检查集合类型
#[test]
fn test_check_gat_set_type() {
    let checker = GATChecker::new();

    // Set<Int> 应该能通过 GAT 检查
    let set_type = MonoType::Set(Box::new(MonoType::Int(32)));
    let result = checker.check_gat(&set_type);
    assert!(result.is_ok());
}

/// 测试 GAT 检查 Arc 类型
#[test]
fn test_check_gat_arc_type() {
    let checker = GATChecker::new();

    // Arc<Int> 应该能通过 GAT 检查
    let arc_type = MonoType::Arc(Box::new(MonoType::Int(32)));
    let result = checker.check_gat(&arc_type);
    assert!(result.is_ok());
}

/// 测试 GAT 检查 Range 类型
#[test]
fn test_check_gat_range_type() {
    let checker = GATChecker::new();

    // Range<Int> 应该能通过 GAT 检查
    let range_type = MonoType::Range {
        elem_type: Box::new(MonoType::Int(32)),
    };
    let result = checker.check_gat(&range_type);
    assert!(result.is_ok());
}

// ============ 泛型参数检测测试 ============

/// 测试 contains_generic_params - 基本类型
#[test]
fn test_contains_generic_params_basic() {
    let checker = GATChecker::new();

    // 基本类型不包含泛型参数
    assert!(!checker.contains_generic_params(&MonoType::Int(32)));
    assert!(!checker.contains_generic_params(&MonoType::String));
    assert!(!checker.contains_generic_params(&MonoType::Bool));
}

/// 测试 contains_generic_params - 类型变量
#[test]
fn test_contains_generic_params_type_var() {
    let checker = GATChecker::new();

    // 类型变量包含泛型参数
    let type_var = MonoType::TypeVar(TypeVar::new(0));
    assert!(checker.contains_generic_params(&type_var));
}

/// 测试 contains_generic_params - 嵌套泛型
#[test]
fn test_contains_generic_params_nested() {
    let checker = GATChecker::new();

    // List<T> 包含泛型参数
    let list_type = MonoType::List(Box::new(MonoType::TypeVar(TypeVar::new(0))));
    assert!(checker.contains_generic_params(&list_type));

    // Tuple<(T, U)> 包含泛型参数
    let tuple_type = MonoType::Tuple(vec![
        MonoType::TypeVar(TypeVar::new(0)),
        MonoType::TypeVar(TypeVar::new(1)),
    ]);
    assert!(checker.contains_generic_params(&tuple_type));
}

// ============ 关联类型测试 ============

/// 测试 AssocType 访问
#[test]
fn test_assoc_type_access() {
    // 测试 AssocType 的创建和访问
    let assoc_type = MonoType::AssocType {
        host_type: Box::new(MonoType::TypeRef("Iterator".to_string())),
        assoc_name: "Item".to_string(),
        assoc_args: vec![],
    };

    match &assoc_type {
        MonoType::AssocType {
            host_type,
            assoc_name,
            assoc_args,
        } => {
            assert!(matches!(**host_type, MonoType::TypeRef(ref s) if s == "Iterator"));
            assert_eq!(assoc_name, "Item");
            assert!(assoc_args.is_empty());
        }
        _ => panic!("Expected AssocType"),
    }
}

/// 测试带参数的 AssocType
#[test]
fn test_assoc_type_with_args() {
    let assoc_type = MonoType::AssocType {
        host_type: Box::new(MonoType::TypeRef("Container".to_string())),
        assoc_name: "Value".to_string(),
        assoc_args: vec![MonoType::Int(32)],
    };

    match &assoc_type {
        MonoType::AssocType {
            host_type,
            assoc_name,
            assoc_args,
        } => {
            assert_eq!(assoc_name, "Value");
            assert_eq!(assoc_args.len(), 1);
        }
        _ => panic!("Expected AssocType"),
    }
}

// ============ 错误处理测试 ============

/// 测试无效关联类型错误
#[test]
fn test_invalid_associated_type_error() {
    let checker = GATChecker::new();

    // 不存在的 Trait 应该返回错误
    let result = checker.check_associated_type("NonExistentTrait", "SomeType");
    assert!(result.is_err());

    // 验证错误信息
    if let Err(e) = result {
        let diagnostic = format!("{:?}", e);
        assert!(diagnostic.contains("E0801") || diagnostic.contains("not found"));
    }
}

/// 测试检查空容器名称
#[test]
fn test_empty_container_name() {
    let checker = GATChecker::new();

    // 空容器名称应该返回 false
    let result = checker.is_associated_type_defined("", "Item");
    assert!(!result);
}

/// 测试检查空关联类型名称
#[test]
fn test_empty_assoc_name() {
    let checker = GATChecker::new();

    // 空关联类型名称应该返回 false
    let result = checker.is_associated_type_defined("Iterator", "");
    assert!(!result);
}

// ============ 复杂类型结构测试 ============

/// 测试复杂嵌套结构
#[test]
fn test_complex_nested_structure() {
    let checker = GATChecker::new();

    // 创建复杂的嵌套类型
    let complex_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "ComplexStruct".to_string(),
        fields: vec![
            (
                "items".to_string(),
                MonoType::List(Box::new(MonoType::Int(32))),
            ),
            (
                "mapping".to_string(),
                MonoType::Dict(
                    Box::new(MonoType::String),
                    Box::new(MonoType::TypeVar(TypeVar::new(0))),
                ),
            ),
            (
                "processor".to_string(),
                MonoType::Fn {
                    params: vec![MonoType::Int(64)],
                    return_type: Box::new(MonoType::Bool),
                    is_async: false,
                },
            ),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
    });

    let result = checker.check_gat(&complex_type);
    assert!(result.is_ok());
}

/// 测试包含多个类型变量的函数
#[test]
fn test_fn_with_multiple_type_vars() {
    let checker = GATChecker::new();

    let fn_type = MonoType::Fn {
        params: vec![
            MonoType::TypeVar(TypeVar::new(0)),
            MonoType::TypeVar(TypeVar::new(1)),
            MonoType::TypeVar(TypeVar::new(2)),
        ],
        return_type: Box::new(MonoType::TypeVar(TypeVar::new(0))),
        is_async: false,
    };

    let result = checker.check_gat(&fn_type);
    assert!(result.is_ok());
}

// ============ 联合和交集类型测试 ============

/// 测试 GAT 检查联合类型
#[test]
fn test_check_gat_union_type() {
    let checker = GATChecker::new();

    // Int | String 应该能通过 GAT 检查
    let union_type = MonoType::Union(vec![MonoType::Int(32), MonoType::String]);
    let result = checker.check_gat(&union_type);
    assert!(result.is_ok());
}

/// 测试 GAT 检查交集类型
#[test]
fn test_check_gat_intersection_type() {
    let checker = GATChecker::new();

    // Int & String 应该能通过 GAT 检查
    let intersection_type = MonoType::Intersection(vec![MonoType::Int(32), MonoType::String]);
    let result = checker.check_gat(&intersection_type);
    assert!(result.is_ok());
}

// ============ GATChecker 默认实现测试 ============

/// 测试 GATChecker 默认实现
#[test]
fn test_gat_checker_default() {
    let checker = GATChecker::default();
    // 应该能正常工作
    let result = checker.is_associated_type_defined("Iterator", "Item");
    assert!(result);
}

/// 测试 GATChecker 创建方法
#[test]
fn test_gat_checker_new() {
    let checker = GATChecker::new();
    // 应该能正常工作
    let result = checker.resolve_associated_type("IntoIterator", "Item");
    assert_eq!(result, Some("IntoIterator::Item".to_string()));
}

// ============ 更多关联类型场景测试 ============

/// 测试 Provider Trait 的关联类型
#[test]
fn test_provider_associated_type() {
    let checker = GATChecker::new();

    // 假设 Provider::Output 是一个有效的关联类型
    let result = checker.resolve_associated_type("Provider", "Output");
    // 当前实现返回 None，因为不是已知的关联类型
    assert_eq!(result, None);
}

/// 测试默认关联类型
#[test]
fn test_default_associated_types() {
    let checker = GATChecker::new();

    // 测试更多的默认关联类型定义
    // Iterator::Item 和 IntoIterator::Item 是已定义的
    assert!(checker.is_associated_type_defined("Iterator", "Item"));
    assert!(checker.is_associated_type_defined("IntoIterator", "Item"));

    // 其他常见的关联类型
    assert!(!checker.is_associated_type_defined("Iterator", "Value"));
    assert!(!checker.is_associated_type_defined("IntoIterator", "Element"));
}
