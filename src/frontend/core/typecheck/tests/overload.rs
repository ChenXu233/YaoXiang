//! 重载解析测试 — 基于语言规范 §3.15 & RFC-011 §6
//!
//! §3.15: 函数重载与特化
//! RFC-011 §6: 函数重载特化

use crate::frontend::core::typecheck::overload::{OverloadCandidate, OverloadResolver};
use crate::frontend::core::types::base::{MonoType, TypeVar};

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_overload_candidate_creation() {
    // Arrange
    let name = "add".to_string();
    let param_types = vec![MonoType::Int(32), MonoType::Int(32)];
    let return_type = MonoType::Int(32);

    // Act
    let candidate = OverloadCandidate::new(
        name.clone(),
        param_types.clone(),
        return_type.clone(),
        vec![],
    );

    // Assert
    assert_eq!(candidate.name, name, "候选名称应为 'add'");
    assert_eq!(
        candidate.param_types, param_types,
        "参数类型列表应与构造时一致"
    );
    assert_eq!(candidate.return_type, return_type, "返回类型应与构造时一致");
    assert!(!candidate.is_generic, "无泛型参数时 is_generic 应为 false");
}

#[test]
fn test_overload_candidate_creation_generic() {
    // Arrange & Act
    let candidate = OverloadCandidate::new(
        "identity".to_string(),
        vec![MonoType::TypeVar(TypeVar::new(0))],
        MonoType::TypeVar(TypeVar::new(0)),
        vec!["T".to_string()],
    );

    // Assert
    assert!(candidate.is_generic, "有泛型参数时 is_generic 应为 true");
    assert_eq!(
        candidate.type_params,
        vec!["T".to_string()],
        "type_params 应包含 'T'"
    );
}

#[test]
fn test_overload_resolver_creation() {
    // Arrange & Act
    let resolver = OverloadResolver::new();

    // Assert
    assert_eq!(resolver.candidate_count(), 0, "新建解析器候选数应为 0");
}

#[test]
fn test_overload_resolver_add_candidate() {
    // Arrange
    let mut resolver = OverloadResolver::new();
    let candidate = OverloadCandidate::new(
        "add".to_string(),
        vec![MonoType::Int(32), MonoType::Int(32)],
        MonoType::Int(32),
        vec![],
    );

    // Act
    resolver.add_candidate(candidate);

    // Assert
    assert_eq!(resolver.candidate_count(), 1, "添加一个候选后计数应为 1");
    let indices = resolver.get_candidates("add");
    assert!(indices.is_some(), "应能通过名称 'add' 查找到候选");
    assert_eq!(indices.unwrap().len(), 1, "'add' 的候选索引数应为 1");
}

#[test]
fn test_resolve_exact_match() {
    // Arrange: 添加 int + int 和 float + float 两个重载
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "add".to_string(),
        vec![MonoType::Int(32), MonoType::Int(32)],
        MonoType::Int(32),
        vec![],
    ));
    resolver.add_candidate(OverloadCandidate::new(
        "add".to_string(),
        vec![MonoType::Float(64), MonoType::Float(64)],
        MonoType::Float(64),
        vec![],
    ));

    // Act: 使用 Int(32) 参数调用
    let result = resolver.resolve("add", &[MonoType::Int(32), MonoType::Int(32)]);

    // Assert: 应精确匹配到 Int 版本
    assert!(result.is_ok(), "resolve 应成功返回匹配候选");
    let candidate = result.unwrap();
    assert_eq!(
        candidate.param_types,
        vec![MonoType::Int(32), MonoType::Int(32)],
        "应匹配到参数类型为 [Int(32), Int(32)] 的候选"
    );
    assert_eq!(
        candidate.return_type,
        MonoType::Int(32),
        "返回类型应为 Int(32)"
    );
}

#[test]
fn test_resolve_exact_match_float() {
    // Arrange
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "add".to_string(),
        vec![MonoType::Int(32), MonoType::Int(32)],
        MonoType::Int(32),
        vec![],
    ));
    resolver.add_candidate(OverloadCandidate::new(
        "add".to_string(),
        vec![MonoType::Float(64), MonoType::Float(64)],
        MonoType::Float(64),
        vec![],
    ));

    // Act: 使用 Float(64) 参数调用
    let result = resolver.resolve("add", &[MonoType::Float(64), MonoType::Float(64)]);

    // Assert: 应精确匹配到 Float 版本
    assert!(result.is_ok(), "resolve 应成功返回匹配候选");
    let candidate = result.unwrap();
    assert_eq!(
        candidate.param_types,
        vec![MonoType::Float(64), MonoType::Float(64)],
        "应匹配到参数类型为 [Float(64), Float(64)] 的候选"
    );
    assert_eq!(
        candidate.return_type,
        MonoType::Float(64),
        "返回类型应为 Float(64)"
    );
}

