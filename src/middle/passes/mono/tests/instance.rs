//! 实例化数据类型测试 — 对应 src/middle/passes/mono/instance.rs
//!
//! RFC-011 §3: 零成本抽象与单态化
//! RFC-011 §4: 实例化请求与缓存键定义

use crate::frontend::core::sema::MonoType;
use crate::middle::passes::mono::instance::{
    GenericFunctionId, InstantiationRequest, SpecializationKey,
};
use crate::util::span::Span;

#[test]
fn test_specialization_key_deduplication() {
    // Arrange
    let key_a = SpecializationKey::new("identity".to_string(), vec![MonoType::Int(64)]);
    let key_b = SpecializationKey::new("identity".to_string(), vec![MonoType::Int(64)]);
    let key_c = SpecializationKey::new("identity".to_string(), vec![MonoType::make_string()]);

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
        vec![crate::frontend::core::types::mono::GenericArg::Type(
            MonoType::Int(64),
        )],
        Span::default(),
    );

    // Act
    let key = req.specialization_key();

    // Assert
    assert_eq!(key.name, "identity");
    assert_eq!(key.type_args, vec![MonoType::Int(64)]);
}

/// 6.9.2-C1 ①：`GenericArg` 的四个投影（`type_name` / `as_type` / `as_term` / `substitute`）
/// 都在同一 impl 块的穷尽 match 里——本条钉住两条变体的语义差：
/// 类型位有类型无项；编译期项位闭项有项、类型位由 **`as_type` 单向派生**。
#[test]
fn test_generic_arg_projections_distinguish_type_and_const() {
    // Arrange
    use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
    use crate::frontend::core::types::mono::GenericArg;
    let type_arg = GenericArg::Type(MonoType::Int(64));
    let const_arg = GenericArg::Const(ConstExpr::Lit(ConstValue::Int(42)));

    // Act + Assert — 类型位：有类型、无项
    assert_eq!(
        type_arg.as_type(),
        Some(MonoType::Int(64)),
        "类型位实参投影回自身"
    );
    assert!(type_arg.as_term().is_none(), "类型位实参不是编译期项");
    assert_eq!(
        type_arg.type_name(),
        "int64",
        "类型位名走 MonoType::type_name"
    );

    // Assert — 项位：有项，类型位由归一化后的值单向派生（Literal）
    assert!(const_arg.as_term().is_some(), "项位实参必须带项");
    match const_arg.as_type() {
        Some(MonoType::Literal { name, value, .. }) => {
            assert_eq!(
                name, "42",
                "Literal 名口径 = 值的 Display（与既有特化名一致）"
            );
            assert_eq!(value, ConstValue::Int(42), "Literal 值 = 项归一化结果");
        }
        other => panic!("闭项应投影出 Literal，实际: {other:?}"),
    }
    assert_eq!(
        const_arg.type_name(),
        const_arg.as_type().expect("闭项必有类型位").type_name(),
        "项位的名与类型位投影同一口径"
    );
}
