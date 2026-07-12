//! Layer 0 is_subtype 单元测试

use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::typecheck::layers::equivalence::is_subtype;
use crate::frontend::core::types::MonoType;

#[test]
fn reflexivity_int64() {
    assert!(is_subtype(&MonoType::Int(64), &MonoType::Int(64), None));
}

#[test]
fn no_implicit_widening_int32_to_int64() {
    // §3.2.1: 禁止隐式 widening
    assert!(!is_subtype(&MonoType::Int(32), &MonoType::Int(64), None));
}

#[test]
fn no_int_to_float() {
    // §3.2.1: 禁止 Int→Float 隐式转换
    assert!(!is_subtype(&MonoType::Int(32), &MonoType::Float(64), None));
}

#[test]
fn list_covariance() {
    let sub = MonoType::List(Box::new(MonoType::Int(32)));
    let sup = MonoType::List(Box::new(MonoType::Int(32)));
    assert!(is_subtype(&sub, &sup, None));
}

#[test]
fn env_some_accepted() {
    // 占位：确保 is_subtype 接受 env: Option<&TypeEnvironment>
    let env = TypeEnvironment::new();
    let sub = MonoType::Int(32);
    let sup = MonoType::Int(32);
    assert!(is_subtype(&sub, &sup, Some(&env)));
}

#[test]
fn test_never_is_subtype_of_any() {
    assert!(is_subtype(&MonoType::Never, &MonoType::Int(64), None));
    assert!(is_subtype(&MonoType::Never, &MonoType::Void, None));
    assert!(is_subtype(&MonoType::Never, &MonoType::Bool, None));
    assert!(is_subtype(&MonoType::Never, &MonoType::String, None));
    assert!(is_subtype(
        &MonoType::Never,
        &MonoType::Fn {
            params: vec![],
            return_type: Box::new(MonoType::Int(64))
        },
        None
    ));
}
