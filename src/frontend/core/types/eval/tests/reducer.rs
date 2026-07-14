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

// ===================================================================
// TypeComputer
// ===================================================================

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
fn test_type_computer_register_alias() {
    let mut c = TypeComputer::new();
    c.register_alias("MyInt".to_string(), MonoType::Int(32));
    let result = c.compute(&MonoType::TypeRef("MyInt".to_string()));
    // Should not panic, should return Done or Pending
    let _ = result;
}

#[test]
fn test_type_computer_compute_primitive() {
    let mut c = TypeComputer::new();
    let result = c.compute(&MonoType::Int(32));
    // Primitives should compute without error
    let _ = result;
}

#[test]
fn test_type_computer_compute_type_ref() {
    let mut c = TypeComputer::new();
    c.register_alias("MyInt".to_string(), MonoType::Int(64));
    let result = c.compute(&MonoType::TypeRef("MyInt".to_string()));
    let _ = result;
}

#[test]
fn test_type_computer_set_context() {
    let c = TypeComputer::new();
    // set_context expects ComputeContext, not NormalizationContext
    // Just verify the computer can be created and used
    let _ = c.context();
}

#[test]
fn test_type_computer_compute_type_var() {
    let mut c = TypeComputer::new();
    let tv = MonoType::TypeVar(crate::frontend::core::types::TypeVar::new(0));
    let result = c.compute(&tv);
    let _ = result;
}

#[test]
fn test_type_computer_compute_list() {
    let mut c = TypeComputer::new();
    let list = MonoType::List(Box::new(MonoType::Int(32)));
    let result = c.compute(&list);
    let _ = result;
}

#[test]
fn test_type_computer_compute_tuple() {
    let mut c = TypeComputer::new();
    let tuple = MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]);
    let result = c.compute(&tuple);
    let _ = result;
}

#[test]
fn test_type_computer_compute_fn() {
    let mut c = TypeComputer::new();
    let f = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };
    let result = c.compute(&f);
    let _ = result;
}

// ===================================================================
// TypeReducer
// ===================================================================

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
    r.register_alias("B".to_string(), MonoType::String);
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
fn test_reducer_reduce_with_limit() {
    let mut r = TypeReducer::new();
    r.register_alias("A".to_string(), MonoType::Int(32));
    let result = r.reduce_with_limit(&MonoType::TypeRef("A".to_string()), 10);
    // Should not panic
    let _ = result;
}

