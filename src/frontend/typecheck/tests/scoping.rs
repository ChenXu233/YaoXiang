//! 作用域管理测试
//!
//! 合并自: scope.rs, shadowing.rs, use_scope.rs, use_block_scope.rs
//! 测试: 变量遮蔽、嵌套作用域、模块导入作用域、块作用域

use std::collections::HashMap;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::ast;
use crate::frontend::core::parser::parse;
use crate::frontend::core::type_system::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::typecheck::inference::ExprInferrer;
use crate::frontend::typecheck::inference::ScopeManager;
use crate::frontend::typecheck::overload;
use crate::frontend::typecheck::TypeChecker;

// ============================================================================
// 作用域基础 (scope.rs)
// ============================================================================

#[test]
fn test_scope_shadowing() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut scope = ScopeManager::new();
    let mut inferrer = ExprInferrer::new(&mut scope, &mut solver, &overload_candidates);

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
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut scope = ScopeManager::new();
    let mut inferrer = ExprInferrer::new(&mut scope, &mut solver, &overload_candidates);

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

// ============================================================================
// 变量遮蔽 (shadowing.rs)
// ============================================================================

#[test]
fn test_var_shadowing_basic() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut scope = ScopeManager::new();
    let mut inferrer = ExprInferrer::new(&mut scope, &mut solver, &overload_candidates);

    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));
    assert!(inferrer.get_var("x").is_some());

    inferrer.enter_scope();
    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::String));
    let poly = inferrer.get_var("x").unwrap().clone();
    let ty = inferrer.solver().instantiate(&poly);
    assert_eq!(ty, MonoType::String);
    inferrer.exit_scope();

    let poly = inferrer.get_var("x").unwrap().clone();
    let ty = inferrer.solver().instantiate(&poly);
    assert_eq!(ty, MonoType::Int(64));
}

#[test]
fn test_var_shadowing_multi_level() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut scope = ScopeManager::new();
    let mut inferrer = ExprInferrer::new(&mut scope, &mut solver, &overload_candidates);

    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));
    inferrer.enter_scope();
    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::Float(64)));
    inferrer.enter_scope();
    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::String));

    let poly = inferrer.get_var("x").unwrap().clone();
    let ty = inferrer.solver().instantiate(&poly);
    assert_eq!(ty, MonoType::String);

    inferrer.exit_scope();
    let poly = inferrer.get_var("x").unwrap().clone();
    let ty = inferrer.solver().instantiate(&poly);
    assert_eq!(ty, MonoType::Float(64));

    inferrer.exit_scope();
    let poly = inferrer.get_var("x").unwrap().clone();
    let ty = inferrer.solver().instantiate(&poly);
    assert_eq!(ty, MonoType::Int(64));
}

#[test]
fn test_var_not_shadowed_in_sibling_scopes() {
    let mut solver = TypeConstraintSolver::new();
    let overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>> = HashMap::new();
    let mut scope = ScopeManager::new();
    let mut inferrer = ExprInferrer::new(&mut scope, &mut solver, &overload_candidates);

    inferrer.add_var("x".to_string(), PolyType::mono(MonoType::Int(64)));

    inferrer.enter_scope();
    inferrer.add_var("y".to_string(), PolyType::mono(MonoType::Float(64)));
    inferrer.exit_scope();

    // y should be gone after exiting scope
    assert!(inferrer.get_var("y").is_none());
    // x should still be visible
    assert!(inferrer.get_var("x").is_some());
}

#[test]
fn test_shadow_in_for_loop_scope() {
    let code = r#"
main = {
    mut acc = 0
    for i in 1..5 {
        acc = acc + i
    }
}
"#;
    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();
    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(&module);
    assert!(
        result.is_ok(),
        "for loop should compile: {:?}",
        result.err()
    );
}

// ============================================================================
// 模块导入作用域 (use_scope.rs)
// ============================================================================

fn typecheck(code: &str) -> Result<(), Vec<crate::util::diagnostic::Diagnostic>> {
    let tokens = tokenize(code).unwrap();
    let module = parse(&tokens).unwrap();
    let mut checker = TypeChecker::new("test");
    checker.check_module(&module).map(|_| ())
}

#[test]
fn test_use_module_inside_function_scope() {
    let code = r#"
main = {
    use std.string
    pos = string.index_of("hello", "l")
}
"#;
    assert!(typecheck(code).is_ok());
}

#[test]
fn test_use_module_alias_inside_function_scope() {
    let code = r#"
main = {
    use std.string as str
    pos = str.index_of("hello", "l")
}
"#;
    assert!(typecheck(code).is_ok());
}

// ============================================================================
// 块作用域 (use_block_scope.rs)
// ============================================================================

#[test]
fn test_use_inside_block_not_visible_outside() {
    let code = r#"
main = {
    if true {
        use std.string
        pos = string.index_of("hello", "l")
    }
    pos2 = string.index_of("world", "o")
    return
}
"#;
    let errors = typecheck(code).expect_err("string should be out of scope outside block");
    assert!(
        errors.iter().any(|e| e.code == "E1001"),
        "expected E1001 unknown variable, got: {errors:?}"
    );
}

#[test]
fn test_use_inside_block_stays_available_in_same_block() {
    let code = r#"
main = {
    if true {
        use std.string
        a = string.index_of("hello", "l")
        b = string.index_of("world", "o")
    }
    return
}
"#;
    assert!(typecheck(code).is_ok());
}

#[test]
fn test_use_inside_function_not_visible_in_other_function() {
    let code = r#"
f = {
    use std.string
    _ = string.index_of("hello", "l")
    return
}

g = {
    x = string.index_of("world", "o")
    return
}
"#;
    let errors =
        typecheck(code).expect_err("function-local use should not leak to another function");
    assert!(
        errors.iter().any(|e| e.code == "E1001"),
        "expected E1001 unknown variable, got: {errors:?}"
    );
}
