//! 类型替换测试 — 补充内联测试未覆盖的容器类型替换路径
//!
//! Substitution: bind, insert, get, contains_var, merge, len, is_empty, bound_vars
//! Substituter: substitute 经过所有容器类型 (Struct, Tuple, Dict, Set, Enum, Range, Arc, Weak, Union, Intersection, AssocType, Fn, Option, Result)

use crate::frontend::core::types::base::{
    EnumType, MonoType, StructType, Substitution, Substituter, TypeVar,
};
use std::collections::HashMap;

#[test]
fn test_substitution_bind_insert_get() {
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Int(32));
    sub.insert(1, MonoType::String);
    assert_eq!(sub.get(&0), Some(&MonoType::Int(32)));
    assert_eq!(sub.get(&1), Some(&MonoType::String));
    assert_eq!(sub.get(&99), None);
}

#[test]
fn test_substitution_contains_var() {
    let mut sub = Substitution::new();
    sub.insert(5, MonoType::Bool);
    assert!(sub.contains_var(&5));
    assert!(!sub.contains_var(&0));
}

#[test]
fn test_substitution_merge() {
    let mut a = Substitution::new();
    a.insert(0, MonoType::Int(32));
    let mut b = Substitution::new();
    b.insert(1, MonoType::String);
    let merged = a.merge(&b);
    assert_eq!(merged.get(&0), Some(&MonoType::Int(32)));
    assert_eq!(merged.get(&1), Some(&MonoType::String));
}

#[test]
fn test_substitution_bound_vars_len_is_empty() {
    let mut sub = Substitution::new();
    assert!(sub.is_empty());
    assert_eq!(sub.len(), 0);
    sub.insert(0, MonoType::Bool);
    sub.insert(2, MonoType::Int(32));
    assert!(!sub.is_empty());
    assert_eq!(sub.len(), 2);
    let vars = sub.bound_vars();
    assert!(vars.contains(&0));
    assert!(vars.contains(&2));
}

#[test]
fn test_substituter_substitute_var() {
    let subber = Substituter::new();
    let tv = TypeVar::new(0);
    let result = subber.substitute_var(&MonoType::TypeVar(tv), &tv, &MonoType::Int(32));
    assert_eq!(result, MonoType::Int(32));
    // Non-matching var stays as-is
    let result2 = subber.substitute_var(&MonoType::TypeVar(tv), &TypeVar::new(1), &MonoType::Bool);
    assert_eq!(result2, MonoType::TypeVar(tv));
}

#[test]
fn test_substituter_substitute_with_map() {
    let subber = Substituter::new();
    let mut map = HashMap::new();
    map.insert(0, MonoType::String);
    let tv = MonoType::TypeVar(TypeVar::new(0));
    let result = subber.substitute_with_map(&tv, &map);
    assert_eq!(result, MonoType::String);
}

// ===================================================================
// 以下测试覆盖 substitute_internal 对所有容器类型的递归替换
// ===================================================================

fn tv(idx: usize) -> MonoType {
    MonoType::TypeVar(TypeVar::new(idx))
}

#[test]
fn test_substitute_through_struct() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Int(32));
    let ty = MonoType::Struct(StructType {
        name: "Wrapper".to_string(),
        fields: vec![("value".to_string(), tv(0))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let result = subber.substitute(&ty, &sub);
    match result {
        MonoType::Struct(ref s) => assert_eq!(s.fields[0].1, MonoType::Int(32)),
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn test_substitute_through_tuple() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::String);
    let ty = MonoType::Tuple(vec![tv(0), MonoType::Bool]);
    let result = subber.substitute(&ty, &sub);
    assert_eq!(
        result,
        MonoType::Tuple(vec![MonoType::String, MonoType::Bool])
    );
}

#[test]
fn test_substitute_through_dict() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Int(32));
    let ty = MonoType::Dict(Box::new(MonoType::String), Box::new(tv(0)));
    let result = subber.substitute(&ty, &sub);
    assert_eq!(
        result,
        MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(32)))
    );
}