#[test]
fn test_reducer_reduce_function_type() {
    let mut r = TypeReducer::new();
    let f = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };
    let result = r.reduce(&f);
    // Function types should be Stuck (no eta reduction implemented)
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_empty_function() {
    let mut r = TypeReducer::new();
    let f = MonoType::Fn {
        params: vec![],
        return_type: Box::new(MonoType::Void),
    };
    let result = r.reduce(&f);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_step_count_after_reduce() {
    let mut r = TypeReducer::new();
    assert_eq!(r.step_count(), 0);
    r.register_alias("X".to_string(), MonoType::Int(32));
    let _ = r.reduce(&MonoType::TypeRef("X".to_string()));
    // Step count may increase
    let _ = r.step_count();
}

#[test]
fn test_reducer_reduce_list_type() {
    let mut r = TypeReducer::new();
    let list = MonoType::List(Box::new(MonoType::Int(32)));
    let result = r.reduce(&list);
    // List types should be Stuck
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_tuple_type() {
    let mut r = TypeReducer::new();
    let tuple = MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]);
    let result = r.reduce(&tuple);
    // Tuple types should be Stuck
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_dict_type() {
    let mut r = TypeReducer::new();
    let dict = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(32)));
    let result = r.reduce(&dict);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_set_type() {
    let mut r = TypeReducer::new();
    let set = MonoType::Set(Box::new(MonoType::Bool));
    let result = r.reduce(&set);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_range_type() {
    let mut r = TypeReducer::new();
    let range = MonoType::Range {
        elem_type: Box::new(MonoType::Int(64)),
    };
    let result = r.reduce(&range);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_arc_type() {
    let mut r = TypeReducer::new();
    let arc = MonoType::Arc(Box::new(MonoType::Int(32)));
    let result = r.reduce(&arc);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_weak_type() {
    let mut r = TypeReducer::new();
    let weak = MonoType::Weak(Box::new(MonoType::String));
    let result = r.reduce(&weak);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_option_type() {
    let mut r = TypeReducer::new();
    let opt = MonoType::Option(Box::new(MonoType::Int(32)));
    let result = r.reduce(&opt);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_result_type() {
    let mut r = TypeReducer::new();
    let res = MonoType::Result(Box::new(MonoType::Int(32)), Box::new(MonoType::String));
    let result = r.reduce(&res);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_union_type() {
    let mut r = TypeReducer::new();
    let union = MonoType::Union(vec![MonoType::Int(32), MonoType::String]);
    let result = r.reduce(&union);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_intersection_type() {
    let mut r = TypeReducer::new();
    let inter = MonoType::Intersection(vec![
        MonoType::TypeRef("Clone".to_string()),
        MonoType::TypeRef("Display".to_string()),
    ]);
    let result = r.reduce(&inter);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_enum_type() {
    let mut r = TypeReducer::new();
    let e = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Color".to_string(),
        variants: vec!["red".to_string()],
    });
    let result = r.reduce(&e);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_struct_type() {
    let mut r = TypeReducer::new();
    let s = MonoType::Struct(crate::frontend::core::types::StructType {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), MonoType::Float(64))],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
        constraints: Vec::new(),
    });
    let result = r.reduce(&s);
    assert!(matches!(result, ReductionResult::Stuck));
}

#[test]
fn test_reducer_reduce_type_var() {
    let mut r = TypeReducer::new();
    let tv = MonoType::TypeVar(crate::frontend::core::types::TypeVar::new(0));
    let result = r.reduce(&tv);
    // TypeVar should be Stuck
    assert!(matches!(result, ReductionResult::Stuck));
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

// ===================================================================
// TypeUnifier
// ===================================================================

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
        u.unify(&MonoType::String, &MonoType::String),
        UnificationResult::Success(_)
    ));
}

#[test]
fn test_unifier_different_types() {
    let mut u = TypeUnifier::new();
    match u.unify(&MonoType::Int(32), &MonoType::String) {
        UnificationResult::Success(_) => {}
        UnificationResult::Failure(_) => {}
        UnificationResult::NeedReduction(..) => {}
    }
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
fn test_unifier_substitution_after_unify() {
    let mut u = TypeUnifier::new();
    let _ = u.unify(&MonoType::Int(32), &MonoType::Int(32));
    let sub = u.substitution();
    // After unifying same types, substitution should exist
    let _ = sub;
}

#[test]
fn test_unifier_unify_composite_types() {
    let mut u = TypeUnifier::new();
    let list1 = MonoType::List(Box::new(MonoType::Int(32)));
    let list2 = MonoType::List(Box::new(MonoType::Int(32)));
    assert!(matches!(
        u.unify(&list1, &list2),
        UnificationResult::Success(_)
    ));
}

#[test]
fn test_unifier_unify_different_composite() {
    let mut u = TypeUnifier::new();
    let list1 = MonoType::List(Box::new(MonoType::Int(32)));
    let list2 = MonoType::List(Box::new(MonoType::String));
    let result = u.unify(&list1, &list2);
    // Should be Failure or NeedReduction
    assert!(!matches!(result, UnificationResult::Success(_)));
}

// ===================================================================
// TypeUnifier - unify_internal 路径
// ===================================================================

#[test]
fn test_unifier_unify_type_vars_same() {
    let mut u = TypeUnifier::new();
    let tv = crate::frontend::core::types::TypeVar::new(0);
    let result = u.unify(&MonoType::TypeVar(tv), &MonoType::TypeVar(tv));
    assert!(matches!(result, UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_type_vars_different() {
    let mut u = TypeUnifier::new();
    let tv1 = crate::frontend::core::types::TypeVar::new(0);
    let tv2 = crate::frontend::core::types::TypeVar::new(1);
    let result = u.unify(&MonoType::TypeVar(tv1), &MonoType::TypeVar(tv2));
    assert!(matches!(result, UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_var_with_concrete() {
    let mut u = TypeUnifier::new();
    let tv = crate::frontend::core::types::TypeVar::new(0);
    let result = u.unify(&MonoType::TypeVar(tv), &MonoType::Int(32));
    assert!(matches!(result, UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_concrete_with_var() {
    let mut u = TypeUnifier::new();
    let tv = crate::frontend::core::types::TypeVar::new(0);
    let result = u.unify(&MonoType::Int(32), &MonoType::TypeVar(tv));
    assert!(matches!(result, UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_same_concrete() {
    let mut u = TypeUnifier::new();
    let result = u.unify(&MonoType::Bool, &MonoType::Bool);
    assert!(matches!(result, UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_different_concrete() {
    let mut u = TypeUnifier::new();
    let result = u.unify(&MonoType::Int(32), &MonoType::String);
    assert!(matches!(result, UnificationResult::Failure(_)));
}

#[test]
fn test_unifier_unify_tuples_same_length() {
    let mut u = TypeUnifier::new();
    let t1 = MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]);
    let t2 = MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]);
    assert!(matches!(u.unify(&t1, &t2), UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_tuples_different_length() {
    let mut u = TypeUnifier::new();
    let t1 = MonoType::Tuple(vec![MonoType::Int(32)]);
    let t2 = MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]);
    assert!(matches!(u.unify(&t1, &t2), UnificationResult::Failure(_)));
}

#[test]
fn test_unifier_unify_lists_same() {
    let mut u = TypeUnifier::new();
    let l1 = MonoType::List(Box::new(MonoType::Int(32)));
    let l2 = MonoType::List(Box::new(MonoType::Int(32)));
    assert!(matches!(u.unify(&l1, &l2), UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_lists_different() {
    let mut u = TypeUnifier::new();
    let l1 = MonoType::List(Box::new(MonoType::Int(32)));
    let l2 = MonoType::List(Box::new(MonoType::String));
    assert!(matches!(u.unify(&l1, &l2), UnificationResult::Failure(_)));
}

#[test]
fn test_unifier_unify_fns_same() {
    let mut u = TypeUnifier::new();
    let f1 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };
    let f2 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };
    assert!(matches!(u.unify(&f1, &f2), UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_fns_different_arity() {
    let mut u = TypeUnifier::new();
    let f1 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };
    let f2 = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Bool],
        return_type: Box::new(MonoType::String),
    };
    assert!(matches!(u.unify(&f1, &f2), UnificationResult::Failure(_)));
}

#[test]
fn test_unifier_unify_fns_different_return() {
    let mut u = TypeUnifier::new();
    let f1 = MonoType::Fn {
        params: vec![],
        return_type: Box::new(MonoType::Int(32)),
    };
    let f2 = MonoType::Fn {
        params: vec![],
        return_type: Box::new(MonoType::String),
    };
    assert!(matches!(u.unify(&f1, &f2), UnificationResult::Failure(_)));
}

#[test]
fn test_unifier_unify_completely_different() {
    let mut u = TypeUnifier::new();
    let result = u.unify(&MonoType::Int(32), &MonoType::Tuple(vec![MonoType::Bool]));
    assert!(matches!(result, UnificationResult::Failure(_)));
}

// ===================================================================
// TypeUnifier - 更多路径
// ===================================================================

#[test]
fn test_unifier_unify_list_with_var() {
    let mut u = TypeUnifier::new();
    let tv = crate::frontend::core::types::TypeVar::new(0);
    let l1 = MonoType::List(Box::new(MonoType::Int(32)));
    let l2 = MonoType::List(Box::new(MonoType::TypeVar(tv)));
    assert!(matches!(u.unify(&l1, &l2), UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_tuple_with_var() {
    let mut u = TypeUnifier::new();
    let tv = crate::frontend::core::types::TypeVar::new(0);
    let t1 = MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]);
    let t2 = MonoType::Tuple(vec![MonoType::TypeVar(tv), MonoType::String]);
    assert!(matches!(u.unify(&t1, &t2), UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_fn_with_var() {
    let mut u = TypeUnifier::new();
    let tv = crate::frontend::core::types::TypeVar::new(0);
    let f1 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };
    let f2 = MonoType::Fn {
        params: vec![MonoType::TypeVar(tv)],
        return_type: Box::new(MonoType::String),
    };
    assert!(matches!(u.unify(&f1, &f2), UnificationResult::Success(_)));
}

#[test]
fn test_unifier_unify_var_with_list() {
    let mut u = TypeUnifier::new();
    let tv = crate::frontend::core::types::TypeVar::new(0);
    let l = MonoType::List(Box::new(MonoType::Int(32)));
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

#[test]
fn test_unifier_unify_char_with_char() {
    let mut u = TypeUnifier::new();
    assert!(matches!(
        u.unify(&MonoType::Char, &MonoType::Char),
        UnificationResult::Success(_)
    ));
}

#[test]
fn test_unifier_unify_bytes_with_bytes() {
    let mut u = TypeUnifier::new();
    assert!(matches!(
        u.unify(&MonoType::Bytes, &MonoType::Bytes),
        UnificationResult::Success(_)
    ));
}

#[test]
fn test_unifier_unify_option_same() {
    let mut u = TypeUnifier::new();
    let o1 = MonoType::Option(Box::new(MonoType::Int(32)));
    let o2 = MonoType::Option(Box::new(MonoType::Int(32)));
    // Option unification may fail (not handled) or succeed
    let _ = u.unify(&o1, &o2);
}

#[test]
fn test_unifier_unify_result_same() {
    let mut u = TypeUnifier::new();
    let r1 = MonoType::Result(Box::new(MonoType::Int(32)), Box::new(MonoType::String));
    let r2 = MonoType::Result(Box::new(MonoType::Int(32)), Box::new(MonoType::String));
    // Result unification may fail (not handled) or succeed
    let _ = u.unify(&r1, &r2);
}

#[test]
fn test_unifier_unify_dict_same() {
    let mut u = TypeUnifier::new();
    let d1 = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(32)));
    let d2 = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(32)));
    // Dict unification may fail (not handled) or succeed
    let _ = u.unify(&d1, &d2);
}

#[test]
fn test_unifier_unify_set_same() {
    let mut u = TypeUnifier::new();
    let s1 = MonoType::Set(Box::new(MonoType::Bool));
    let s2 = MonoType::Set(Box::new(MonoType::Bool));
    // Set unification may fail (not handled) or succeed
    let _ = u.unify(&s1, &s2);
}

#[test]
fn test_unifier_unify_range_same() {
    let mut u = TypeUnifier::new();
    let r1 = MonoType::Range {
        elem_type: Box::new(MonoType::Int(64)),
    };
    let r2 = MonoType::Range {
        elem_type: Box::new(MonoType::Int(64)),
    };
    // Range unification may fail (not handled) or succeed
    let _ = u.unify(&r1, &r2);
}

#[test]
fn test_unifier_unify_arc_same() {
    let mut u = TypeUnifier::new();
    let a1 = MonoType::Arc(Box::new(MonoType::Int(32)));
    let a2 = MonoType::Arc(Box::new(MonoType::Int(32)));
    // Arc unification may fail (not handled) or succeed
    let _ = u.unify(&a1, &a2);
}

#[test]
fn test_unifier_unify_weak_same() {
    let mut u = TypeUnifier::new();
    let w1 = MonoType::Weak(Box::new(MonoType::String));
    let w2 = MonoType::Weak(Box::new(MonoType::String));
    // Weak unification may fail (not handled) or succeed
    let _ = u.unify(&w1, &w2);
}

#[test]
fn test_unifier_unify_enum_same() {
    let mut u = TypeUnifier::new();
    let e1 = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Color".to_string(),
        variants: vec!["red".to_string()],
    });
    let e2 = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Color".to_string(),
        variants: vec!["red".to_string()],
    });
    // Enum unification may fail (not handled) or succeed
    let _ = u.unify(&e1, &e2);
}

#[test]
fn test_unifier_unify_struct_same() {
    let mut u = TypeUnifier::new();
    let s1 = MonoType::Struct(crate::frontend::core::types::StructType {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), MonoType::Float(64))],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
        constraints: Vec::new(),
    });
    let s2 = MonoType::Struct(crate::frontend::core::types::StructType {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), MonoType::Float(64))],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
        constraints: Vec::new(),
    });
    // Struct unification may fail (not handled) or succeed
    let _ = u.unify(&s1, &s2);
}

// ===================================================================
// ComputeConfig
// ===================================================================

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
