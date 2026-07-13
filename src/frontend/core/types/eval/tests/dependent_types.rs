//! 依赖类型与类型族测试 — 基于语言规范 §3.11 & RFC-011 §4-5
//!
//! §4.3: 编译期验证（IsTrue / Assert 类型族）
//! §5.1: If 条件类型（AssociatedTypeDef::Match 归约）
//! RFC-011 §4.3: Assert 定义为 IsTrue(cond)
//! spec 2026-07-12-assert-refinement-unification-design.md §1.3: IsTrue 桥

use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::eval::dependent_types::{
    register_builtin_type_families, AssociatedType, AssociatedTypeDef, DependentTypeEnv,
    RecursiveArm, RecursivePattern, TypeFamily, check_structural_termination, parse_nat_from_type,
    nat_to_type,
};
use crate::frontend::core::types::eval::type_families::Nat;
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

#[test]
fn test_associated_type_def_match_variant() {
    // Arrange — 构造 Match 变体：true→Void, false→Never
    let match_def = AssociatedTypeDef::Match {
        arg_index: 0,
        arms: vec![
            (MonoType::TypeRef("true".into()), MonoType::Void),
            (MonoType::TypeRef("false".into()), MonoType::Never),
        ],
    };

    // Act & Assert — Match 变体不依赖类型参数，has_unbound_params 返回 false
    assert!(
        !match_def.has_unbound_params(&[]),
        "Match with no type params should have no unbound params"
    );

    // Act & Assert — substitute 不改变 Match 内容
    let mut subs = HashMap::new();
    subs.insert("b".to_string(), MonoType::TypeRef("true".into()));
    let substituted = match_def.substitute(&subs);
    assert_eq!(
        substituted, match_def,
        "substitute should not alter Match variant"
    );

    // Assert — into_type 返回 Unknown（Match 不能直接转为类型）
    assert_eq!(
        match_def.into_type(),
        MonoType::TypeRef("Unknown".to_string()),
        "Match::into_type should return TypeRef(\"Unknown\")"
    );
}

#[test]
fn test_istrue_type_family_instantiate() {
    // Arrange — 注册内建类型族
    let mut env = DependentTypeEnv::new();
    register_builtin_type_families(&mut env);
    let istrue = env
        .get_type_family("IsTrue")
        .expect("IsTrue should be registered after register_builtin_type_families");

    // Assert — 类型参数为 ["b"]
    assert_eq!(
        istrue.type_params(),
        &["b"],
        "IsTrue should have one type param 'b'"
    );

    // Act & Assert — IsTrue(true) => Void（spec §1.3）
    let result = istrue.instantiate(&[MonoType::TypeRef("true".into())]);
    assert!(
        result.is_some(),
        "IsTrue(true) should instantiate successfully"
    );
    assert_eq!(
        result.unwrap().into_type(),
        MonoType::Void,
        "IsTrue(true) must reduce to Void"
    );

    // Act & Assert — IsTrue(false) => Never（spec §1.3）
    let result = istrue.instantiate(&[MonoType::TypeRef("false".into())]);
    assert!(
        result.is_some(),
        "IsTrue(false) should instantiate successfully"
    );
    assert_eq!(
        result.unwrap().into_type(),
        MonoType::Never,
        "IsTrue(false) must reduce to Never"
    );

    // Act & Assert — IsTrue(unknown) => None（无匹配 arm，保留不归约）
    let result = istrue.instantiate(&[MonoType::String]);
    assert!(
        result.is_none(),
        "IsTrue(String) should not match any arm and return None"
    );
}

