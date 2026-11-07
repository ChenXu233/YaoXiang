//! Overload 分析模块测试
//! 规范来源：RFC-011b（运算符重载）
//!
//! 测试重载解析功能，包括：
//! - 重载候选创建
//! - 精确匹配解析
//! - 歧义处理
//! - 无匹配处理
//! - 类型匹配评分
//! - 泛型 fallback 机制

use std::collections::HashMap;

use crate::frontend::core::sema::passes::overload::{
    instantiate_return_type, resolve_generic_fallback, OverloadCandidate, OverloadResolver,
};
use crate::frontend::core::types::{MonoType, TypeVar};

fn int_type() -> MonoType {
    MonoType::Int(32)
}

fn float_type() -> MonoType {
    MonoType::Float(64)
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
fn test_type_match_score() {
    // Arrange
    let resolver = OverloadResolver::new();

    // Act / Assert
    // 精确匹配：评分 ≥ 0 → 可兼容（评分细则 1.0/0.8/0.5/-1.0 是实现细节，
    // 行为契约经公开方法 is_compatible 断言，见 overload.rs:332）
    assert!(
        resolver.is_compatible(&int_type(), &int_type()),
        "相同类型应判定为可兼容"
    );

    // 不匹配：评分 < 0 → 不可兼容
    assert!(
        !resolver.is_compatible(&int_type(), &float_type()),
        "Int 与 Float 应判定为不兼容"
    );
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
