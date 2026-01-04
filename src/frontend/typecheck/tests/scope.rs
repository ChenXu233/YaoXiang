use crate::frontend::parser::ast;
use crate::frontend::typecheck::infer::TypeInferrer;
use crate::frontend::typecheck::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::util::span::Span;

#[test]
fn test_scope_shadowing() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // Global scope: x = Int
    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));

    // Enter local scope
    inferrer.enter_scope();

    // Local scope: x = String
    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::String));

    // Check x is String
    let poly = inferrer.get_var("x").unwrap().clone();
    let ty = inferrer.solver().instantiate(&poly);
    assert!(matches!(ty, MonoType::String));

    // Exit local scope
    inferrer.exit_scope();

    // Check x is Int again
    let poly = inferrer.get_var("x").unwrap().clone();
    let ty = inferrer.solver().instantiate(&poly);
    assert!(matches!(ty, MonoType::Int(64)));
}

#[test]
fn test_nested_scopes() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // Global: a = Int
    inferrer.add_var("a".to_string(), PolyType::mono(MonoType::Int(64)));

    inferrer.enter_scope();
    // Level 1: b = Float
    inferrer.add_var("b".to_string(), PolyType::mono(MonoType::Float(64)));

    // Can see a
    assert!(inferrer.get_var("a").is_some());

    inferrer.enter_scope();
    // Level 2: c = Bool
    inferrer.add_var("c".to_string(), PolyType::mono(MonoType::Bool));

    // Can see a and b
    assert!(inferrer.get_var("a").is_some());
    assert!(inferrer.get_var("b").is_some());
    assert!(inferrer.get_var("c").is_some());

    inferrer.exit_scope();
    // Back to Level 1
    assert!(inferrer.get_var("c").is_none());
    assert!(inferrer.get_var("b").is_some());

    inferrer.exit_scope();
    // Back to Global
    assert!(inferrer.get_var("b").is_none());
    assert!(inferrer.get_var("a").is_some());
}