#[test]
fn test_assert_type_family_instantiate() {
    // Arrange — 注册内建类型族
    let mut env = DependentTypeEnv::new();
    register_builtin_type_families(&mut env);
    let assert_tf = env
        .get_type_family("Assert")
        .expect("Assert should be registered after register_builtin_type_families");

    // Assert — 类型参数为 ["cond"]
    assert_eq!(
        assert_tf.type_params(),
        &["cond"],
        "Assert should have one type param 'cond'"
    );

    // Act — Assert(true) 展开为 IsTrue(true)
    let result = assert_tf.instantiate(&[MonoType::TypeRef("true".into())]);
    assert!(
        result.is_some(),
        "Assert(true) should instantiate successfully"
    );

    // Assert — Assert(true) 参数替换后得到 IsTrue(true)
    let result_ty = result.unwrap().into_type();
    assert_eq!(
        result_ty,
        MonoType::TypeRef("IsTrue(true)".to_string()),
        "Assert(true) must expand to IsTrue(true) via parameter substitution"
    );
}

// ===================================================================
// Recursive 类型族测试
// ===================================================================

#[test]
fn test_recursive_instantiate_zero() {
    // Arrange — factorial(Zero) = 1; factorial(Succ(n)) = Succ(n) * factorial(n)
    let factorial = TypeFamily::new(
        "factorial".to_string(),
        vec!["n".to_string()],
        vec![],
        AssociatedTypeDef::Recursive {
            arg_index: 0,
            arms: vec![
                RecursiveArm {
                    pattern: RecursivePattern::Zero,
                    result: MonoType::Int(1),
                },
                RecursiveArm {
                    pattern: RecursivePattern::Succ("ih_n".to_string()),
                    result: MonoType::TypeRef("Nat(Mul, Succ(n), factorial(ih_n))".to_string()),
                },
            ],
        },
    );

    // Act — factorial(Zero) → Int(1)
    let result = factorial.instantiate(&[MonoType::TypeRef("Zero".to_string())]);

    // Assert
    assert!(result.is_some(), "factorial(Zero) should instantiate");
    assert_eq!(
        result.unwrap().into_type(),
        MonoType::Int(1),
        "factorial(Zero) = 1"
    );
}

#[test]
fn test_recursive_instantiate_succ() {
    // Arrange — factorial(Succ(Zero)) = Succ(Zero) * factorial(Zero)
    let factorial = TypeFamily::new(
        "factorial".to_string(),
        vec!["n".to_string()],
        vec![],
        AssociatedTypeDef::Recursive {
            arg_index: 0,
            arms: vec![
                RecursiveArm {
                    pattern: RecursivePattern::Zero,
                    result: MonoType::Int(1),
                },
                RecursiveArm {
                    pattern: RecursivePattern::Succ("ih_n".to_string()),
                    result: MonoType::TypeRef("Nat(Mul, Succ(n), factorial(ih_n))".to_string()),
                },
            ],
        },
    );

    // Act — factorial(Succ(Zero))
    let result = factorial.instantiate(&[MonoType::TypeRef("Succ(Zero)".to_string())]);

    // Assert — should produce Nat(Mul, Succ(Succ(Zero)), factorial(Zero))
    assert!(result.is_some(), "factorial(Succ(Zero)) should instantiate");
    let result_ty = result.unwrap().into_type();
    let expected = MonoType::TypeRef("Nat(Mul, Succ(Succ(Zero)), factorial(Zero))".to_string());
    assert_eq!(
        result_ty, expected,
        "factorial(Succ(Zero)) should expand to one step"
    );
}

// ===================================================================
// Nat 解析/转换测试
// ===================================================================

#[test]
fn test_parse_nat_from_type_zero() {
    assert_eq!(
        parse_nat_from_type(&MonoType::TypeRef("Zero".to_string())),
        Some(Nat::Zero),
        "TypeRef('Zero') → Nat::Zero"
    );
}

