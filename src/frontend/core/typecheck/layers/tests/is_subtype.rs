//! is_subtype 单元测试 — spec §3.x-L0 + RFC-027 §3.2

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

#[test]
fn test_metatype_subtype_lower_to_higher() {
    // Arrange
    let t0 = MonoType::MetaType {
        universe_level: crate::frontend::core::types::UniverseLevel::type0(),
        type_params: vec![],
    };
    let t1 = MonoType::MetaType {
        universe_level: crate::frontend::core::types::UniverseLevel::type1(),
        type_params: vec![],
    };

    // Act
    let result = is_subtype(&t0, &t1, None);

    // Assert
    assert!(result, "Type₀ should be subtype of Type₁ (n ≤ m)");
}

#[test]
fn test_metatype_subtype_higher_to_lower() {
    // Arrange
    let t0 = MonoType::MetaType {
        universe_level: crate::frontend::core::types::UniverseLevel::type0(),
        type_params: vec![],
    };
    let t1 = MonoType::MetaType {
        universe_level: crate::frontend::core::types::UniverseLevel::type1(),
        type_params: vec![],
    };

    // Act & Assert
    assert!(
        !is_subtype(&t1, &t0, None),
        "Type₁ should NOT be subtype of Type₀ (n > m)"
    );
}

#[test]
fn test_metatype_subtype_same_level() {
    // Arrange
    let t0 = MonoType::MetaType {
        universe_level: crate::frontend::core::types::UniverseLevel::type0(),
        type_params: vec![],
    };

    // Act & Assert
    assert!(
        is_subtype(&t0, &t0, None),
        "Type₀ should be subtype of itself (reflexivity)"
    );
}
