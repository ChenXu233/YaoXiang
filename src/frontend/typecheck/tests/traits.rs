//! Trait 系统测试 (RFC-011)
//!
//! 测试 Trait 求解器功能：
//! - 内置特质求解
//! - 用户定义 Trait 支持
//! - 约束传播

use crate::frontend::core::type_system::MonoType;
use crate::frontend::typecheck::traits::{TraitSolver, TraitConstraint};
use crate::frontend::type_level::trait_bounds::{TraitTable, TraitDefinition, TraitImplementation};
use crate::util::span::{Position, Span};

fn create_dummy_span() -> Span {
    Span::new(Position::dummy(), Position::dummy())
}

/// 测试内置特质求解
#[test]
fn test_builtin_trait_solver() {
    let mut solver = TraitSolver::new();

    // 求解 Clone 约束
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Int(64)],
    };

    let result = solver.solve(&constraint);
    assert!(result.is_ok());
}

/// 测试内置特质检查
#[test]
fn test_builtin_trait_check() {
    let mut solver = TraitSolver::new();

    // 检查 Clone - Int 应该是可 Clone 的
    assert!(solver.check_trait(&MonoType::Int(64), "Clone"));

    // 检查 Clone - String 应该是可 Clone 的
    assert!(solver.check_trait(&MonoType::String, "Clone"));

    // 检查 Debug - 所有类型都应该有 Debug
    assert!(solver.check_trait(&MonoType::Int(64), "Debug"));
}

/// 测试用户定义 Trait
#[test]
fn test_user_defined_trait() {
    let mut solver = TraitSolver::new();
    let mut table = TraitTable::new();

    // 添加用户定义的 Trait
    let trait_def = TraitDefinition {
        name: "MyTrait".to_string(),
        methods: std::collections::HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: Some(create_dummy_span()),
    };
    table.add_trait(trait_def);

    // 添加 Trait 实现
    let impl_ = TraitImplementation {
        trait_name: "MyTrait".to_string(),
        for_type_name: "MyStruct".to_string(),
        methods: std::collections::HashMap::new(),
    };
    table.add_impl(impl_);

    // 设置 Trait 表
    solver.set_trait_table(table);

    // 检查 MyTrait 实现
    let my_struct_type = MonoType::TypeRef("MyStruct".to_string());
    assert!(solver.check_trait(&my_struct_type, "MyTrait"));
}

/// 测试 Trait 表克隆
#[test]
fn test_trait_table_clone() {
    let mut table = TraitTable::new();

    // 添加 Trait 定义
    let trait_def = TraitDefinition {
        name: "Clone".to_string(),
        methods: std::collections::HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: Some(create_dummy_span()),
    };
    table.add_trait(trait_def);

    // 克隆表
    let cloned = table.clone();

    // 验证克隆
    assert!(cloned.has_trait("Clone"));
}

/// 测试 Trait 求解器集成
#[test]
fn test_trait_solver_integration() {
    let mut solver = TraitSolver::new();
    let mut table = TraitTable::new();

    // 定义 MyTrait
    let trait_def = TraitDefinition {
        name: "MyTrait".to_string(),
        methods: std::collections::HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: Some(create_dummy_span()),
    };
    table.add_trait(trait_def);

    // 为 MyType 实现 MyTrait
    let impl_ = TraitImplementation {
        trait_name: "MyTrait".to_string(),
        for_type_name: "MyType".to_string(),
        methods: std::collections::HashMap::new(),
    };
    table.add_impl(impl_);

    solver.set_trait_table(table);

    // 求解约束
    let constraint = TraitConstraint {
        name: "MyTrait".to_string(),
        args: vec![MonoType::TypeRef("MyType".to_string())],
    };

    let result = solver.solve(&constraint);
    assert!(result.is_ok());
}

/// 测试 Send/Sync 特质
#[test]
fn test_send_sync_traits() {
    let mut solver = TraitSolver::new();

    // 基本类型应该是 Send
    assert!(solver.check_trait(&MonoType::Int(64), "Send"));
    assert!(solver.check_trait(&MonoType::Float(64), "Send"));
    assert!(solver.check_trait(&MonoType::Bool, "Send"));
    assert!(solver.check_trait(&MonoType::String, "Send"));

    // 基本类型应该是 Sync
    assert!(solver.check_trait(&MonoType::Int(64), "Sync"));
    assert!(solver.check_trait(&MonoType::Float(64), "Sync"));
    assert!(solver.check_trait(&MonoType::Bool, "Sync"));
    assert!(solver.check_trait(&MonoType::String, "Sync"));
}

/// 测试批量约束求解
#[test]
fn test_solve_all_constraints() {
    let mut solver = TraitSolver::new();

    let constraints = vec![
        TraitConstraint {
            name: "Clone".to_string(),
            args: vec![MonoType::Int(64)],
        },
        TraitConstraint {
            name: "Debug".to_string(),
            args: vec![MonoType::String],
        },
    ];

    let result = solver.solve_all(&constraints);
    assert!(result.is_ok());
}

/// 测试约束传播
#[test]
fn test_constraint_propagation() {
    let solver = TraitSolver::new();

    // 约束传播应该返回空列表（简化实现）
    let constraints = solver
        .propagate_constraints_to_type_args(&MonoType::TypeRef("Vec<T>".to_string()), "Clone");
    assert!(constraints.is_empty());
}