#[test]
fn test_substitute_through_set() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Bool);
    let ty = MonoType::Set(Box::new(tv(0)));
    let result = subber.substitute(&ty, &sub);
    assert_eq!(result, MonoType::Set(Box::new(MonoType::Bool)));
}

#[test]
fn test_substitute_through_fn() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Int(64));
    let ty = MonoType::Fn {
        params: vec![tv(0)],
        return_type: Box::new(tv(0)),
    };
    let result = subber.substitute(&ty, &sub);
    match result {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params[0], MonoType::Int(64));
            assert_eq!(*return_type, MonoType::Int(64));
        }
        _ => panic!("Expected Fn"),
    }
}

#[test]
fn test_substitute_through_option_result() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::String);
    // Option/Result not explicitly in substitute_internal → falls through
    let opt = MonoType::Option(Box::new(tv(0)));
    assert_eq!(subber.substitute(&opt, &sub), opt);
    let res = MonoType::Result(Box::new(tv(0)), Box::new(MonoType::Int(32)));
    assert_eq!(subber.substitute(&res, &sub), res);
}

#[test]
fn test_substitute_through_range() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Int(32));
    let ty = MonoType::Range {
        elem_type: Box::new(tv(0)),
    };
    let result = subber.substitute(&ty, &sub);
    assert_eq!(
        result,
        MonoType::Range {
            elem_type: Box::new(MonoType::Int(32))
        }
    );
}

#[test]
fn test_substitute_through_arc_weak() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::String);
    let arc = MonoType::Arc(Box::new(tv(0)));
    assert_eq!(
        subber.substitute(&arc, &sub),
        MonoType::Arc(Box::new(MonoType::String))
    );
    let weak = MonoType::Weak(Box::new(tv(0)));
    assert_eq!(
        subber.substitute(&weak, &sub),
        MonoType::Weak(Box::new(MonoType::String))
    );
}

#[test]
fn test_substitute_through_union_intersection() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Bool);
    let union = MonoType::Union(vec![tv(0), MonoType::Int(32)]);
    let result = subber.substitute(&union, &sub);
    assert_eq!(
        result,
        MonoType::Union(vec![MonoType::Bool, MonoType::Int(32)])
    );
    let inter = MonoType::Intersection(vec![tv(0), MonoType::String]);
    let result2 = subber.substitute(&inter, &sub);
    assert_eq!(
        result2,
        MonoType::Intersection(vec![MonoType::Bool, MonoType::String])
    );
}

#[test]
fn test_substitute_through_enum() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Int(32));
    // Enum variants don't contain type variables, so substitution is a no-op
    let ty = MonoType::Enum(EnumType {
        name: "Color".to_string(),
        variants: vec!["red".to_string()],
    });
    assert_eq!(subber.substitute(&ty, &sub), ty);
}

#[test]
fn test_substitute_through_assoc_type() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Int(32));
    let ty = MonoType::AssocType {
        host_type: Box::new(tv(0)),
        assoc_name: "Item".to_string(),
        assoc_args: vec![MonoType::String],
    };
    let result = subber.substitute(&ty, &sub);
    match result {
        MonoType::AssocType {
            host_type,
            assoc_name,
            ..
        } => {
            assert_eq!(*host_type, MonoType::Int(32));
            assert_eq!(assoc_name, "Item");
        }
        _ => panic!("Expected AssocType"),
    }
}

#[test]
fn test_substitute_no_match_returns_original() {
    let subber = Substituter::new();
    let sub = Substitution::new();
    let ty = MonoType::Int(32);
    // No variables in the type → returns clone
    assert_eq!(subber.substitute(&ty, &sub), MonoType::Int(32));
}

