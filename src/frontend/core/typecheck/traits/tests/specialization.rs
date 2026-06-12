//! 特化测试 — 基于语言规范 §3.15 & RFC-011 §6
//!
//! §3.15: 函数重载与特化
//! RFC-011 §6: 函数重载特化

use crate::frontend::core::typecheck::traits::specialization::{
    Specializer, SpecializationAlgorithm, Substituter, Instantiator,
};
use crate::frontend::core::types::{MonoType, PolyType, TypeVar};

// ===================================================================
// 辅助函数
// ===================================================================

/// 创建单参数多态类型 `forall T. T`
fn mono_poly(body: MonoType) -> PolyType {
    PolyType::new(vec![], body)
}

/// 创建单参数多态类型 `forall T. List(T)`
fn list_of_type_var(index: usize) -> PolyType {
    PolyType::new(
        vec![TypeVar::new(index)],
        MonoType::List(Box::new(MonoType::TypeVar(TypeVar::new(index)))),
    )
}

/// 创建多参数多态类型 `forall T, U. Dict(T, U)`
fn dict_type_vars() -> PolyType {
    PolyType::new(
        vec![TypeVar::new(0), TypeVar::new(1)],
        MonoType::Dict(
            Box::new(MonoType::TypeVar(TypeVar::new(0))),
            Box::new(MonoType::TypeVar(TypeVar::new(1))),
        ),
    )
}

/// 创建返回 TypeVar 的函数多态类型 `forall T. Fn(T) -> T`
fn identity_fn_poly() -> PolyType {
    PolyType::new(
        vec![TypeVar::new(0)],
        MonoType::Fn {
            params: vec![MonoType::TypeVar(TypeVar::new(0))],
            return_type: Box::new(MonoType::TypeVar(TypeVar::new(0))),
        },
    )
}

// ===================================================================
// Specializer — Happy path 测试
// ===================================================================

#[test]
fn test_specializer_creation() {
    // Arrange & Act
    let specializer = Specializer::new();

    // Assert
    let _ = specializer;
}

#[test]
fn test_specializer_default() {
    // Arrange & Act
    let specializer = Specializer::default();

    // Assert
    let _ = specializer;
}

#[test]
fn test_specialize_list_type_var_to_int() {
    // Arrange
    let mut specializer = Specializer::new();
    let poly = list_of_type_var(0);

    // Act
    let result = specializer.specialize(&poly, &[MonoType::Int(64)]);

    // Assert
    assert!(result.is_ok(), "List(T) 特化为 List(Int) 应成功");
    assert_eq!(
        result.unwrap(),
        MonoType::List(Box::new(MonoType::Int(64))),
        "特化结果应为 List(Int(64))"
    );
}

#[test]
fn test_specialize_list_type_var_to_string() {
    // Arrange
    let mut specializer = Specializer::new();
    let poly = list_of_type_var(0);

    // Act
    let result = specializer.specialize(&poly, &[MonoType::String]);

    // Assert
    assert!(result.is_ok(), "List(T) 特化为 List(String) 应成功");
    assert_eq!(
        result.unwrap(),
        MonoType::List(Box::new(MonoType::String)),
        "特化结果应为 List(String)"
    );
}

#[test]
fn test_specialize_dict_two_type_vars() {
    // Arrange
    let mut specializer = Specializer::new();
    let poly = dict_type_vars();

    // Act
    let result = specializer.specialize(&poly, &[MonoType::String, MonoType::Int(64)]);

    // Assert
    assert!(result.is_ok(), "Dict(T, U) 特化应成功");
    assert_eq!(
        result.unwrap(),
        MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(64))),
        "特化结果应为 Dict(String, Int(64))"
    );
}

#[test]
fn test_specialize_identity_fn() {
    // Arrange
    let mut specializer = Specializer::new();
    let poly = identity_fn_poly();

    // Act
    let result = specializer.specialize(&poly, &[MonoType::Bool]);

    // Assert
    assert!(result.is_ok(), "Fn(T)->T 特化为 Fn(Bool)->Bool 应成功");
    let expected = MonoType::Fn {
        params: vec![MonoType::Bool],
        return_type: Box::new(MonoType::Bool),
    };
    assert_eq!(result.unwrap(), expected, "特化结果应为 Fn(Bool) -> Bool");
}

