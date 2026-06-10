//! 借用令牌系统测试 — 基于 RFC-009 v9 §2.1（引用语义与类型属性），
//! RFC-011 §2.4（Dup 蕴含 Clone）
//!
//! RFC-009 v9 核心语义：
//! - &T 是零大小令牌，可自由复制（Dup）
//! - &mut T 是线性令牌，不可复制（非 Dup）
//! - Dup 蕴含 Clone（RFC-011 §2.4）

use crate::frontend::core::typecheck::traits::solver::{TraitConstraint, TraitSolver};
use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::trait_data::TraitTable;

/// 创建预配置的 TraitSolver，设置空 TraitTable。
fn make_solver() -> TraitSolver {
    let mut solver = TraitSolver::new();
    solver.set_trait_table(TraitTable::new());
    solver
}

// ===================================================================
// Dup 语义测试
// ===================================================================

#[test]
fn test_dup_immutable_ref_is_dup() {
    // Arrange
    let mut solver = make_solver();
    let ref_ty = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Int(64)),
    };

    // Act
    let result = solver.check_trait(&ref_ty, "Dup");

    // Assert
    assert!(result, "&T 应满足 Dup");
}

#[test]
fn test_dup_mutable_ref_is_not_dup() {
    // Arrange
    let mut solver = make_solver();
    let mut_ref_ty = MonoType::Ref {
        mutable: true,
        inner: Box::new(MonoType::Int(64)),
    };

    // Act
    let result = solver.check_trait(&mut_ref_ty, "Dup");

    // Assert
    assert!(!result, "&mut T 不应满足 Dup");
}

#[test]
fn test_dup_ref_to_string_is_dup() {
    // Arrange
    let mut solver = make_solver();
    let ref_ty = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::String),
    };

    // Act
    let result = solver.check_trait(&ref_ty, "Dup");

    // Assert
    assert!(result, "&String 应满足 Dup");
}

#[test]
fn test_dup_ref_to_arc_is_dup() {
    // Arrange
    let mut solver = make_solver();
    let ref_ty = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Arc(Box::new(MonoType::Int(64)))),
    };

    // Act
    let result = solver.check_trait(&ref_ty, "Dup");

    // Assert
    assert!(result, "&Arc<Int> 应满足 Dup");
}

#[test]
fn test_dup_tuple_with_ref_is_dup() {
    // Arrange - 所有元素都是 Dup 类型：&T + String
    let mut solver = make_solver();
    let tuple_ty = MonoType::Tuple(vec![
        MonoType::Ref {
            mutable: false,
            inner: Box::new(MonoType::Int(64)),
        },
        MonoType::String,
    ]);

    // Act
    let result = solver.check_trait(&tuple_ty, "Dup");

    // Assert
    assert!(result, "(&T, String) 应满足 Dup");
}

// ===================================================================
// Dup solve() 路径测试
// ===================================================================

#[test]
fn test_dup_solve_ref_via_constraint() {
    // Arrange
    let mut solver = make_solver();
    let ref_ty = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Bool),
    };

    // Act
    let result = solver.solve(&TraitConstraint {
        name: "Dup".to_string(),
        args: vec![ref_ty],
    });

    // Assert
    assert!(result.is_ok(), "solve(&Bool: Dup) 应成功");
}

#[test]
fn test_dup_solve_mut_ref_fails() {
    // Arrange
    let mut solver = make_solver();
    let mut_ref_ty = MonoType::Ref {
        mutable: true,
        inner: Box::new(MonoType::Bool),
    };

    // Act
    let result = solver.solve(&TraitConstraint {
        name: "Dup".to_string(),
        args: vec![mut_ref_ty],
    });

    // Assert
    assert!(result.is_err(), "solve(&mut Bool: Dup) 应失败");
}

// ===================================================================
// Clone 与 Dup 关系测试（RFC-011 §2.4: Dup implies Clone）
// ===================================================================

#[test]
fn test_clone_ref_is_clone() {
    // RFC-011 §2.4: Dup implies Clone，所以 &T 应该 Clone
    // Arrange
    let mut solver = make_solver();
    let ref_ty = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Int(64)),
    };

    // Act
    let result = solver.check_trait(&ref_ty, "Clone");

    // Assert
    assert!(result, "&T 应满足 Clone（Dup 蕴含 Clone）");
}
