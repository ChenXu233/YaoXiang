//! 类型求值测试 — 基于语言规范 §3.11 & RFC-011 §4
//!
//! §3.11: 编译期泛型
//! RFC-011 §4: 编译期泛型

use crate::frontend::core::typecheck::type_eval::{TypeEvaluator, EvalResult};
use crate::frontend::core::types::base::MonoType;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_type_evaluator_creation() {
    // Arrange & Act
    let evaluator = TypeEvaluator::new();
    let _evaluator = TypeEvaluator::new();

    // Assert - 应该成功创建
}

#[test]
fn test_type_evaluator_eval_simple_type() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();

    // Act
    let result = evaluator.eval(&MonoType::Int(32));

    // Assert
    assert!(
        matches!(result, EvalResult::Value(_)),
        "should eval simple type"
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

// 类型求值的错误路径测试

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_type_evaluator_complex_type() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();
    let complex_type = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Float(64)],
        return_type: Box::new(MonoType::String),
        is_async: false,
    };

    // Act
    let result = evaluator.eval(&complex_type);

    // Assert
    assert!(
        matches!(result, EvalResult::Value(_)),
        "should handle complex type"
    );
}
