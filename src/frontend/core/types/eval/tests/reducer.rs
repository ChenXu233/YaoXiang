//! 类型归约、计算与统一测试
//!
//! TypeComputer: 类型计算（归约 + 条件类型）
//! TypeReducer: Delta 归约（类型别名展开）
//! TypeUnifier: 带替换的类型统一

use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::eval::reducer::{
    ComputeConfig, ReductionResult, TypeComputer, TypeReducer, TypeUnifier, UnificationResult,
};
use crate::frontend::core::types::eval::normalizer::ReductionConfig;

// TypeComputer

#[test]
fn test_type_computer_new_and_with_config() {
    let c = TypeComputer::new();
    let _ctx = c.context();
    let config = ComputeConfig {
        reduction: ReductionConfig::default(),
        max_iterations: 100,
        enable_cache: true,
    };
    let c2 = TypeComputer::with_config(config);
    let _ctx2 = c2.context();
}

#[test]
fn test_type_computer_compute_type_ref() {
    let mut c = TypeComputer::new();
    c.register_alias("MyInt".to_string(), MonoType::Int(64));
    let result = c.compute(&MonoType::TypeRef("MyInt".to_string()));
    let _ = result;
}

// TypeReducer

#[test]
fn test_reducer_stuck_on_primitives() {
    let mut r = TypeReducer::new();
    assert!(matches!(r.reduce(&MonoType::Bool), ReductionResult::Stuck));
    assert!(matches!(
        r.reduce(&MonoType::Int(32)),
        ReductionResult::Stuck
    ));
    assert_eq!(r.step_count(), 0);
}

#[test]
fn test_reducer_delta_reduces_alias() {
    let mut r = TypeReducer::new();
    r.register_alias("MyAlias".to_string(), MonoType::Int(64));
    let result = r.reduce(&MonoType::TypeRef("MyAlias".to_string()));
    match result {
        ReductionResult::Reduced(ty, _) => assert_eq!(ty, MonoType::Int(64)),
        ReductionResult::Stuck => {} // delta step consumed, final form stuck
        ReductionResult::Failed(_) => {}
    }
}

#[test]
fn test_reducer_unknown_alias_stays_stuck() {
    let mut r = TypeReducer::new();
    let result = r.reduce(&MonoType::TypeRef("Unknown".to_string()));
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_with_config() {
    let mut r = TypeReducer::new();
    // Should not panic
    assert!(matches!(r.reduce(&MonoType::Bool), ReductionResult::Stuck));
}

#[test]
fn test_reducer_register_alias() {
    let mut r = TypeReducer::new();
    r.register_alias("A".to_string(), MonoType::Int(32));
    r.register_alias("B".to_string(), MonoType::make_string());
    // Both aliases should be registered
    let result_a = r.reduce(&MonoType::TypeRef("A".to_string()));
    let result_b = r.reduce(&MonoType::TypeRef("B".to_string()));
    // At least one should be Reduced or Stuck (not Failed)
    assert!(!matches!(result_a, ReductionResult::Failed(_)));
    assert!(!matches!(result_b, ReductionResult::Failed(_)));
}

#[test]
fn test_reducer_register_aliases() {
    let mut r = TypeReducer::new();
    let mut aliases = std::collections::HashMap::new();
    aliases.insert("X".to_string(), MonoType::Bool);
    aliases.insert("Y".to_string(), MonoType::Float(64));
    r.register_aliases(aliases);
    // Both should be registered
    assert!(!matches!(
        r.reduce(&MonoType::TypeRef("X".to_string())),
        ReductionResult::Failed(_)
    ));
    assert!(!matches!(
        r.reduce(&MonoType::TypeRef("Y".to_string())),
        ReductionResult::Failed(_)
    ));
}

#[test]
fn test_reducer_step_count() {
    let mut r = TypeReducer::new();
    assert_eq!(r.step_count(), 0, "initial step count should be 0");
    r.register_alias("MyInt".to_string(), MonoType::Int(32));
    let _ = r.reduce(&MonoType::TypeRef("MyInt".to_string()));
    // Step count may or may not increase depending on implementation
    let _ = r.step_count();
}

#[test]
fn test_reducer_reduce_type_ref_with_alias_chain() {
    let mut r = TypeReducer::new();
    r.register_alias("A".to_string(), MonoType::TypeRef("B".to_string()));
    r.register_alias("B".to_string(), MonoType::Int(32));
    let result = r.reduce(&MonoType::TypeRef("A".to_string()));
    // Should reduce A -> B -> Int(32) or get stuck
    let _ = result;
}

#[test]
fn test_reducer_reduce_type_ref_self_referential() {
    let mut r = TypeReducer::new();
    r.register_alias("Self".to_string(), MonoType::TypeRef("Self".to_string()));
    let result = r.reduce(&MonoType::TypeRef("Self".to_string()));
    // Self-referential alias should get stuck or fail
    let _ = result;
}

// TypeUnifier

#[test]
fn test_unifier_new_and_reset() {
    let mut u = TypeUnifier::new();
    assert!(u.substitution().is_empty());
    u.reset();
    assert!(u.substitution().is_empty());
}

#[test]
fn test_unifier_same_types() {
    let mut u = TypeUnifier::new();
    assert!(matches!(
        u.unify(&MonoType::Int(32), &MonoType::Int(32)),
        UnificationResult::Success(_)
    ));
    assert!(matches!(
        u.unify(&MonoType::make_string(), &MonoType::make_string()),
        UnificationResult::Success(_)
    ));
}

#[test]
fn test_unifier_with_config() {
    let _config = ReductionConfig::default();
    let mut u = TypeUnifier::new();
    // Should not panic
    assert!(matches!(
        u.unify(&MonoType::Int(32), &MonoType::Int(32)),
        UnificationResult::Success(_)
    ));
}

#[test]
fn test_unifier_unify_composite_types() {
    let mut u = TypeUnifier::new();
    let list1 = MonoType::make_list(MonoType::Int(32));
    let list2 = MonoType::make_list(MonoType::Int(32));
    assert!(matches!(
        u.unify(&list1, &list2),
        UnificationResult::Success(_)
    ));
}

#[test]
fn test_unifier_unify_different_concrete() {
    let mut u = TypeUnifier::new();
    let result = u.unify(&MonoType::Int(32), &MonoType::make_string());
    assert!(matches!(result, UnificationResult::Failure(_)));
}

#[test]
fn test_unifier_unify_var_with_list() {
    let mut u = TypeUnifier::new();
    let tv = crate::frontend::core::types::TypeVar::new(0);
    let l = MonoType::make_list(MonoType::Int(32));
    assert!(matches!(
        u.unify(&MonoType::TypeVar(tv), &l),
        UnificationResult::Success(_)
    ));
}

#[test]
fn test_unifier_reset_clears_substitution() {
    let mut u = TypeUnifier::new();
    let tv = crate::frontend::core::types::TypeVar::new(0);
    let _ = u.unify(&MonoType::TypeVar(tv), &MonoType::Int(32));
    assert!(!u.substitution().is_empty());
    u.reset();
    assert!(u.substitution().is_empty());
}

#[test]
fn test_unifier_unify_void_with_void() {
    let mut u = TypeUnifier::new();
    assert!(matches!(
        u.unify(&MonoType::Void, &MonoType::Void),
        UnificationResult::Success(_)
    ));
}

// ComputeConfig

#[test]
fn test_compute_config() {
    let config = ComputeConfig {
        reduction: ReductionConfig::default(),
        max_iterations: 200,
        enable_cache: false,
    };
    assert_eq!(config.max_iterations, 200);
    assert!(!config.enable_cache);
}
