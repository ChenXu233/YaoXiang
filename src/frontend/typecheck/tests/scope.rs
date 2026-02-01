//! 作用域管理测试

use crate::frontend::core::parser::ast;
use crate::frontend::core::type_system::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::typecheck::inference::ExprInferrer;
use crate::util::span::Span;

#[test]
fn test_scope_shadowing() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = ExprInferrer::new(&mut solver);

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
    let mut inferrer = ExprInferrer::new(&mut solver);

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

/// 测试作用域级别变化
#[test]
fn test_scope_level_changes() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = ExprInferrer::new(&mut solver);

    // 初始为全局作用域
    assert_eq!(inferrer.scope_level(), 1);

    // 进入嵌套作用域
    inferrer.enter_scope();
    assert_eq!(inferrer.scope_level(), 2);

    inferrer.enter_scope();
    assert_eq!(inferrer.scope_level(), 3);

    // 退出作用域
    inferrer.exit_scope();
    assert_eq!(inferrer.scope_level(), 2);

    inferrer.exit_scope();
    assert_eq!(inferrer.scope_level(), 1);

    // 不能退出全局作用域
    inferrer.exit_scope();
    assert_eq!(inferrer.scope_level(), 1);
}

/// 测试变量在作用域间正确隔离
#[test]
fn test_variable_isolation_between_scopes() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = ExprInferrer::new(&mut solver);

    // 在全局作用域添加变量
    inferrer.add_var("global".to_string(), PolyType::mono(MonoType::Int(64)));

    // 进入局部作用域
    inferrer.enter_scope();

    // 在局部作用域添加同名变量
    inferrer.add_var("global".to_string(), PolyType::mono(MonoType::String));

    // 局部作用域应该返回 String
    let local_var = inferrer.get_var("global").unwrap();
    assert_eq!(local_var.body, MonoType::String);

    // 退出局部作用域
    inferrer.exit_scope();

    // 全局作用域应该返回 Int
    let global_var = inferrer.get_var("global").unwrap();
    assert_eq!(global_var.body, MonoType::Int(64));
}

/// 测试空作用域退出
#[test]
fn test_exit_empty_scope() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = ExprInferrer::new(&mut solver);

    // 初始作用域
    assert_eq!(inferrer.scope_level(), 1);

    // 进入一个空作用域并退出
    inferrer.enter_scope();
    assert_eq!(inferrer.scope_level(), 2);
    inferrer.exit_scope();
    assert_eq!(inferrer.scope_level(), 1);

    // 验证全局变量仍然可用
    inferrer.add_var("test".to_string(), PolyType::mono(MonoType::Bool));
    assert!(inferrer.get_var("test").is_some());
}