#[test]
fn test_specialize_mono_type_no_binders() {
    // Arrange
    let mut specializer = Specializer::new();
    let poly = mono_poly(MonoType::Int(32));

    // Act — 传入空参数，type_binders 也为空 => 匹配
    let result = specializer.specialize(&poly, &[]);

    // Assert
    assert!(result.is_ok(), "单态类型特化应成功");
    assert_eq!(result.unwrap(), MonoType::Int(32), "单态类型应原样返回");
}

// ===================================================================
// Specializer — can_specialize 测试
// ===================================================================

#[test]
fn test_can_specialize_matching_arity() {
    // Arrange
    let specializer = Specializer::new();
    let poly = list_of_type_var(0);

    // Act & Assert
    assert!(
        specializer.can_specialize(&poly, &[MonoType::Int(32)]),
        "1 个 binder + 1 个 arg 应可特化"
    );
}

#[test]
fn test_can_specialize_two_binders() {
    // Arrange
    let specializer = Specializer::new();
    let poly = dict_type_vars();

    // Act & Assert
    assert!(
        specializer.can_specialize(&poly, &[MonoType::String, MonoType::Int(32)]),
        "2 个 binder + 2 个 arg 应可特化"
    );
}

#[test]
fn test_can_specialize_empty_args_returns_false() {
    // Arrange
    let specializer = Specializer::new();
    let poly = list_of_type_var(0);

    // Act & Assert
    assert!(
        !specializer.can_specialize(&poly, &[]),
        "有 binder 但无 arg 时应返回 false"
    );
}

#[test]
fn test_can_specialize_mono_no_binders_no_args() {
    // Arrange
    let specializer = Specializer::new();
    let poly = mono_poly(MonoType::Int(32));

    // Act — can_specialize 要求 !args.is_empty()，所以空 args 返回 false
    let result = specializer.can_specialize(&poly, &[]);

    // Assert
    assert!(
        !result,
        "无 binder 且无 arg 时 can_specialize 应返回 false（!args.is_empty()）"
    );
}

// ===================================================================
// Specializer — Error path 测试
// ===================================================================

#[test]
fn test_specialize_arity_mismatch_too_few_args() {
    // Arrange
    let mut specializer = Specializer::new();
    let poly = dict_type_vars();

    // Act — 2 个 binder 但只传 1 个 arg
    let result = specializer.specialize(&poly, &[MonoType::String]);

    // Assert
    assert!(result.is_err(), "参数数量不足时应返回 Err");
    let err = result.unwrap_err();
    assert_eq!(err.code, "E1060", "错误码应为 E1060（类型参数数量不匹配）");
}

#[test]
fn test_specialize_arity_mismatch_too_many_args() {
    // Arrange
    let mut specializer = Specializer::new();
    let poly = list_of_type_var(0);

    // Act — 1 个 binder 但传 2 个 arg
    let result = specializer.specialize(&poly, &[MonoType::Int(32), MonoType::Bool]);

    // Assert
    assert!(result.is_err(), "参数过多时应返回 Err");
    let err = result.unwrap_err();
    assert_eq!(err.code, "E1060", "错误码应为 E1060（类型参数数量不匹配）");
}

// ===================================================================
// Specializer — Boundary 测试
// ===================================================================

#[test]
fn test_specialize_with_nested_list_type_var() {
    // Arrange
    let mut specializer = Specializer::new();
    // forall T. List(List(T))
    let poly = PolyType::new(
        vec![TypeVar::new(0)],
        MonoType::List(Box::new(MonoType::List(Box::new(MonoType::TypeVar(
            TypeVar::new(0),
        ))))),
    );

    // Act
    let result = specializer.specialize(&poly, &[MonoType::Bool]);

    // Assert
    assert!(result.is_ok(), "嵌套 List 特化应成功");
    assert_eq!(
        result.unwrap(),
        MonoType::List(Box::new(MonoType::List(Box::new(MonoType::Bool)))),
        "应递归替换内层 TypeVar"
    );
}

