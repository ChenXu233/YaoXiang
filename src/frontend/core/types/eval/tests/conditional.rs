//! 条件类型与模式匹配测试 — 基于 RFC-011 §5
//!
//! §5.1: TypeCondition 所有变体 (Bool, Eq, Neq, IsVoid, IsNever, IsType, And, Or, Not)
//! §5.1: If[C, T, E] 条件类型选择
//! §5.2: 模式匹配 (MatchPattern, PatternMatcher, PatternMatchType, PatternBuilder)
//! §5.2: 类型族 (TypeFamily, Nat, Bool)

use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::eval::conditional::{
    conditions, If, TypeCondition, MatchType, MatchArm, ConditionalType, MatchPattern,
    MatchBinding, PatternBuilder, PatternMatcher, PatternMatchType, PatternMatchArm, nat_examples,
};
use crate::frontend::core::types::eval::TypeLevelResult;

// ===================================================================
// §5.1: TypeCondition — 所有条件变体
// ===================================================================

#[test]
fn test_condition_bool() {
    assert_eq!(TypeCondition::Bool(true).eval(), Some(true));
    assert!(TypeCondition::Bool(true).is_determined());
    assert_eq!(TypeCondition::Bool(false).eval(), Some(false));
}

#[test]
fn test_condition_eq() {
    let eq = |a, b| TypeCondition::Eq(Box::new(a), Box::new(b));
    assert_eq!(eq(MonoType::Int(32), MonoType::Int(32)).eval(), Some(true));
    assert_eq!(eq(MonoType::Int(32), MonoType::String).eval(), Some(false));
    assert_eq!(eq(MonoType::Bool, MonoType::Bool).eval(), Some(true));
    assert_eq!(eq(MonoType::Void, MonoType::Bool).eval(), Some(false));
}

#[test]
fn test_condition_neq() {
    let nq = |a, b| TypeCondition::Neq(Box::new(a), Box::new(b));
    assert_eq!(nq(MonoType::Int(32), MonoType::String).eval(), Some(true));
    assert_eq!(nq(MonoType::Int(32), MonoType::Int(32)).eval(), Some(false));
}

#[test]
fn test_condition_is_void() {
    let iv = |t| TypeCondition::IsVoid(Box::new(t));
    assert_eq!(iv(MonoType::Void).eval(), Some(true));
    assert_eq!(iv(MonoType::Int(32)).eval(), Some(false));
    assert_eq!(iv(MonoType::String).eval(), Some(false));
}

#[test]
fn test_condition_is_never() {
    let inn = |t| TypeCondition::IsNever(Box::new(t));
    assert_eq!(
        inn(MonoType::TypeRef("Never".to_string())).eval(),
        Some(true)
    );
    // Non-"Never" TypeRef returns false
    assert_eq!(
        inn(MonoType::TypeRef("Int".to_string())).eval(),
        Some(false)
    );
    assert_eq!(inn(MonoType::Int(32)).eval(), Some(false));
}

#[test]
fn test_condition_is_type() {
    let it = |t, e| TypeCondition::IsType(Box::new(t), Box::new(e));
    assert_eq!(it(MonoType::Int(32), MonoType::Int(32)).eval(), Some(true));
    assert_eq!(it(MonoType::Int(32), MonoType::String).eval(), Some(false));
}

#[test]
fn test_condition_and_or_not() {
    let t = TypeCondition::Bool(true);
    let f = TypeCondition::Bool(false);

    // And
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

    // Or
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

    // Not
    assert_eq!(TypeCondition::Not(Box::new(t.clone())).eval(), Some(false));
    assert_eq!(TypeCondition::Not(Box::new(f.clone())).eval(), Some(true));
    assert_eq!(
        TypeCondition::Not(Box::new(TypeCondition::Not(Box::new(t.clone())))).eval(),
        Some(true)
    );
}

#[test]
fn test_condition_is_determined() {
    assert!(TypeCondition::Bool(true).is_determined());
    assert!(TypeCondition::Bool(false).is_determined());
}

// ===================================================================
// §5.1: If[C, T, E] — 条件类型
// ===================================================================

