//! 泛型推断测试 — 基于语言规范 §3.8 & RFC-011 §1
//!
//! §3.8: 泛型类型
//! RFC-011 §1: 基础泛型

use crate::frontend::core::typecheck::inference::generics::GenericInferrer;
use crate::frontend::core::types::base::mono::StructType;
use crate::frontend::core::types::base::MonoType;
use crate::util::span::Span;
use std::collections::HashMap;

// ===================================================================
// Happy path 测试
// ===================================================================

/// §3.8 / RFC-011 §1: 单泛型参数函数推断 — 为每个 type_param 分配一个 fresh TypeVar，
/// 最终返回的也是一个 fresh TypeVar
#[test]
fn test_infer_generic_function_single_param() {
    // Arrange
    let mut inferrer = GenericInferrer::new();

    // Act
    let result = inferrer
        .infer_generic_function("f", &["T".to_string()])
        .expect("infer_generic_function 单参数应成功");

    // Assert
    match result {
        MonoType::TypeVar(_) => {}
        other => panic!("期望 TypeVar，实际得到 {:?}", other),
    }
}

/// §3.8 / RFC-011 §1: 连续调用两次 infer_generic_function 应返回不同的 TypeVar
#[test]
fn test_infer_generic_function_fresh_vars() {
    // Arrange
    let mut inferrer = GenericInferrer::new();

    // Act
    let t1 = inferrer
        .infer_generic_function("f", &["T".to_string()])
        .expect("第一次调用应成功");
    let t2 = inferrer
        .infer_generic_function("g", &["U".to_string()])
        .expect("第二次调用应成功");

    // Assert
    match (&t1, &t2) {
        (MonoType::TypeVar(v1), MonoType::TypeVar(v2)) => {
            assert_ne!(v1, v2, "连续调用应产生不同的 TypeVar");
        }
        _ => panic!("期望两个 TypeVar，实际得到 {:?} 和 {:?}", t1, t2),
    }
}

/// §3.8 / RFC-011 §1: 单参数实例化
///
/// 规范定义：`List(Int)` 应返回泛型实例化类型（如 `GenericType("List", [Int(32)])`），
/// 而非 `args[0]` 本身。当前实现简化为返回 args[0]，这是代码待修复的行为。
/// 测试按当前实现断言，但标注为待修复。
#[test]
fn test_infer_generic_instantiation_single_arg() {
    // Arrange
    let mut inferrer = GenericInferrer::new();
    let int_type = MonoType::Int(32);

    // Act
    let result = inferrer
        .infer_generic_instantiation("List", std::slice::from_ref(&int_type))
        .expect("单参数实例化应成功");

    // Assert
    // TODO(代码待修复): 规范 §3.8 定义 List(Int) 应返回泛型实例化类型，
    // 当前实现简化为返回 args[0]。待实现 GenericType 后需修正此测试。
    assert_eq!(
        result, int_type,
        "当前实现：单参数实例化原样返回传入的类型（待修复为泛型实例化类型）"
    );
}

/// §3.8 / RFC-011 §1: 多参数实例化
///
/// 规范定义：`Map(Int, String)` 应返回泛型实例化类型（如 `GenericType("Map", [Int(32), String])`），
/// 而非 fresh TypeVar。当前实现简化为返回 TypeVar，这是代码待修复的行为。
/// 测试按当前实现断言，但标注为待修复。
#[test]
fn test_infer_generic_instantiation_multi_args() {
    // Arrange
    let mut inferrer = GenericInferrer::new();
    let args = vec![MonoType::Int(32), MonoType::String];

    // Act
    let result = inferrer
        .infer_generic_instantiation("Map", &args)
        .expect("多参数实例化应成功");

    // Assert
    // TODO(代码待修复): 规范 §3.8 定义 Map(Int, String) 应返回泛型实例化类型，
    // 当前实现简化为返回 fresh TypeVar。待实现 GenericType 后需修正此测试。
    match result {
        MonoType::TypeVar(_) => {}
        other => panic!(
            "当前实现：多参数实例化应返回 TypeVar（待修复为泛型实例化类型），实际得到 {:?}",
            other
        ),
    }
}

// ===================================================================
// Error path 测试
// ===================================================================

/// §3.8 / RFC-011 §1: 类型不满足约束时 check_type_constraint 应返回 Err
#[test]
fn test_check_type_constraint_violation() {
    // Arrange
    let mut inferrer = GenericInferrer::new();
    let span = Span::dummy();
    // Clone 接口：要求 clone(self) -> Self 方法
    let clone_iface = MonoType::Struct(StructType {
        name: "Clone".to_string(),
        fields: vec![(
            "clone".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Self".to_string())],
                return_type: Box::new(MonoType::TypeRef("Self".to_string())),
            },
        )],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    // Void 类型没有 clone 方法，应该不满足 Clone 约束
    let void_type = MonoType::Void;

    // Act
    let result = inferrer.check_type_constraint(&void_type, &clone_iface, span, None);

    // Assert
    assert!(result.is_err(), "Void 类型不满足 Clone 约束时应返回 Err");
}

// ===================================================================
// Boundary 测试
// ===================================================================

/// §3.8 / RFC-011 §1: 空约束列表应直接返回 Ok
#[test]
fn test_infer_generic_constraints_empty() {
    // Arrange
    let mut inferrer = GenericInferrer::new();

    // Act
    let result = inferrer.infer_generic_constraints(&[]);

    // Assert
    assert!(result.is_ok(), "空约束列表应返回 Ok，实际得到 {:?}", result);
}

/// §3.8 / RFC-011 §1: 类型满足约束时 check_type_constraint 应返回 Ok
/// — StructType 带有匹配的 clone 字段应通过约束检查
#[test]
fn test_check_type_constraint_satisfied() {
    // Arrange
    let mut inferrer = GenericInferrer::new();
    let span = Span::dummy();

    // Clone 接口
    let clone_iface = MonoType::Struct(StructType {
        name: "Clone".to_string(),
        fields: vec![(
            "clone".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Self".to_string())],
                return_type: Box::new(MonoType::TypeRef("Self".to_string())),
            },
        )],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });

    // 构造一个带有 clone 字段的结构体，使其满足 Clone 约束
    let cloneable_struct = MonoType::Struct(StructType {
        name: "MyStruct".to_string(),
        fields: vec![(
            "clone".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Self".to_string())],
                return_type: Box::new(MonoType::TypeRef("Self".to_string())),
            },
        )],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });

    // Act
    let result = inferrer.check_type_constraint(&cloneable_struct, &clone_iface, span, None);

    // Assert
    assert!(
        result.is_ok(),
        "带 clone 方法的结构体应满足 Clone 约束，实际得到 {:?}",
        result
    );
}
