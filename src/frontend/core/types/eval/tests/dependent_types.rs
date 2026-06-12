use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::eval::dependent_types::{
    AssociatedType, AssociatedTypeDef, DependentTypeEnv, TypeFamily,
};
use std::collections::HashMap;

#[test]
fn test_associated_type() {
    let at = AssociatedType::new(
        "Item".to_string(),
        AssociatedTypeDef::Direct(MonoType::Int(32)),
    );
    assert_eq!(at.name, "Item");
    assert_eq!(
        AssociatedTypeDef::Direct(MonoType::String).into_type(),
        MonoType::String
    );
}

#[test]
fn test_has_unbound_params() {
    let def = AssociatedTypeDef::TypeParam("T".to_string());
    assert!(!def.has_unbound_params(&["T".to_string()]));
    assert!(def.has_unbound_params(&[]));
    assert!(!AssociatedTypeDef::Direct(MonoType::Int(32)).has_unbound_params(&[]));
}

#[test]
fn test_substitute() {
    let def = AssociatedTypeDef::TypeParam("T".to_string());
    let mut subs = HashMap::new();
    subs.insert("T".to_string(), MonoType::Int(32));
    assert_eq!(def.substitute(&subs).into_type(), MonoType::Int(32));
}

#[test]
fn test_type_family() {
    let family = TypeFamily::new(
        "AsString".to_string(),
        vec!["T".to_string()],
        vec![],
        AssociatedTypeDef::Direct(MonoType::String),
    );
    assert_eq!(family.name, "AsString");
    assert_eq!(family.type_params().len(), 1);
    assert!(family.instantiate(&[MonoType::Int(32)]).is_some());
}

#[test]
fn test_type_family_associated_types() {
    let at = AssociatedType::new(
        "Item".to_string(),
        AssociatedTypeDef::TypeParam("T".to_string()),
    );
    let family = TypeFamily::new(
        "Iter".to_string(),
        vec!["T".to_string()],
        vec![at],
        AssociatedTypeDef::Direct(MonoType::Void),
    );
    assert!(family.get_associated_type("Item").is_some());
    assert!(family.get_associated_type("X").is_none());
    assert!(family
        .instantiate_associated_type("Item", &[MonoType::String])
        .is_some());
}

#[test]
fn test_dependent_type_env() {
    let mut env = DependentTypeEnv::new();
    assert!(env.get_type_family("nonexistent").is_none());
    let family = TypeFamily::new(
        "F".to_string(),
        vec![],
        vec![],
        AssociatedTypeDef::Direct(MonoType::Void),
    );
    env.register_type_family(family);
    assert!(env.get_type_family("F").is_some());
    // is_type_family_instance always returns None in current impl
    assert!(env
        .is_type_family_instance(&MonoType::TypeRef("F".to_string()))
        .is_none());
}

// ===================================================================
// 补充测试: AssociatedTypeDef 扩展
// ===================================================================

#[test]
fn test_associated_def_direct_into_type() {
    let def = AssociatedTypeDef::Direct(MonoType::Bool);
    assert_eq!(def.into_type(), MonoType::Bool);
}

#[test]
fn test_associated_def_type_param_into_type() {
    let def = AssociatedTypeDef::TypeParam("T".to_string());
    // TypeParam.into_type() should return a TypeRef
    let ty = def.into_type();
    assert!(matches!(ty, MonoType::TypeRef(_)));
}

#[test]
fn test_associated_def_substitute_no_match() {
    let def = AssociatedTypeDef::TypeParam("T".to_string());
    let mut subs = HashMap::new();
    subs.insert("U".to_string(), MonoType::Int(32));
    // No match for "T" in subs - should return original
    let result = def.substitute(&subs);
    assert!(matches!(result, AssociatedTypeDef::TypeParam(ref name) if name == "T"));
}

#[test]
fn test_associated_def_direct_substitute() {
    let def = AssociatedTypeDef::Direct(MonoType::Int(32));
    let mut subs = HashMap::new();
    subs.insert("T".to_string(), MonoType::String);
    // Direct type is not affected by substitution
    let result = def.substitute(&subs);
    assert_eq!(result.into_type(), MonoType::Int(32));
}

