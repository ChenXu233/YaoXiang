//! 类型替换测试 — 补充内联测试未覆盖的容器类型替换路径
//!
//! Substitution: bind, insert, get, contains_var, merge, len, is_empty, bound_vars
//! Substituter: substitute 经过所有容器类型 (Struct, Tuple, Dict, Set, Enum, Range, Arc, Weak, Union, Intersection, AssocType, Fn, Option, Result)

use crate::frontend::core::types::substitute::contains_type_vars;
use crate::frontend::core::types::{MonoType, Substitution, Substituter, TypeVar};
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// 借用位的「含类型变量」判据（WBS 6.8.3 病灶 M2 的**承重判据**，单元级）
// ─────────────────────────────────────────────────────────────────────────────
//
// 与 `TypeConstraintSolver::contains_var` 同族：缺 `Ref` 臂时「含类型变量」对借用位
// 恒为假，凡依赖它做判断的地方（occurs check 一类）在借用位整段失灵。端到端用例
// 区分不了这一臂（验证轮实测：去掉 Ref 臂后 2950 条全过），故判据落在函数本身。

#[test]
fn test_contains_type_vars_sees_through_borrow() {
    // Arrange — `&T`（借用包着一个类型变量），以及不含量变量的借用对照
    let tv = MonoType::TypeVar(TypeVar::new(7));
    let borrow_tv = MonoType::Ref {
        mutable: false,
        inner: Box::new(tv),
    };
    let borrow_int = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Int(64)),
    };

    // Act & Assert ①：`&T` 含类型变量——穿透借用位（删掉 Ref 臂即红）
    assert!(
        contains_type_vars(&borrow_tv),
        "contains_type_vars(&T) 必须为真：Ref 臂缺失时借用位恒判「不含变量」"
    );

    // Assert ②：负对照——`&Int` 不含类型变量（不是「有借用就恒真」）
    assert!(
        !contains_type_vars(&borrow_int),
        "contains_type_vars(&Int) 必须为假：判据只穿借用、不虚构变量"
    );
}

#[test]
fn test_substitution_bind_insert_get() {
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Int(32));
    sub.insert(1, MonoType::make_string());
    assert_eq!(sub.get(&0), Some(&MonoType::Int(32)));
    assert_eq!(sub.get(&1), Some(&MonoType::make_string()));
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
fn test_substitute_generic_params_matching() {
    let subber = Substituter::new();
    let ty = MonoType::make_tuple(vec![
        MonoType::TypeVar(TypeVar::new(0)),
        MonoType::TypeVar(TypeVar::new(1)),
    ]);
    let args = vec![MonoType::Int(32), MonoType::make_string()];
    let result = subber.substitute_generic_params(&ty, &args);
    assert_eq!(
        result,
        MonoType::make_tuple(vec![MonoType::Int(32), MonoType::make_string()])
    );
}
