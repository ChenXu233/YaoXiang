//! 条件类型测试 — 基于 RFC-011 §5 (条件类型)
//!
//! §5.1: TypeCondition 所有变体 (Bool, Eq, Neq, IsVoid, IsNever, IsType, And, Or, Not)
//! §5.1: If[C, T, E] 条件类型选择
//! §5.2: 类型族 (TypeFamily, Nat, Bool)

use crate::frontend::core::types::base::MonoType;
use crate::frontend::core::types::computation::conditional_types::{conditions, If, TypeCondition};
use crate::frontend::core::types::computation::TypeLevelResult;

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
        crate::frontend::core::types::computation::TypeLevelError::ComputationFailed(
            "test".to_string(),
        ),
    );
    assert!(!err.is_normalized());
    assert_eq!(err.ok(), None);
}

#[test]
fn test_type_level_result_error_result() {
    let err = TypeLevelResult::<MonoType>::Error(
        crate::frontend::core::types::computation::TypeLevelError::ComputationFailed(
            "test".to_string(),
        ),
    );
    assert!(err.result().is_err());
}

// ============ MatchType (conditional_types) 补充测试 ============

#[test]
fn test_conditional_match_type_new_and_eval() {
    use crate::frontend::core::types::computation::conditional_types::MatchType;
    // MatchType with wildcard pattern: pattern_matches returns true for non-TypeVar
    let mt = MatchType::new(
        MonoType::Int(32),
        vec![
            crate::frontend::core::types::computation::conditional_types::MatchArm {
                pattern: MonoType::TypeRef("_".to_string()),
                result: MonoType::String,
            },
        ],
    );
    let result = mt.eval();
    assert!(result.is_normalized());
    assert_eq!(result.ok(), Some(MonoType::String));
}

#[test]
fn test_conditional_match_type_with_wildcard() {
    use crate::frontend::core::types::computation::conditional_types::MatchType;
    let mt = MatchType::with_wildcard(MonoType::Int(32), MonoType::Bool);
    assert!(mt.eval().is_normalized());
}

#[test]
fn test_conditional_match_type_add_arm() {
    use crate::frontend::core::types::computation::conditional_types::MatchType;
    let mut mt = MatchType::new(MonoType::Int(32), vec![]);
    mt.add_arm(MonoType::Int(32), MonoType::String);
    assert_eq!(mt.arms.len(), 1);
}

#[test]
fn test_conditional_match_type_var_pattern_no_match() {
    use crate::frontend::core::types::computation::conditional_types::MatchType;
    use crate::frontend::core::types::base::TypeVar;
    // TypeVar pattern returns false → no match → error
    let mt = MatchType::new(
        MonoType::Int(32),
        vec![
            crate::frontend::core::types::computation::conditional_types::MatchArm {
                pattern: MonoType::TypeVar(TypeVar::new(0)),
                result: MonoType::String,
            },
        ],
    );
    let result = mt.eval();
    assert!(!result.is_normalized()); // Error or Pending
}

#[test]
fn test_conditional_type_match_variant() {
    use crate::frontend::core::types::computation::conditional_types::{ConditionalType, MatchType};
    let mt = MatchType::with_wildcard(MonoType::Void, MonoType::Int(32));
    let ct = ConditionalType::Match(mt);
    assert!(ct.eval().is_normalized());
}