#[test]
fn test_substitute_generic_params_too_few() {
    let subber = Substituter::new();
    let tv = TypeVar::new(5);
    let ty = MonoType::TypeVar(tv);
    // args is empty, index 5 > length → lookup returns None → original returned
    let result = subber.substitute_generic_params(&ty, &[]);
    assert_eq!(result, MonoType::TypeVar(tv));
}

// ===================================================================
// §3: 补充替换测试
// ===================================================================

#[test]
fn test_substitute_through_list() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::String);
    let ty = MonoType::List(Box::new(tv(0)));
    let result = subber.substitute(&ty, &sub);
    assert_eq!(result, MonoType::List(Box::new(MonoType::String)));
}

#[test]
fn test_substitute_type_var_bound_to_type_var() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    // v0 → v1
    sub.bind(TypeVar::new(0), MonoType::TypeVar(TypeVar::new(1)));
    // Substitute v0 should give v1
    let result = subber.substitute(&tv(0), &sub);
    assert_eq!(result, MonoType::TypeVar(TypeVar::new(1)));
}

#[test]
fn test_substitute_with_map_no_match() {
    let subber = Substituter::new();
    let map = HashMap::new();
    let ty = MonoType::Int(32);
    let result = subber.substitute_with_map(&ty, &map);
    assert_eq!(
        result,
        MonoType::Int(32),
        "no substitution should return original"
    );
}

#[test]
fn test_substitute_with_map_multiple_vars() {
    let subber = Substituter::new();
    let mut map = HashMap::new();
    map.insert(0, MonoType::String);
    map.insert(1, MonoType::Bool);
    let ty = MonoType::Tuple(vec![tv(0), tv(1), MonoType::Int(32)]);
    let result = subber.substitute_with_map(&ty, &map);
    assert_eq!(
        result,
        MonoType::Tuple(vec![MonoType::String, MonoType::Bool, MonoType::Int(32)])
    );
}

#[test]
fn test_substitution_bound_vars_empty() {
    let sub = Substitution::new();
    assert!(
        sub.bound_vars().is_empty(),
        "empty substitution should have no bound vars"
    );
}

#[test]
fn test_substitution_bound_vars_non_empty() {
    let mut sub = Substitution::new();
    sub.insert(0, MonoType::Int(32));
    sub.insert(3, MonoType::String);
    let vars = sub.bound_vars();
    assert_eq!(vars.len(), 2, "should have 2 bound vars");
    assert!(vars.contains(&0), "should contain index 0");
    assert!(vars.contains(&3), "should contain index 3");
}

#[test]
fn test_substitute_generic_params_matching() {
    let subber = Substituter::new();
    let ty = MonoType::Tuple(vec![
        MonoType::TypeVar(TypeVar::new(0)),
        MonoType::TypeVar(TypeVar::new(1)),
    ]);
    let args = vec![MonoType::Int(32), MonoType::String];
    let result = subber.substitute_generic_params(&ty, &args);
    assert_eq!(
        result,
        MonoType::Tuple(vec![MonoType::Int(32), MonoType::String])
    );
}

#[test]
fn test_substitute_struct_with_nested_list() {
    let subber = Substituter::new();
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Bool);
    let ty = MonoType::Struct(StructType {
        name: "Wrapper".to_string(),
        fields: vec![("data".to_string(), MonoType::List(Box::new(tv(0))))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let result = subber.substitute(&ty, &sub);
    match result {
        MonoType::Struct(ref s) => {
            assert_eq!(s.fields[0].1, MonoType::List(Box::new(MonoType::Bool)));
        }
        _ => panic!("Expected Struct"),
    }
}

#[test]
fn test_substitute_contains_type_vars() {
    use crate::frontend::core::types::base::substitute::contains_type_vars;
    assert!(
        contains_type_vars(&tv(0)),
        "TypeVar should contain type vars"
    );
    assert!(
        !contains_type_vars(&MonoType::Int(32)),
        "Int should not contain type vars"
    );
    assert!(
        contains_type_vars(&MonoType::List(Box::new(tv(0)))),
        "List containing TypeVar should be detected"
    );
}
