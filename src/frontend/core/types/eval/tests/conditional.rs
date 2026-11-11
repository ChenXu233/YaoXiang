//! 条件类型与模式匹配测试 — 基于 RFC-011 §5
//!
//! §5.1: TypeCondition 所有变体 (Bool, Eq, Neq, IsVoid, IsNever, IsType, And, Or, Not)
//! §5.1: If[C, T, E] 条件类型选择
//! §5.2: 模式匹配 (MatchPattern, PatternMatcher, PatternMatchType, PatternBuilder)
//! §5.2: 类型族 (TypeFamily, Nat, Bool)

use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::eval::conditional::{
    conditions, If, TypeCondition, MatchType, MatchPattern, MatchBinding, PatternBuilder,
    PatternMatcher, PatternMatchType, PatternMatchArm, nat_examples,
};
use crate::frontend::core::types::eval::TypeLevelResult;

// §5.1: TypeCondition — 所有条件变体

/// Helper: test_condition_and_or_not 的完整夹具与断言（逐条断言见函数体）。
/// Assert: And 真值表（t&t=true、t&f=false、f&t=false）。
fn assert_and_truth_table(
    t: &TypeCondition,
    f: &TypeCondition,
) {
    assert_eq!(
        TypeCondition::And(Box::new(t.clone()), Box::new(t.clone())).eval(),
        Some(true)
    );
    assert_eq!(
        TypeCondition::And(Box::new(t.clone()), Box::new(f.clone())).eval(),
        Some(false)
    );
    assert_eq!(
        TypeCondition::And(Box::new(f.clone()), Box::new(t.clone())).eval(),
        Some(false)
    );
}

/// Assert: Or 真值表（t|f=true、f|f=false、t|t=true）。
fn assert_or_truth_table(
    t: &TypeCondition,
    f: &TypeCondition,
) {
    assert_eq!(
        TypeCondition::Or(Box::new(t.clone()), Box::new(f.clone())).eval(),
        Some(true)
    );
    assert_eq!(
        TypeCondition::Or(Box::new(f.clone()), Box::new(f.clone())).eval(),
        Some(false)
    );
    assert_eq!(
        TypeCondition::Or(Box::new(t.clone()), Box::new(t.clone())).eval(),
        Some(true)
    );
}

/// Assert: Not 真值表（!t=false、!f=true、!!t=true）。
fn assert_not_truth_table(
    t: &TypeCondition,
    f: &TypeCondition,
) {
    assert_eq!(TypeCondition::Not(Box::new(t.clone())).eval(), Some(false));
    assert_eq!(TypeCondition::Not(Box::new(f.clone())).eval(), Some(true));
    assert_eq!(
        TypeCondition::Not(Box::new(TypeCondition::Not(Box::new(t.clone())))).eval(),
        Some(true)
    );
}

#[test]
fn test_condition_bool() {
    assert_eq!(TypeCondition::Bool(true).eval(), Some(true));
    assert!(TypeCondition::Bool(true).is_determined());
    assert_eq!(TypeCondition::Bool(false).eval(), Some(false));
}

#[test]
fn test_condition_and_or_not() {
    // Arrange — 布尔真值对
    let t = TypeCondition::Bool(true);
    let f = TypeCondition::Bool(false);

    // Act & Assert — And / Or / Not 各自的真值表
    assert_and_truth_table(&t, &f);
    assert_or_truth_table(&t, &f);
    assert_not_truth_table(&t, &f);
}

#[test]
fn test_condition_is_determined() {
    assert!(TypeCondition::Bool(true).is_determined());
    assert!(TypeCondition::Bool(false).is_determined());
}

#[test]
fn test_if_getters() {
    let if_type = If::new(
        TypeCondition::Bool(true),
        MonoType::Int(32),
        MonoType::make_string(),
    );
    assert_eq!(if_type.condition(), &TypeCondition::Bool(true));
    assert_eq!(if_type.true_branch(), &MonoType::Int(32));
    assert_eq!(if_type.false_branch(), &MonoType::make_string());
}

// §5.1: conditions 辅助函数

#[test]
fn test_conditions_helper_bool() {
    assert_eq!(conditions::bool(true).eval(), Some(true));
    assert_eq!(conditions::bool(false).eval(), Some(false));
}

#[test]
fn test_conditions_helper_type_checks() {
    assert!(conditions::is_void(MonoType::Void).eval() == Some(true));
    assert!(conditions::is_never(MonoType::Never).eval() == Some(true));
}

#[test]
fn test_conditions_helper_logic() {
    assert_eq!(
        conditions::and(conditions::bool(true), conditions::bool(true)).eval(),
        Some(true)
    );
    assert_eq!(
        conditions::or(conditions::bool(false), conditions::bool(true)).eval(),
        Some(true)
    );
    assert_eq!(conditions::not(conditions::bool(true)).eval(), Some(false));
}

// §5.1: TypeLevelResult

#[test]
fn test_type_level_result_normalized() {
    let r: TypeLevelResult<MonoType> = TypeLevelResult::Normalized(MonoType::Int(32));
    assert!(r.is_normalized());
    assert_eq!(r.ok(), Some(MonoType::Int(32)));
}

#[test]
fn test_type_level_result_normalized_result() {
    let r: TypeLevelResult<MonoType> = TypeLevelResult::Normalized(MonoType::Int(32));
    assert!(r.result().is_ok());
}

