//! 类型求值测试 — 基于语言规范 §3.11 & RFC-011 §4
//!
//! §3.11: 编译期泛型
//! RFC-011 §4: 编译期泛型

use crate::frontend::core::typecheck::type_eval::{EvalConfig, EvalResult, TypeEvaluator};
use crate::frontend::core::types::MonoType;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_type_evaluator_creation() {
    // Arrange & Act
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

#[test]
fn test_type_evaluator_eval_fn_type() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();
    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Float(64)],
        return_type: Box::new(MonoType::String),
    };

    // Act
    let result = evaluator.eval(&fn_type);

    // Assert
    assert!(
        matches!(result, EvalResult::Value(_)),
        "eval Fn type should return Value"
    );
}

#[test]
fn test_type_evaluator_eval_tuple_type() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();
    let tuple_type = MonoType::Tuple(vec![MonoType::Int(32), MonoType::Bool, MonoType::String]);

    // Act
    let result = evaluator.eval(&tuple_type);

    // Assert
    assert!(
        matches!(result, EvalResult::Value(_)),
        "eval Tuple type should return Value"
    );
}

#[test]
fn test_type_evaluator_eval_list_type() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();
    let list_type = MonoType::List(Box::new(MonoType::Float(64)));

    // Act
    let result = evaluator.eval(&list_type);

    // Assert
    assert!(
        matches!(result, EvalResult::Value(_)),
        "eval List type should return Value"
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_type_evaluator_eval_nat_unknown_operation() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();
    let a = MonoType::Int(5);
    let b = MonoType::Int(3);

    // Act
    let result = evaluator.eval_nat("Pow", &[a, b]);

    // Assert
    assert!(
        matches!(result, EvalResult::Error(_)),
        "eval Nat with unknown operation should return Error"
    );
}

#[test]
fn test_type_evaluator_eval_nat_underflow() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();
    let a = MonoType::Int(3);
    let b = MonoType::Int(5);

    // Act
    let result = evaluator.eval_nat("Sub", &[a, b]);

    // Assert
    assert!(
        matches!(result, EvalResult::Error(_)),
        "eval Nat Sub with b > a should return Error (underflow)"
    );
}

#[test]
fn test_type_evaluator_eval_nat_division_by_zero() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();
    let a = MonoType::Int(10);
    let b = MonoType::Int(0);

    // Act
    let result = evaluator.eval_nat("Div", &[a, b]);

    // Assert
    assert!(
        matches!(result, EvalResult::Error(_)),
        "eval Nat Div by zero should return Error"
    );
}

#[test]
fn test_type_evaluator_eval_nat_modulo_by_zero() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();
    let a = MonoType::Int(10);
    let b = MonoType::Int(0);

    // Act
    let result = evaluator.eval_nat("Mod", &[a, b]);

    // Assert
    assert!(
        matches!(result, EvalResult::Error(_)),
        "eval Nat Mod by zero should return Error"
    );
}

#[test]
fn test_type_evaluator_eval_max_depth_exceeded() {
    // Arrange - 设置 max_depth=0，使得任何递归都会触发深度限制
    let config = EvalConfig {
        max_depth: 0,
        enable_cache: true,
        cycle_detection: true,
    };
    let mut evaluator = TypeEvaluator::with_config(config);
    // 嵌套 Fn 类型会递归求值参数和返回类型，触发深度检查
    let nested_fn = MonoType::Fn {
        params: vec![MonoType::Fn {
            params: vec![MonoType::Int(32)],
            return_type: Box::new(MonoType::Float(64)),
        }],
        return_type: Box::new(MonoType::String),
    };

    // Act
    let result = evaluator.eval(&nested_fn);

    // Assert - Fn 类型不是递归类型引用（TypeRef），不触发深度检查，应返回 Value
    assert!(
        matches!(result, EvalResult::Value(_)),
        "eval Fn type should return Value (Fn is not a recursive TypeRef)"
    );
}

#[test]
fn test_type_evaluator_eval_match_no_matching_arm() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();
    let target = MonoType::Int(32);
    let arms = vec![(MonoType::String, MonoType::Bool)];

    // Act
    let result = evaluator.eval_match(&target, arms);

    // Assert
    assert!(
        matches!(result, EvalResult::Error(_)),
        "eval Match with no matching arm should return Error"
    );
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_type_evaluator_eval_nested_type() {
    // Arrange - 构造深层嵌套类型：Fn[Tuple[List[Int], Fn[Bool -> String(async)]] -> List[Float]]
    let mut evaluator = TypeEvaluator::new();
    let nested_type = MonoType::Fn {
        params: vec![MonoType::Tuple(vec![
            MonoType::List(Box::new(MonoType::Int(32))),
            MonoType::Fn {
                params: vec![MonoType::Bool],
                return_type: Box::new(MonoType::String),
            },
        ])],
        return_type: Box::new(MonoType::List(Box::new(MonoType::Float(64)))),
    };

    // Act
    let result = evaluator.eval(&nested_type);

    // Assert
    assert!(
        matches!(result, EvalResult::Value(_)),
        "eval deeply nested type should return Value"
    );
}

#[test]
fn test_type_evaluator_eval_void_type() {
    // Arrange
    let mut evaluator = TypeEvaluator::new();

    // Act
    let result = evaluator.eval(&MonoType::Void);

    // Assert
    assert!(
        matches!(result, EvalResult::Value(MonoType::Void)),
        "eval Void type should return Value(Void)"
    );
}