// ===================================================================
// 补充测试: TypeFamily 扩展
// ===================================================================

#[test]
fn test_type_family_instantiate_no_args() {
    let family = TypeFamily::new(
        "Void".to_string(),
        vec![],
        vec![],
        AssociatedTypeDef::Direct(MonoType::Void),
    );
    let result = family.instantiate(&[]);
    assert!(result.is_some());
    // instantiate returns AssociatedTypeDef, not MonoType
    assert_eq!(result.unwrap().into_type(), MonoType::Void);
}

#[test]
fn test_type_family_instantiate_wrong_arg_count() {
    let family = TypeFamily::new(
        "Id".to_string(),
        vec!["T".to_string()],
        vec![],
        AssociatedTypeDef::TypeParam("T".to_string()),
    );
    // Wrong number of args
    let result = family.instantiate(&[]);
    assert!(result.is_none());
}

#[test]
fn test_type_family_multiple_params() {
    let family = TypeFamily::new(
        "Pair".to_string(),
        vec!["A".to_string(), "B".to_string()],
        vec![],
        AssociatedTypeDef::Direct(MonoType::Tuple(vec![
            MonoType::TypeRef("A".to_string()),
            MonoType::TypeRef("B".to_string()),
        ])),
    );
    assert_eq!(family.type_params().len(), 2);
    let result = family.instantiate(&[MonoType::Int(32), MonoType::String]);
    assert!(result.is_some());
}

#[test]
fn test_type_family_get_associated_type_none() {
    let family = TypeFamily::new(
        "Simple".to_string(),
        vec![],
        vec![],
        AssociatedTypeDef::Direct(MonoType::Void),
    );
    assert!(family.get_associated_type("Nonexistent").is_none());
}

#[test]
fn test_type_family_instantiate_associated_type_not_found() {
    let family = TypeFamily::new(
        "Simple".to_string(),
        vec![],
        vec![],
        AssociatedTypeDef::Direct(MonoType::Void),
    );
    assert!(family
        .instantiate_associated_type("Nonexistent", &[])
        .is_none());
}

// ===================================================================
// 补充测试: DependentTypeEnv 扩展
// ===================================================================

#[test]
fn test_dependent_type_env_register_multiple() {
    let mut env = DependentTypeEnv::new();
    let f1 = TypeFamily::new(
        "F1".to_string(),
        vec![],
        vec![],
        AssociatedTypeDef::Direct(MonoType::Int(32)),
    );
    let f2 = TypeFamily::new(
        "F2".to_string(),
        vec![],
        vec![],
        AssociatedTypeDef::Direct(MonoType::String),
    );
    env.register_type_family(f1);
    env.register_type_family(f2);
    assert!(env.get_type_family("F1").is_some());
    assert!(env.get_type_family("F2").is_some());
    assert!(env.get_type_family("F3").is_none());
}

#[test]
fn test_dependent_type_env_overwrite() {
    let mut env = DependentTypeEnv::new();
    let f1 = TypeFamily::new(
        "F".to_string(),
        vec![],
        vec![],
        AssociatedTypeDef::Direct(MonoType::Int(32)),
    );
    let f2 = TypeFamily::new(
        "F".to_string(),
        vec![],
        vec![],
        AssociatedTypeDef::Direct(MonoType::String),
    );
    env.register_type_family(f1);
    env.register_type_family(f2);
    // Should be overwritten
    let family = env.get_type_family("F").unwrap();
    assert_eq!(
        family.instantiate(&[]).unwrap().into_type(),
        MonoType::String
    );
}

#[test]
fn test_associated_type_new() {
    let at = AssociatedType::new(
        "Item".to_string(),
        AssociatedTypeDef::Direct(MonoType::Bool),
    );
    assert_eq!(at.name, "Item");
    assert_eq!(at.definition.into_type(), MonoType::Bool);
}