#[test]
fn test_if_true_selects_true_branch() {
    let if_type = If::new(
        TypeCondition::Bool(true),
        MonoType::Int(32),
        MonoType::String,
    );
    let result = if_type.eval();
    assert!(result.is_normalized());
    assert_eq!(result.ok(), Some(MonoType::Int(32)));
}

#[test]
fn test_if_false_selects_false_branch() {
    let if_type = If::new(
        TypeCondition::Bool(false),
        MonoType::Int(32),
        MonoType::String,
    );
    let result = if_type.eval();
    assert_eq!(result.ok(), Some(MonoType::String));
}

#[test]
fn test_if_with_eq_condition() {
    let cond = TypeCondition::Eq(Box::new(MonoType::Int(32)), Box::new(MonoType::Int(32)));
    let if_type = If::new(cond, MonoType::Bool, MonoType::Void);
    assert_eq!(if_type.eval().ok(), Some(MonoType::Bool));
}

#[test]
fn test_if_getters() {
    let if_type = If::new(
        TypeCondition::Bool(true),
        MonoType::Int(32),
        MonoType::String,
    );
    assert_eq!(if_type.condition(), &TypeCondition::Bool(true));
    assert_eq!(if_type.true_branch(), &MonoType::Int(32));
    assert_eq!(if_type.false_branch(), &MonoType::String);
}

// ===================================================================
// §5.1: conditions 辅助函数
// ===================================================================

#[test]
fn test_conditions_helper_bool() {
    assert_eq!(conditions::bool(true).eval(), Some(true));
    assert_eq!(conditions::bool(false).eval(), Some(false));
}

#[test]
fn test_conditions_helper_eq_and_neq() {
    assert_eq!(
        conditions::eq(MonoType::Int(32), MonoType::Int(32)).eval(),
        Some(true)
    );
    assert_eq!(
        conditions::neq(MonoType::Int(32), MonoType::String).eval(),
        Some(true)
    );
}