#[test]
fn test_specialize_preserves_non_type_var_fields() {
    // Arrange
    let mut specializer = Specializer::new();
    // forall T. Tuple(Int, T, String)
    let poly = PolyType::new(
        vec![TypeVar::new(0)],
        MonoType::Tuple(vec![
            MonoType::Int(32),
            MonoType::TypeVar(TypeVar::new(0)),
            MonoType::String,
        ]),
    );

    // Act
    let result = specializer.specialize(&poly, &[MonoType::Float(64)]);

    // Assert
    assert!(result.is_ok(), "元组中混合具体类型和 TypeVar 的特化应成功");
    assert_eq!(
        result.unwrap(),
        MonoType::Tuple(vec![
            MonoType::Int(32),
            MonoType::Float(64),
            MonoType::String
        ]),
        "只应替换 TypeVar，保留具体类型"
    );
}

#[test]
fn test_specialize_reuses_specializer() {
    // Arrange
    let mut specializer = Specializer::new();
    let poly = list_of_type_var(0);

    // Act — 连续两次特化，验证状态正确清除
    let result1 = specializer.specialize(&poly, &[MonoType::Int(32)]);
    let result2 = specializer.specialize(&poly, &[MonoType::String]);

    // Assert
    assert!(result1.is_ok(), "第一次特化应成功");
    assert!(result2.is_ok(), "第二次特化应成功");
    assert_eq!(
        result1.unwrap(),
        MonoType::List(Box::new(MonoType::Int(32)))
    );
    assert_eq!(result2.unwrap(), MonoType::List(Box::new(MonoType::String)));
}

// ===================================================================
// SpecializationAlgorithm — Happy path 测试
// ===================================================================

#[test]
fn test_algorithm_creation() {
    // Arrange & Act
    let algorithm = SpecializationAlgorithm::new();

    // Assert
    let _ = algorithm;
}

#[test]
fn test_algorithm_default() {
    // Arrange & Act
    let algorithm = SpecializationAlgorithm::default();

    // Assert
    let _ = algorithm;
}

#[test]
fn test_algorithm_specialize_list() {
    // Arrange
    let mut algorithm = SpecializationAlgorithm::new();
    let poly = list_of_type_var(0);

    // Act
    let result = algorithm.specialize(&poly, &[MonoType::String]);

    // Assert
    assert!(result.is_ok(), "算法层 List(T) 特化应成功");
    assert_eq!(
        result.unwrap(),
        MonoType::List(Box::new(MonoType::String)),
        "特化结果应为 List(String)"
    );
}

#[test]
fn test_algorithm_can_specialize() {
    // Arrange
    let algorithm = SpecializationAlgorithm::new();
    let poly = list_of_type_var(0);

    // Act & Assert
    assert!(
        algorithm.can_specialize(&poly, &[MonoType::Int(32)]),
        "匹配时应返回 true"
    );
    assert!(
        !algorithm.can_specialize(&poly, &[]),
        "无参数时应返回 false"
    );
}

// ===================================================================
// Substituter — Happy path 测试
// ===================================================================

#[test]
fn test_substituter_creation() {
    // Arrange & Act
    let substituter = Substituter::new();

    // Assert
    let _ = substituter;
}

#[test]
fn test_substituter_substitute_type_var() {
    // Arrange
    let substituter = Substituter::new();
    let ty = MonoType::TypeVar(TypeVar::new(0));

    // Act
    let result = substituter.substitute(&ty, &TypeVar::new(0), &MonoType::Int(64));

    // Assert
    assert!(result.is_ok(), "替换 TypeVar 应成功");
    let sub = result.unwrap();
    assert!(sub.success, "替换应标记为成功");
    assert_eq!(
        sub.substituted,
        MonoType::Int(64),
        "TypeVar(0) 应被替换为 Int(64)"
    );
}

#[test]
fn test_substituter_substitute_no_match() {
    // Arrange
    let substituter = Substituter::new();
    let ty = MonoType::TypeVar(TypeVar::new(1));

    // Act — 替换 TypeVar(0)，但类型是 TypeVar(1)
    let result = substituter.substitute(&ty, &TypeVar::new(0), &MonoType::String);

    // Assert
    assert!(result.is_ok(), "不匹配的替换应成功（无变化）");
    let sub = result.unwrap();
    assert_eq!(
        sub.substituted,
        MonoType::TypeVar(TypeVar::new(1)),
        "不匹配时应保持原样"
    );
}

#[test]
fn test_substituter_substitute_in_list() {
    // Arrange
    let substituter = Substituter::new();
    let ty = MonoType::List(Box::new(MonoType::TypeVar(TypeVar::new(0))));

    // Act
    let result = substituter.substitute(&ty, &TypeVar::new(0), &MonoType::Bool);

    // Assert
    assert!(result.is_ok(), "在 List 内替换 TypeVar 应成功");
    assert_eq!(
        result.unwrap().substituted,
        MonoType::List(Box::new(MonoType::Bool)),
        "应递归替换 List 内的 TypeVar"
    );
}