#[test]
fn test_parse_nat_from_type_succ() {
    // Succ(Zero) → Nat::Succ(Nat::Zero)
    let parsed = parse_nat_from_type(&MonoType::TypeRef("Succ(Zero)".to_string()));
    assert!(parsed.is_some());
    assert_eq!(parsed.unwrap().to_usize(), 1, "Succ(Zero) = 1");

    // Succ(Succ(Zero)) → 2
    let parsed = parse_nat_from_type(&MonoType::TypeRef("Succ(Succ(Zero))".to_string()));
    assert!(parsed.is_some());
    assert_eq!(parsed.unwrap().to_usize(), 2, "Succ(Succ(Zero)) = 2");
}

#[test]
fn test_parse_nat_from_type_int() {
    assert_eq!(
        parse_nat_from_type(&MonoType::Int(5)),
        Some(Nat::from_usize(5)),
        "Int(5) → Nat::from_usize(5)"
    );
}

#[test]
fn test_parse_nat_from_type_bool() {
    assert_eq!(
        parse_nat_from_type(&MonoType::Bool),
        None,
        "Bool should not parse as Nat"
    );
}

#[test]
fn test_nat_to_type_roundtrip() {
    for i in 0..=5 {
        let nat = Nat::from_usize(i);
        let ty = nat_to_type(&nat);
        let parsed_back = parse_nat_from_type(&ty);
        assert_eq!(
            parsed_back,
            Some(nat.clone()),
            "Roundtrip failed for {}: {:?} → {:?} → {:?}",
            i,
            nat,
            ty,
            parsed_back
        );
    }
}

// ===================================================================
// 结构性终止检查测试
// ===================================================================

#[test]
fn test_structural_termination_ok() {
    // Standard factorial passes structural termination
    let arms = vec![
        RecursiveArm {
            pattern: RecursivePattern::Zero,
            result: MonoType::Int(1),
        },
        RecursiveArm {
            pattern: RecursivePattern::Succ("n".to_string()),
            result: MonoType::TypeRef("Nat(Mul, Succ(n), factorial(n))".to_string()),
        },
    ];
    assert!(
        check_structural_termination("factorial", &arms).is_ok(),
        "Standard factorial should pass structural termination"
    );
}

#[test]
fn test_structural_termination_zero_branch_recursive() {
    // Zero branch with self-call → err
    let arms = vec![
        RecursiveArm {
            pattern: RecursivePattern::Zero,
            result: MonoType::TypeRef("factorial(Zero)".to_string()),
        },
        RecursiveArm {
            pattern: RecursivePattern::Succ("n".to_string()),
            result: MonoType::TypeRef("Nat(Mul, Succ(n), factorial(n))".to_string()),
        },
    ];
    assert!(
        check_structural_termination("factorial", &arms).is_err(),
        "Zero branch with self-call should fail"
    );
}

#[test]
fn test_structural_termination_succ_wrong_arg() {
    // Succ(n) with func(Succ(n)) → err
    let arms = vec![
        RecursiveArm {
            pattern: RecursivePattern::Zero,
            result: MonoType::Int(1),
        },
        RecursiveArm {
            pattern: RecursivePattern::Succ("n".to_string()),
            result: MonoType::TypeRef("factorial(Succ(n))".to_string()),
        },
    ];
    assert!(
        check_structural_termination("factorial", &arms).is_err(),
        "Succ(n) with factorial(Succ(n)) should fail"
    );
}

#[test]
fn test_structural_termination_missing_zero() {
    // Missing Zero branch → err
    let arms = vec![RecursiveArm {
        pattern: RecursivePattern::Succ("n".to_string()),
        result: MonoType::TypeRef("factorial(n)".to_string()),
    }];
    assert!(
        check_structural_termination("fib", &arms).is_err(),
        "Missing Zero branch should fail"
    );
}

#[test]
fn test_structural_termination_missing_succ() {
    // Missing Succ branch → err
    let arms = vec![RecursiveArm {
        pattern: RecursivePattern::Zero,
        result: MonoType::Int(0),
    }];
    assert!(
        check_structural_termination("constant", &arms).is_err(),
        "Missing Succ branch should fail"
    );
}
