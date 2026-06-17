//! 实例化数据类型测试 — 对应 src/middle/passes/mono/instance.rs
//!
//! RFC-011 §3: 零成本抽象与单态化
//! RFC-011 §4: 实例化请求与缓存键定义

use crate::frontend::core::typecheck::MonoType;
use crate::middle::passes::mono::instance::{
    GenericFunctionId, InstantiationRequest, SpecializationKey,
};
use crate::util::span::Span;

#[test]
fn test_specialization_key_deduplication() {
    // Arrange
    let key_a = SpecializationKey::new("identity".to_string(), vec![MonoType::Int(64)]);
    let key_b = SpecializationKey::new("identity".to_string(), vec![MonoType::Int(64)]);
    let key_c = SpecializationKey::new("identity".to_string(), vec![MonoType::String]);

    // Assert: 相同类型参数的 key 应相等
    assert_eq!(key_a, key_b, "相同类型参数的 SpecializationKey 应相等");

    // Assert: 不同类型参数的 key 应不等
    assert_ne!(key_a, key_c, "不同类型参数的 SpecializationKey 应不等");
}

#[test]
fn test_generic_function_id_name_accessor() {
    // Arrange
    let id = GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]);

    // Assert
    assert_eq!(id.name(), "identity");
    assert_eq!(id.type_params(), &["T".to_string()]);
}

#[test]
fn test_instantiation_request_specialization_key_generation() {
    // Arrange
    let req = InstantiationRequest::new(
        GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
        vec![MonoType::Int(64)],
        Span::default(),
    );

    // Act
    let key = req.specialization_key();

    // Assert
    assert_eq!(key.name, "identity");
    assert_eq!(key.type_args, vec![MonoType::Int(64)]);
}
