//! 重载解析测试 — 基于语言规范 §3.15 & RFC-011 §6
//!
//! §3.15: 函数重载与特化
//! RFC-011 §6: 函数重载特化

use crate::frontend::core::sema::passes::overload::{OverloadCandidate, OverloadResolver};
use crate::frontend::core::types::{MonoType, TypeVar};

// Happy path 测试

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
fn test_resolve_arg_count_mismatch_too_few() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
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
fn test_resolve_empty_resolver() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // Arrange: 空解析器
    let resolver = OverloadResolver::new();

    // Act
    let result = resolver.resolve("any_func", &[MonoType::Int(32)]);

    // Assert: 空解析器应返回 NoMatchingDefinition
    assert!(result.is_err(), "空解析器调用 resolve 应返回错误");
    let err = result.unwrap_err();
    assert!(
        err.code.starts_with("E10"),
        "error code should be E1xxx, got: {}",
        err.code
    );
}

#[test]
fn test_param_count_matches_boundary() {
    // Arrange
    let candidate = OverloadCandidate::new(
        "f".to_string(),
        vec![MonoType::Int(32), MonoType::Float(64)],
        MonoType::make_string(),
        vec![],
    );

    // Act & Assert
    assert!(candidate.param_count_matches(2), "参数数量为 2 时应匹配");
    assert!(!candidate.param_count_matches(1), "参数数量为 1 时不应匹配");
    assert!(!candidate.param_count_matches(3), "参数数量为 3 时不应匹配");
    assert!(!candidate.param_count_matches(0), "参数数量为 0 时不应匹配");
}