#[test]
fn test_conditions_helper_type_checks() {
    assert!(conditions::is_void(MonoType::Void).eval() == Some(true));
    assert!(conditions::is_never(MonoType::TypeRef("Never".to_string())).eval() == Some(true));
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

// ===================================================================
// §5.1: TypeLevelResult
// ===================================================================

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
fn test_type_level_result_pending() {
    let r: TypeLevelResult<MonoType> = TypeLevelResult::Pending(MonoType::TypeRef("T".to_string()));
    assert!(!r.is_normalized());
    assert_eq!(r.ok(), Some(MonoType::TypeRef("T".to_string())));
}

#[test]
fn test_type_level_result_pending_result() {
    let r: TypeLevelResult<MonoType> = TypeLevelResult::Pending(MonoType::TypeRef("T".to_string()));
    assert!(r.result().is_ok());
}

#[test]
fn test_type_level_result_error() {
    let err = TypeLevelResult::<MonoType>::Error(
        crate::frontend::core::types::eval::TypeLevelError::ComputationFailed("test".to_string()),
    );
    assert!(!err.is_normalized());
    assert_eq!(err.ok(), None);
}

#[test]
fn test_type_level_result_error_result() {
    let err = TypeLevelResult::<MonoType>::Error(
        crate::frontend::core::types::eval::TypeLevelError::ComputationFailed("test".to_string()),
    );
    assert!(err.result().is_err());
}

// ===================================================================
// §5.1: MatchType (conditional_types) 补充测试
// ===================================================================

#[test]
fn test_conditional_match_type_new_and_eval() {
    // MatchType with wildcard pattern: pattern_matches returns true for non-TypeVar
    let mt = MatchType::new(
        MonoType::Int(32),
        vec![MatchArm {
            pattern: MonoType::TypeRef("_".to_string()),
            result: MonoType::String,
        }],
    );
    let result = mt.eval();
    assert!(result.is_normalized());
    assert_eq!(result.ok(), Some(MonoType::String));
}

#[test]
fn test_conditional_match_type_with_wildcard() {
    let mt = MatchType::with_wildcard(MonoType::Int(32), MonoType::Bool);
    assert!(mt.eval().is_normalized());
}

#[test]
fn test_conditional_match_type_add_arm() {
    let mut mt = MatchType::new(MonoType::Int(32), vec![]);
    mt.add_arm(MonoType::Int(32), MonoType::String);
    assert_eq!(mt.arms.len(), 1);
}

#[test]
fn test_conditional_match_type_var_pattern_no_match() {
    use crate::frontend::core::types::TypeVar;
    // TypeVar pattern returns false -> no match -> error
    let mt = MatchType::new(
        MonoType::Int(32),
        vec![MatchArm {
            pattern: MonoType::TypeVar(TypeVar::new(0)),
            result: MonoType::String,
        }],
    );
    let result = mt.eval();
    assert!(!result.is_normalized()); // Error or Pending
}

#[test]
fn test_conditional_type_match_variant() {
    let mt = MatchType::with_wildcard(MonoType::Void, MonoType::Int(32));
    let ct = ConditionalType::Match(mt);
    assert!(ct.eval().is_normalized());
}

// ===================================================================
// §5.2: MatchPattern — 模式匹配
// ===================================================================

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
fn test_match_pattern_is_wildcard_extended() {
    assert!(
        MatchPattern::wildcard().is_wildcard(),
        "wildcard should be wildcard"
    );
    assert!(
        MatchPattern::wildcard_named("x").is_wildcard(),
        "wildcard_named should be wildcard"
    );
    assert!(
        !MatchPattern::named("Zero").is_wildcard(),
        "named should not be wildcard"
    );
    assert!(
        !MatchPattern::literal(MonoType::Int(32)).is_wildcard(),
        "literal should not be wildcard"
    );
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

// ===================================================================
// §5.2: PatternMatchArm
// ===================================================================

#[test]
fn test_pattern_match_arm() {
    let arm = PatternMatchArm::new(MatchPattern::wildcard(), MonoType::Int(32));
    assert!(arm.pattern.is_wildcard());
    assert_eq!(arm.result, MonoType::Int(32));
    let wc = PatternMatchArm::wildcard(MonoType::String);
    assert!(wc.pattern.is_wildcard());
}

#[test]
fn test_pattern_match_arm_new() {
    let arm = PatternMatchArm::new(MatchPattern::named("Zero"), MonoType::Int(32));
    assert!(!arm.pattern.is_wildcard());
    assert_eq!(arm.result, MonoType::Int(32));
}

#[test]
fn test_pattern_match_arm_wildcard() {
    let arm = PatternMatchArm::wildcard(MonoType::String);
    assert!(arm.pattern.is_wildcard());
    assert_eq!(arm.result, MonoType::String);
}

// ===================================================================
// §5.2: MatchBinding
// ===================================================================

#[test]
fn test_match_binding() {
    let mut b = MatchBinding::new();
    assert_eq!(b.get("x"), None);
    b.bind("x", MonoType::Int(32));
    assert_eq!(b.get("x"), Some(&MonoType::Int(32)));
    b.bind("x", MonoType::String);
    assert_eq!(b.get("x"), Some(&MonoType::String));
    let mut c = MatchBinding::new();
    c.bind("y", MonoType::Bool);
    let merged = b.merge(&c);
    assert_eq!(merged.get("x"), Some(&MonoType::String));
    assert_eq!(merged.get("y"), Some(&MonoType::Bool));
}

#[test]
fn test_match_binding_bind_and_get() {
    let mut b = MatchBinding::new();
    assert_eq!(b.get("x"), None);
    b.bind("x", MonoType::Int(32));
    assert_eq!(b.get("x"), Some(&MonoType::Int(32)));
}

#[test]
fn test_match_binding_overwrite() {
    let mut b = MatchBinding::new();
    b.bind("x", MonoType::Int(32));
    b.bind("x", MonoType::String);
    assert_eq!(b.get("x"), Some(&MonoType::String));
}

// ===================================================================
// §5.2: PatternMatcher
// ===================================================================

#[test]
fn test_pattern_matcher() {
    let m = PatternMatcher::new();
    assert!(m.matches(&MonoType::Int(32), &MatchPattern::wildcard()));
    assert!(m.matches(
        &MonoType::Tuple(vec![MonoType::Int(32)]),
        &MatchPattern::tuple(vec![MatchPattern::wildcard()])
    ));
    assert!(!m.matches(
        &MonoType::Tuple(vec![MonoType::Int(32)]),
        &MatchPattern::tuple(vec![MatchPattern::wildcard(), MatchPattern::wildcard()])
    ));
}

#[test]
fn test_pattern_matcher_named() {
    let m = PatternMatcher::new();
    // MatchPattern::named creates a Constructor pattern - doesn't match arbitrary types
    assert!(!m.matches(&MonoType::Int(32), &MatchPattern::named("x")));
    // MatchPattern::wildcard_named creates a Wildcard pattern - matches anything
    assert!(m.matches(&MonoType::Int(32), &MatchPattern::wildcard_named("x")));
    assert!(m.matches(&MonoType::String, &MatchPattern::wildcard_named("x")));
}

#[test]
fn test_pattern_matcher_literal() {
    let m = PatternMatcher::new();
    assert!(m.matches(
        &MonoType::Int(32),
        &MatchPattern::literal(MonoType::Int(32))
    ));
    assert!(!m.matches(
        &MonoType::Int(64),
        &MatchPattern::literal(MonoType::Int(32))
    ));
}

#[test]
fn test_pattern_matcher_tuple_arity() {
    let m = PatternMatcher::new();
    assert!(m.matches(
        &MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]),
        &MatchPattern::tuple(vec![MatchPattern::wildcard(), MatchPattern::wildcard()])
    ));
    assert!(!m.matches(
        &MonoType::Tuple(vec![MonoType::Int(32)]),
        &MatchPattern::tuple(vec![MatchPattern::wildcard(), MatchPattern::wildcard()])
    ));
}

