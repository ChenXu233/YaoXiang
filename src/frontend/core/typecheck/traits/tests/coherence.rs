//! 一致性测试 — 基于语言规范 §3.5.1
//!
//! §3.5.1: 接口实现唯一性规则
//!
//! 一致性检查确保：
//! - 同一类型对同一接口只能实现一次（唯一性规则）
//! - 孤儿规则（orphan rule）防止跨模块的实现冲突
//!
//! 注意：当前实现为简化版本，一致性检查逻辑待完善。

use crate::frontend::core::typecheck::traits::coherence::{CoherenceChecker, OrphanChecker};

// ===================================================================
// CoherenceChecker — Happy path 测试
// ===================================================================

#[test]
fn test_coherence_checker_creation() {
    // Arrange & Act
    let checker = CoherenceChecker::new();

    // Assert — 创建后 check 应通过
    assert!(checker.check().is_ok(), "新创建的一致性检查器 check 应通过");
}

#[test]
fn test_coherence_checker_default_trait() {
    // Arrange & Act
    let checker = CoherenceChecker;

    // Assert
    assert!(
        checker.check().is_ok(),
        "通过 Default 创建的检查器 check 应通过"
    );
}

#[test]
fn test_coherence_checker_check_passes() {
    // Arrange
    let checker = CoherenceChecker::new();

    // Act
    let result = checker.check();

    // Assert
    assert!(result.is_ok(), "基本一致性检查应通过");
}

// ===================================================================
// CoherenceChecker — Error path 测试
// ===================================================================

// 当前简化实现始终返回 Ok，错误路径测试预留

// ===================================================================
// CoherenceChecker — Boundary 测试
// ===================================================================

#[test]
fn test_coherence_checker_multiple_checks_idempotent() {
    // Arrange
    let checker = CoherenceChecker::new();

    // Act — 连续多次调用 check
    let r1 = checker.check();
    let r2 = checker.check();
    let r3 = checker.check();

    // Assert
    assert!(r1.is_ok(), "第一次 check 应通过");
    assert!(r2.is_ok(), "第二次 check 应通过");
    assert!(r3.is_ok(), "第三次 check 应通过");
}

// ===================================================================
// OrphanChecker — Happy path 测试
// ===================================================================

#[test]
fn test_orphan_checker_creation() {
    // Arrange & Act
    let checker = OrphanChecker::new();

    // Assert
    assert!(checker.check().is_ok(), "新创建的孤儿检查器 check 应通过");
}

#[test]
fn test_orphan_checker_default_trait() {
    // Arrange & Act
    let checker = OrphanChecker;

    // Assert
    assert!(
        checker.check().is_ok(),
        "通过 Default 创建的孤儿检查器 check 应通过"
    );
}

#[test]
fn test_orphan_checker_check_passes() {
    // Arrange
    let checker = OrphanChecker::new();

    // Act
    let result = checker.check();

    // Assert
    assert!(result.is_ok(), "基本孤儿检查应通过");
}

// ===================================================================
// OrphanChecker — Error path 测试
// ===================================================================

// 当前简化实现始终返回 Ok，错误路径测试预留

// ===================================================================
// OrphanChecker — Boundary 测试
// ===================================================================

#[test]
fn test_orphan_checker_multiple_checks_idempotent() {
    // Arrange
    let checker = OrphanChecker::new();

    // Act
    let r1 = checker.check();
    let r2 = checker.check();

    // Assert
    assert!(r1.is_ok(), "第一次 check 应通过");
    assert!(r2.is_ok(), "第二次 check 应通过");
}

#[test]
fn test_orphan_checker_independent_instances() {
    // Arrange
    let checker_a = OrphanChecker::new();
    let checker_b = OrphanChecker::new();

    // Act
    let result_a = checker_a.check();
    let result_b = checker_b.check();

    // Assert
    assert!(result_a.is_ok(), "第一个实例 check 应通过");
    assert!(result_b.is_ok(), "第二个实例 check 应通过");
}