#[test]
fn test_resolve_type_compatible_match() {
    // Arrange: 添加一个使用类型变量的泛型候选
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "identity".to_string(),
        vec![MonoType::TypeVar(TypeVar::new(0))],
        MonoType::TypeVar(TypeVar::new(0)),
        vec!["T".to_string()],
    ));

    // Act: 传入 Int(32)，泛型候选应兼容匹配（规范 §3.15.1: 泛型匹配优先级 3）
    let result = resolver.resolve("identity", &[MonoType::Int(32)]);

    // Assert
    assert!(result.is_ok(), "TypeVar 参数应兼容匹配任意具体类型");
    let candidate = result.unwrap();
    assert_eq!(
        candidate.param_types,
        vec![MonoType::TypeVar(TypeVar::new(0))],
        "匹配的候选应使用 TypeVar 参数"
    );
    assert!(candidate.is_generic, "匹配的候选应标记为泛型");
}

#[test]
fn test_overload_resolver_with_many_candidates() {
    // Arrange: 添加 100 个同名但参数类型不同的候选
    let mut resolver = OverloadResolver::new();
    for i in 0..100usize {
        resolver.add_candidate(OverloadCandidate::new(
            "func".to_string(),
            vec![MonoType::Int(i)],
            MonoType::Int(i),
            vec![],
        ));
    }

    // Act: 使用 Int(0) 精确匹配第一个候选
    let result = resolver.resolve("func", &[MonoType::Int(0)]);

    // Assert: 大量候选下仍应正确解析，不 panic
    assert!(
        result.is_ok(),
        "100 个候选下 resolve 不应 panic，且应匹配到精确候选"
    );
    assert_eq!(
        result.unwrap().return_type,
        MonoType::Int(0),
        "应匹配返回类型为 Int(0) 的候选"
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_resolve_no_matching_candidate() {
    // Arrange: 只添加 Int 候选
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "add".to_string(),
        vec![MonoType::Int(32), MonoType::Int(32)],
        MonoType::Int(32),
        vec![],
    ));

    // Act: 用 String 类型调用
    let result = resolver.resolve("add", &[MonoType::String, MonoType::String]);

    // Assert: 应返回 NoMatchingDefinition 错误
    assert!(result.is_err(), "不兼容类型调用应返回错误");
    let err = result.unwrap_err();
    assert!(err.code.starts_with("E10"), "error code should be E1xxx, got: {}", err.code);
}

#[test]
fn test_resolve_unknown_function() {
    // Arrange: 添加名为 "add" 的候选
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "add".to_string(),
        vec![MonoType::Int(32)],
        MonoType::Int(32),
        vec![],
    ));

    // Act: 查询不存在的函数名 "multiply"
    let result = resolver.resolve("multiply", &[MonoType::Int(32)]);

    // Assert: 应返回 NoMatchingDefinition（函数名不存在）
    assert!(result.is_err(), "查询不存在的函数名应返回错误");
    let err = result.unwrap_err();
    assert!(err.code.starts_with("E10"), "error code should be E1xxx, got: {}", err.code);
}

#[test]
fn test_resolve_arg_count_mismatch_too_few() {
    // Arrange: 候选需要 2 个参数
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "add".to_string(),
        vec![MonoType::Int(32), MonoType::Int(32)],
        MonoType::Int(32),
        vec![],
    ));

    // Act: 只传 1 个参数
    let result = resolver.resolve("add", &[MonoType::Int(32)]);

    // Assert: 应返回 ArgCountMismatch 错误
    assert!(result.is_err(), "参数数量不足应返回错误");
    let err = result.unwrap_err();
    assert!(err.code == "E1010", "expected E1010, got: {}", err.code);
}

#[test]
fn test_resolve_arg_count_mismatch_too_many() {
    // Arrange: 候选需要 1 个参数
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "negate".to_string(),
        vec![MonoType::Int(32)],
        MonoType::Int(32),
        vec![],
    ));

    // Act: 传 3 个参数
    let result = resolver.resolve(
        "negate",
        &[MonoType::Int(32), MonoType::Int(32), MonoType::Int(32)],
    );

    // Assert: 应返回 ArgCountMismatch 错误
    assert!(result.is_err(), "参数数量过多应返回错误");
    let err = result.unwrap_err();
    assert!(err.code == "E1010", "expected E1010, got: {}", err.code);
}

