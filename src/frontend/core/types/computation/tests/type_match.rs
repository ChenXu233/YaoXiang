use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::computation::type_match::{
    MatchArm, MatchBinding, MatchPattern, MatchType, PatternBuilder, PatternMatcher, nat_examples,
};

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
fn test_match_arm() {
    let arm = MatchArm::new(MatchPattern::wildcard(), MonoType::Int(32));
    assert!(arm.pattern.is_wildcard());
    assert_eq!(arm.result, MonoType::Int(32));
    let wc = MatchArm::wildcard(MonoType::String);
    assert!(wc.pattern.is_wildcard());
}

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
fn test_match_type() {
    let mt = MatchType::on(
        MonoType::TypeRef("Zero".to_string()),
        vec![
            MatchArm::new(
                MatchPattern::named("Zero"),
                MonoType::TypeRef("Nat".to_string()),
            ),
            MatchArm::wildcard(MonoType::TypeRef("Unknown".to_string())),
        ],
    );
    assert!(mt.has_wildcard());
    assert_eq!(mt.arm_count(), 2);
}

#[test]
fn test_pattern_builder() {
    let pat = PatternBuilder::new().wildcard(Some("a")).named("b").tuple();
    assert!(matches!(pat, MatchPattern::Tuple(ref p) if p.len() == 2));
}

#[test]
fn test_nat_examples() {
    let mt = nat_examples::add_type(
        MonoType::TypeRef("Zero".to_string()),
        MonoType::TypeRef("Nat".to_string()),
    );
    assert_eq!(mt.arm_count(), 2);
}

// ===================================================================
// RFC-011 §5: 模式匹配补充测试
// ===================================================================

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

#[test]
fn test_match_arm_new() {
    let arm = MatchArm::new(MatchPattern::named("Zero"), MonoType::Int(32));
    assert!(!arm.pattern.is_wildcard());
    assert_eq!(arm.result, MonoType::Int(32));
}

#[test]
fn test_match_arm_wildcard() {
    let arm = MatchArm::wildcard(MonoType::String);
    assert!(arm.pattern.is_wildcard());
    assert_eq!(arm.result, MonoType::String);
}

#[test]
fn test_match_type_on_creates_match() {
    let mt = MatchType::on(
        MonoType::TypeRef("Zero".to_string()),
        vec![
            MatchArm::new(MatchPattern::named("Zero"), MonoType::Int(32)),
            MatchArm::wildcard(MonoType::String),
        ],
    );
    assert_eq!(mt.arm_count(), 2);
    assert!(mt.has_wildcard());
}

#[test]
fn test_match_type_add_arm() {
    let mut mt = MatchType::on(MonoType::Int(32), vec![]);
    assert_eq!(mt.arm_count(), 0);
    mt.add_arm(MatchPattern::wildcard(), MonoType::String);
    assert_eq!(mt.arm_count(), 1);
}

#[test]
fn test_match_type_with_wildcard() {
    let mut mt = MatchType::on(MonoType::Int(32), vec![]);
    assert!(!mt.has_wildcard());
    mt.with_wildcard(MonoType::String);
    assert!(mt.has_wildcard());
    assert_eq!(mt.arm_count(), 1);
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
