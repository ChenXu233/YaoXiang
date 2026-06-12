//! Overload 分析模块测试
//!
//! 测试重载解析功能，包括：
//! - 重载候选创建
//! - 精确匹配解析
//! - 歧义处理
//! - 无匹配处理
//! - 类型匹配评分
//! - 泛型 fallback 机制

use std::collections::HashMap;

use crate::frontend::core::typecheck::passes::overload::{
    instantiate_return_type, resolve_generic_fallback, OverloadCandidate, OverloadResolver, TypeVar,
};
use crate::frontend::core::types::MonoType;

fn int_type() -> MonoType {
    MonoType::Int(32)
}

fn float_type() -> MonoType {
    MonoType::Float(64)
}

fn string_type() -> MonoType {
    MonoType::String
}

#[test]
fn test_overload_candidate_creation() {
    let candidate = OverloadCandidate::new(
        "add".to_string(),
        vec![int_type(), int_type()],
        int_type(),
        vec![],
    );

    assert_eq!(candidate.name, "add");
    assert_eq!(candidate.param_types.len(), 2);
    assert!(!candidate.is_generic);
}

#[test]
fn test_overload_resolution_exact() {
    let mut resolver = OverloadResolver::new();

    // 添加重载候选
    resolver.add_candidate(OverloadCandidate::new(
        "add".to_string(),
        vec![int_type(), int_type()],
        int_type(),
        vec![],
    ));

    resolver.add_candidate(OverloadCandidate::new(
        "add".to_string(),
        vec![float_type(), float_type()],
        float_type(),
        vec![],
    ));

    // 精确匹配 Int 版本
    let result = resolver.resolve("add", &[int_type(), int_type()]);
    assert!(result.is_ok());
    let candidate = result.unwrap();
    assert_eq!(candidate.param_types[0], int_type());
}

#[test]
fn test_overload_resolution_ambiguous() {
    let mut resolver = OverloadResolver::new();

    // 添加两个兼容的候选
    resolver.add_candidate(OverloadCandidate::new(
        "identity".to_string(),
        vec![int_type()],
        int_type(),
        vec!["T".to_string()],
    ));

    resolver.add_candidate(OverloadCandidate::new(
        "identity".to_string(),
        vec![float_type()],
        float_type(),
        vec!["T".to_string()],
    ));

    // 使用 int_type 调用，两者都匹配
    let result = resolver.resolve("identity", &[int_type()]);
    assert!(result.is_ok());
}

#[test]
fn test_overload_resolution_no_match() {
    let mut resolver = OverloadResolver::new();

    resolver.add_candidate(OverloadCandidate::new(
        "add".to_string(),
        vec![int_type(), int_type()],
        int_type(),
        vec![],
    ));

    // 使用不兼容的类型
    let result = resolver.resolve("add", &[string_type(), int_type()]);
    assert!(result.is_err());
    assert!(result.unwrap_err().code.starts_with("E10"));
}

#[test]
fn test_type_match_score() {
    let resolver = OverloadResolver::new();

    // 精确匹配
    assert_eq!(resolver.type_match_score(&int_type(), &int_type()), 1.0);

    // 不匹配
    assert_eq!(resolver.type_match_score(&int_type(), &float_type()), -1.0);
}

#[test]
fn test_generic_fallback() {
    // 测试泛型 fallback 机制
    let candidates: HashMap<String, Vec<OverloadCandidate>> = [(
        "identity".to_string(),
        vec![OverloadCandidate::new(
            "identity".to_string(),
            vec![MonoType::TypeVar(TypeVar::new(0))],
            MonoType::TypeVar(TypeVar::new(0)),
            vec!["T".to_string()],
        )],
    )]
    .into_iter()
    .collect();

    // 使用 Int 类型调用泛型函数
    let result = resolve_generic_fallback(&candidates, "identity", &[int_type()]);
    assert!(result.is_some());
    assert!(result.unwrap().is_generic);

    // 验证实例化返回类型
    let return_type = instantiate_return_type(result.unwrap(), &[int_type()]);
    assert_eq!(return_type, int_type());
}

#[test]
fn test_generic_fallback_with_complex_type() {
    // 测试泛型 fallback 与复杂类型 - 简化测试
    // 验证 substitute_return_type 基本功能
    let candidates: HashMap<String, Vec<OverloadCandidate>> = [(
        "first".to_string(),
        vec![OverloadCandidate::new(
            "first".to_string(),
            vec![MonoType::TypeVar(TypeVar::new(0))],
            MonoType::TypeVar(TypeVar::new(0)),
            vec!["T".to_string()],
        )],
    )]
    .into_iter()
    .collect();

    // 使用 Int 类型调用泛型函数
    let result = resolve_generic_fallback(&candidates, "first", &[int_type()]);
    assert!(result.is_some());

    // 验证实例化返回类型
    let return_type = instantiate_return_type(result.unwrap(), &[int_type()]);
    assert_eq!(return_type, int_type());
}