#[test]
fn test_resolve_empty_resolver() {
    // Arrange: 空解析器
    let resolver = OverloadResolver::new();

    // Act
    let result = resolver.resolve("any_func", &[MonoType::Int(32)]);

    // Assert: 空解析器应返回 NoMatchingDefinition
    assert!(result.is_err(), "空解析器调用 resolve 应返回错误");
    let err = result.unwrap_err();
    assert!(err.code.starts_with("E10"), "error code should be E1xxx, got: {}", err.code);
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_resolve_selects_best_among_multiple_candidates() {
    // Arrange: 添加 Int 精确候选和 TypeVar 泛型候选
    let mut resolver = OverloadResolver::new();
    // 泛型版本（规范 §3.15.1: 优先级 3，泛型匹配）
    resolver.add_candidate(OverloadCandidate::new(
        "process".to_string(),
        vec![MonoType::TypeVar(TypeVar::new(0))],
        MonoType::TypeVar(TypeVar::new(0)),
        vec!["T".to_string()],
    ));
    // 精确版本（规范 §3.15.1: 优先级 1，精确匹配）
    resolver.add_candidate(OverloadCandidate::new(
        "process".to_string(),
        vec![MonoType::Int(32)],
        MonoType::Int(32),
        vec![],
    ));

    // Act: 用 Int(32) 调用，应选精确匹配（规范 §3.15.1: 精确匹配优先于泛型匹配）
    let result = resolver.resolve("process", &[MonoType::Int(32)]);

    // Assert: 精确匹配优先级更高，应被选中
    assert!(result.is_ok(), "存在精确匹配和泛型匹配时应成功解析");
    let candidate = result.unwrap();
    assert!(
        !candidate.is_generic,
        "应优先选择精确匹配（非泛型）候选（规范 §3.15.1: 优先级 1 > 3）"
    );
    assert_eq!(
        candidate.return_type,
        MonoType::Int(32),
        "选中的候选返回类型应为 Int(32)"
    );
}

#[test]
fn test_resolve_ambiguous_same_score() {
    // Arrange: 两个同优先级的泛型候选
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "identity".to_string(),
        vec![MonoType::TypeVar(TypeVar::new(0))],
        MonoType::TypeVar(TypeVar::new(0)),
        vec!["T".to_string()],
    ));
    resolver.add_candidate(OverloadCandidate::new(
        "identity".to_string(),
        vec![MonoType::TypeVar(TypeVar::new(1))],
        MonoType::TypeVar(TypeVar::new(1)),
        vec!["U".to_string()],
    ));

    // Act: 用 Int(32) 调用，两个泛型候选同优先级
    let result = resolver.resolve("identity", &[MonoType::Int(32)]);

    // Assert: 同优先级候选应产生歧义错误（规范 §3.15.1: 多个同优先级候选且无法区分 → 编译错误）
    assert!(result.is_err(), "两个同优先级的候选应产生歧义错误");
    let err = result.unwrap_err();
    assert!(err.code.starts_with("E10"), "error code should be E1xxx, got: {}", err.code);
}

#[test]
fn test_resolve_generic_candidate_match() {
    // Arrange: 只有泛型候选
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "identity".to_string(),
        vec![MonoType::TypeVar(TypeVar::new(0))],
        MonoType::TypeVar(TypeVar::new(0)),
        vec!["T".to_string()],
    ));

    // Act: 传入具体类型
    let result = resolver.resolve("identity", &[MonoType::Float(64)]);

    // Assert: 泛型候选应兼容匹配
    assert!(result.is_ok(), "泛型候选应能匹配任意具体类型");
    let candidate = result.unwrap();
    assert!(candidate.is_generic, "匹配的候选应标记为泛型");
    assert_eq!(
        candidate.param_types,
        vec![MonoType::TypeVar(TypeVar::new(0))],
        "泛型候选的参数类型应保持 TypeVar"
    );
}

#[test]
fn test_param_count_matches_boundary() {
    // Arrange
    let candidate = OverloadCandidate::new(
        "f".to_string(),
        vec![MonoType::Int(32), MonoType::Float(64)],
        MonoType::String,
        vec![],
    );

    // Act & Assert
    assert!(candidate.param_count_matches(2), "参数数量为 2 时应匹配");
    assert!(!candidate.param_count_matches(1), "参数数量为 1 时不应匹配");
    assert!(!candidate.param_count_matches(3), "参数数量为 3 时不应匹配");
    assert!(!candidate.param_count_matches(0), "参数数量为 0 时不应匹配");
}

#[test]
fn test_resolve_single_candidate_perfect_match() {
    // Arrange: 只有一个候选，参数类型完全匹配
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "to_string".to_string(),
        vec![MonoType::Int(32)],
        MonoType::String,
        vec![],
    ));

    // Act
    let result = resolver.resolve("to_string", &[MonoType::Int(32)]);

    // Assert
    assert!(result.is_ok(), "唯一候选精确匹配时应成功");
    let candidate = result.unwrap();
    assert_eq!(
        candidate.return_type,
        MonoType::String,
        "返回类型应为 String"
    );
    assert_eq!(candidate.name, "to_string", "候选名称应为 'to_string'");
}

#[test]
fn test_resolve_mixed_exact_and_incompatible() {
    // Arrange: 一个精确匹配、一个不兼容
    let mut resolver = OverloadResolver::new();
    resolver.add_candidate(OverloadCandidate::new(
        "convert".to_string(),
        vec![MonoType::Int(32)],
        MonoType::String,
        vec![],
    ));
    resolver.add_candidate(OverloadCandidate::new(
        "convert".to_string(),
        vec![MonoType::String],
        MonoType::Int(32),
        vec![],
    ));

    // Act: 传入 Int(32)，只有第一个候选匹配
    let result = resolver.resolve("convert", &[MonoType::Int(32)]);

    // Assert: 应选择唯一匹配的候选
    assert!(result.is_ok(), "存在唯一匹配候选时应成功");
    let candidate = result.unwrap();
    assert_eq!(
        candidate.return_type,
        MonoType::String,
        "应匹配返回类型为 String 的 Int->String 候选"
    );
}