// ===================================================================
// §5.2: PatternMatchType
// ===================================================================

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

#[test]
fn test_pattern_match_type_on_creates_match() {
    let mt = PatternMatchType::on(
        MonoType::TypeRef("Zero".to_string()),
        vec![
            PatternMatchArm::new(MatchPattern::named("Zero"), MonoType::Int(32)),
            PatternMatchArm::wildcard(MonoType::String),
        ],
    );
    assert_eq!(mt.arm_count(), 2);
    assert!(mt.has_wildcard());
}

#[test]
fn test_pattern_match_type_add_arm() {
    let mut mt = PatternMatchType::on(MonoType::Int(32), vec![]);
    assert_eq!(mt.arm_count(), 0);
    mt.add_arm(MatchPattern::wildcard(), MonoType::String);
    assert_eq!(mt.arm_count(), 1);
}

#[test]
fn test_pattern_match_type_with_wildcard() {
    let mut mt = PatternMatchType::on(MonoType::Int(32), vec![]);
    assert!(!mt.has_wildcard());
    mt.with_wildcard(MonoType::String);
    assert!(mt.has_wildcard());
    assert_eq!(mt.arm_count(), 1);
}

// ===================================================================
// §5.2: PatternBuilder
// ===================================================================

#[test]
fn test_pattern_builder() {
    let pat = PatternBuilder::new().wildcard(Some("a")).named("b").tuple();
    assert!(matches!(pat, MatchPattern::Tuple(ref p) if p.len() == 2));
}

#[test]
fn test_pattern_builder_wildcard() {
    let pat = PatternBuilder::new().wildcard(None).build();
    assert!(pat.is_wildcard());
}

#[test]
fn test_pattern_builder_named() {
    // PatternBuilder::named uses wildcard_named internally
    let pat = PatternBuilder::new().named("x").build();
    assert!(
        pat.is_wildcard(),
        "builder named uses wildcard_named internally"
    );
}

#[test]
fn test_pattern_builder_tuple() {
    let pat = PatternBuilder::new().wildcard(None).named("x").tuple();
    assert!(matches!(pat, MatchPattern::Tuple(ref p) if p.len() == 2));
}

// ===================================================================
// §5.2: nat_examples
// ===================================================================

#[test]
fn test_nat_examples() {
    let mt = nat_examples::add_type(
        MonoType::TypeRef("Zero".to_string()),
        MonoType::TypeRef("Nat".to_string()),
    );
    assert_eq!(mt.arm_count(), 2);
}