#[test]
fn test_type_level_result_error_result() {
    let err = TypeLevelResult::<MonoType>::Error(
        crate::frontend::core::types::eval::TypeLevelError::ComputationFailed("test".to_string()),
    );
    assert!(err.result().is_err());
}

#[test]
fn test_conditional_match_type_with_wildcard() {
    let mt = MatchType::with_wildcard(MonoType::Int(32), MonoType::Bool);
    assert!(mt.eval().is_normalized());
}

#[test]
fn test_conditional_match_type_add_arm() {
    let mut mt = MatchType::new(MonoType::Int(32), vec![]);
    mt.add_arm(MonoType::Int(32), MonoType::make_string());
    assert_eq!(mt.arms.len(), 1);
}

// §5.2: MatchPattern — 模式匹配

#[test]
fn test_match_pattern() {
    assert!(MatchPattern::wildcard().is_wildcard());
    assert!(MatchPattern::wildcard_named("x").is_wildcard());
    assert!(!MatchPattern::named("Zero").is_wildcard());
    assert!(!MatchPattern::zero().is_wildcard());
    assert!(!MatchPattern::succ(MatchPattern::zero()).is_wildcard());
    assert!(!MatchPattern::t().is_wildcard());
    assert!(!MatchPattern::f().is_wildcard());
    let lit = MatchPattern::literal(MonoType::Int(32));
    assert_eq!(lit.as_literal(), Some(&MonoType::Int(32)));
}

#[test]
fn test_match_pattern_as_literal_extended() {
    let lit = MatchPattern::literal(MonoType::Int(32));
    assert_eq!(lit.as_literal(), Some(&MonoType::Int(32)));
    let named = MatchPattern::named("x");
    assert_eq!(named.as_literal(), None);
}

#[test]
fn test_match_pattern_constructors() {
    let zero = MatchPattern::zero();
    assert!(!zero.is_wildcard());
    let succ = MatchPattern::succ(MatchPattern::zero());
    assert!(!succ.is_wildcard());
    let t = MatchPattern::t();
    assert!(!t.is_wildcard());
    let f = MatchPattern::f();
    assert!(!f.is_wildcard());
}

#[test]
fn test_match_pattern_tuple_with_elements() {
    let pat = MatchPattern::tuple(vec![MatchPattern::named("a"), MatchPattern::named("b")]);
    assert!(!pat.is_wildcard());
    assert!(matches!(pat, MatchPattern::Tuple(ref p) if p.len() == 2));
}

#[test]
fn test_match_pattern_constructor() {
    let pat = MatchPattern::constructor("Some", vec![MatchPattern::named("x")]);
    assert!(!pat.is_wildcard());
}

#[test]
fn test_pattern_match_arm_new() {
    let arm = PatternMatchArm::new(MatchPattern::named("Zero"), MonoType::Int(32));
    assert!(!arm.pattern.is_wildcard());
    assert_eq!(arm.result, MonoType::Int(32));
}

// §5.2: MatchBinding

#[test]
fn test_match_binding() {
    let mut b = MatchBinding::new();
    assert_eq!(b.get("x"), None);
    b.bind("x", MonoType::Int(32));
    assert_eq!(b.get("x"), Some(&MonoType::Int(32)));
    b.bind("x", MonoType::make_string());
    assert_eq!(b.get("x"), Some(&MonoType::make_string()));
    let mut c = MatchBinding::new();
    c.bind("y", MonoType::Bool);
    let merged = b.merge(&c);
    assert_eq!(merged.get("x"), Some(&MonoType::make_string()));
    assert_eq!(merged.get("y"), Some(&MonoType::Bool));
}

#[test]
fn test_pattern_matcher_tuple_arity() {
    let m = PatternMatcher::new();
    assert!(m.matches(
        &MonoType::make_tuple(vec![MonoType::Int(32), MonoType::make_string()]),
        &MatchPattern::tuple(vec![MatchPattern::wildcard(), MatchPattern::wildcard()])
    ));
    assert!(!m.matches(
        &MonoType::make_tuple(vec![MonoType::Int(32)]),
        &MatchPattern::tuple(vec![MatchPattern::wildcard(), MatchPattern::wildcard()])
    ));
}

// §5.2: PatternMatchType

#[test]
fn test_pattern_match_type() {
    let mt = PatternMatchType::on(
        MonoType::TypeRef("Zero".to_string()),
        vec![
            PatternMatchArm::new(
                MatchPattern::named("Zero"),
                MonoType::TypeRef("Nat".to_string()),
            ),
            PatternMatchArm::wildcard(MonoType::TypeRef("Unknown".to_string())),
        ],
    );
    assert!(mt.has_wildcard());
    assert_eq!(mt.arm_count(), 2);
}

// §5.2: PatternBuilder

#[test]
fn test_pattern_builder() {
    let pat = PatternBuilder::new().wildcard(Some("a")).named("b").tuple();
    assert!(matches!(pat, MatchPattern::Tuple(ref p) if p.len() == 2));
}

// §5.2: nat_examples

#[test]
fn test_nat_examples() {
    let mt = nat_examples::add_type(
        MonoType::TypeRef("Zero".to_string()),
        MonoType::TypeRef("Nat".to_string()),
    );
    assert_eq!(mt.arm_count(), 2);
}