#[test]
fn test_substituter_substitute_batch() {
    // Arrange
    let substituter = Substituter::new();
    let ty = MonoType::Tuple(vec![
        MonoType::TypeVar(TypeVar::new(0)),
        MonoType::TypeVar(TypeVar::new(1)),
    ]);
    let subs = vec![
        (TypeVar::new(0), MonoType::Int(32)),
        (TypeVar::new(1), MonoType::String),
    ];

    // Act
    let result = substituter.substitute_batch(&ty, &subs);

    // Assert
    assert!(result.is_ok(), "批量替换应成功");
    assert_eq!(
        result.unwrap(),
        MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]),
        "两个 TypeVar 应被分别替换"
    );
}

#[test]
fn test_substituter_substitute_batch_empty() {
    // Arrange
    let substituter = Substituter::new();
    let ty = MonoType::Int(64);

    // Act
    let result = substituter.substitute_batch(&ty, &[]);

    // Assert
    assert!(result.is_ok(), "空替换列表应成功");
    assert_eq!(result.unwrap(), MonoType::Int(64), "无替换时应保持原样");
}

// ===================================================================
// Substituter — Boundary 测试
// ===================================================================

#[test]
fn test_substituter_substitute_concrete_type_unchanged() {
    // Arrange
    let substituter = Substituter::new();
    let ty = MonoType::Int(32);

    // Act — 对具体类型执行替换，应无变化
    let result = substituter.substitute(&ty, &TypeVar::new(0), &MonoType::String);

    // Assert
    assert!(result.is_ok(), "对具体类型替换应成功");
    assert_eq!(
        result.unwrap().substituted,
        MonoType::Int(32),
        "具体类型不应被替换"
    );
}

#[test]
fn test_substituter_batch_preserves_order() {
    // Arrange
    let substituter = Substituter::new();
    // 先替换 0 -> Bool，再替换 1 -> String
    // 如果先替换 1 再替换 0，结果应相同（因为替换的是不同变量）
    let ty = MonoType::Tuple(vec![
        MonoType::TypeVar(TypeVar::new(0)),
        MonoType::TypeVar(TypeVar::new(1)),
    ]);

    // Act
    let result = substituter.substitute_batch(
        &ty,
        &[
            (TypeVar::new(0), MonoType::Bool),
            (TypeVar::new(1), MonoType::Float(64)),
        ],
    );

    // Assert
    assert_eq!(
        result.unwrap(),
        MonoType::Tuple(vec![MonoType::Bool, MonoType::Float(64)]),
        "批量替换应按顺序依次应用"
    );
}

// ===================================================================
// Instantiator — Happy path 测试
// ===================================================================

#[test]
fn test_instantiator_creation() {
    // Arrange & Act
    let instantiator = Instantiator::new();

    // Assert
    let _ = instantiator;
}

#[test]
fn test_instantiator_instantiate_list_type_var() {
    // Arrange
    let instantiator = Instantiator::new();
    let generic = MonoType::List(Box::new(MonoType::TypeVar(TypeVar::new(0))));

    // Act
    let result = instantiator.instantiate(&generic, &[MonoType::Int(64)]);

    // Assert
    assert!(result.is_ok(), "实例化 List(T) 应成功");
    let inst = result.unwrap();
    assert_eq!(
        inst.instance,
        MonoType::List(Box::new(MonoType::Int(64))),
        "实例应为 List(Int)"
    );
    assert_eq!(inst.generic, generic, "generic 字段应保存原始类型");
}

#[test]
fn test_instantiator_instantiate_fn_type_var() {
    // Arrange
    let instantiator = Instantiator::new();
    let generic = MonoType::Fn {
        params: vec![MonoType::TypeVar(TypeVar::new(0))],
        return_type: Box::new(MonoType::TypeVar(TypeVar::new(0))),
    };

    // Act
    let result = instantiator.instantiate(&generic, &[MonoType::String]);

    // Assert
    assert!(result.is_ok(), "实例化 Fn(T)->T 应成功");
    let expected = MonoType::Fn {
        params: vec![MonoType::String],
        return_type: Box::new(MonoType::String),
    };
    assert_eq!(
        result.unwrap().instance,
        expected,
        "实例应为 Fn(String)->String"
    );
}

#[test]
fn test_instantiator_instantiate_multiple_args() {
    // Arrange
    let instantiator = Instantiator::new();
    let generic = MonoType::Dict(
        Box::new(MonoType::TypeVar(TypeVar::new(0))),
        Box::new(MonoType::TypeVar(TypeVar::new(1))),
    );

    // Act
    let result = instantiator.instantiate(&generic, &[MonoType::String, MonoType::Bool]);

    // Assert
    assert!(result.is_ok(), "实例化 Dict(T, U) 应成功");
    assert_eq!(
        result.unwrap().instance,
        MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Bool)),
        "实例应为 Dict(String, Bool)"
    );
}

// ===================================================================
// Instantiator — can_instantiate 测试
// ===================================================================

#[test]
fn test_can_instantiate_with_type_var() {
    // Arrange
    let instantiator = Instantiator::new();
    let generic = MonoType::TypeVar(TypeVar::new(0));

    // Act & Assert
    assert!(
        instantiator.can_instantiate(&generic, &[MonoType::Int(32)]),
        "含 TypeVar 且有参数时应可实例化"
    );
}

#[test]
fn test_can_instantiate_concrete_type_false() {
    // Arrange
    let instantiator = Instantiator::new();

    // Act & Assert
    assert!(
        !instantiator.can_instantiate(&MonoType::Int(32), &[MonoType::String]),
        "具体类型即使有参数也不应可实例化"
    );
}

#[test]
fn test_can_instantiate_no_args_false() {
    // Arrange
    let instantiator = Instantiator::new();
    let generic = MonoType::TypeVar(TypeVar::new(0));

    // Act & Assert
    assert!(
        !instantiator.can_instantiate(&generic, &[]),
        "无参数时即使含 TypeVar 也不应可实例化"
    );
}

// ===================================================================
// Instantiator — Error path 测试
// ===================================================================

#[test]
fn test_instantiate_concrete_type_returns_error() {
    // Arrange
    let instantiator = Instantiator::new();

    // Act — 具体类型不含 TypeVar，can_instantiate 返回 false
    let result = instantiator.instantiate(&MonoType::Int(32), &[MonoType::String]);

    // Assert
    assert!(result.is_err(), "对具体类型实例化应返回 Err");
    let err = result.unwrap_err();
    assert_eq!(err.code, "E1061", "错误码应为 E1061（无法实例化泛型类型）");
}

#[test]
fn test_instantiate_no_args_returns_error() {
    // Arrange
    let instantiator = Instantiator::new();
    let generic = MonoType::TypeVar(TypeVar::new(0));

    // Act
    let result = instantiator.instantiate(&generic, &[]);

    // Assert
    assert!(result.is_err(), "无参数实例化应返回 Err");
    let err = result.unwrap_err();
    assert_eq!(err.code, "E1061", "错误码应为 E1061（无法实例化泛型类型）");
}

// ===================================================================
// Instantiator — Boundary 测试
// ===================================================================

#[test]
fn test_instantiate_nested_type_var() {
    // Arrange
    let instantiator = Instantiator::new();
    // List(List(T))
    let generic = MonoType::List(Box::new(MonoType::List(Box::new(MonoType::TypeVar(
        TypeVar::new(0),
    )))));

    // Act
    let result = instantiator.instantiate(&generic, &[MonoType::Float(64)]);

    // Assert
    assert!(result.is_ok(), "嵌套 TypeVar 实例化应成功");
    assert_eq!(
        result.unwrap().instance,
        MonoType::List(Box::new(MonoType::List(Box::new(MonoType::Float(64))))),
        "应递归替换深层嵌套的 TypeVar"
    );
}

#[test]
fn test_instantiate_preserves_generic_reference() {
    // Arrange
    let instantiator = Instantiator::new();
    let generic = MonoType::List(Box::new(MonoType::TypeVar(TypeVar::new(0))));

    // Act
    let result = instantiator
        .instantiate(&generic, &[MonoType::Bool])
        .unwrap();

    // Assert
    assert_eq!(
        result.generic, generic,
        "InstanceResult.generic 应保存原始泛型类型"
    );
    assert_ne!(result.instance, result.generic, "实例与泛型应不同");
}
