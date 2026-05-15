//! 类型错误测试 — 基于语言规范 §3
//!
//! §3: 类型分类与错误定义
//! TypeMismatch, TypeConstraintError, ConstEvalError 的创建、相等性、Display 格式

use crate::frontend::core::types::base::{ConstEvalError, MonoType, TypeConstraintError, TypeMismatch};
use crate::util::span::Span;

// ===== TypeMismatch 测试 =====

#[test]
fn test_type_mismatch_creation() {
    let err = TypeMismatch {
        left: MonoType::Int(32),
        right: MonoType::String,
        span: Span::dummy(),
    };
    assert_eq!(err.left, MonoType::Int(32), "left should be Int(32)");
    assert_eq!(err.right, MonoType::String, "right should be String");
}

#[test]
fn test_type_mismatch_equality() {
    let err1 = TypeMismatch {
        left: MonoType::Int(32),
        right: MonoType::String,
        span: Span::dummy(),
    };
    let err2 = TypeMismatch {
        left: MonoType::Int(32),
        right: MonoType::String,
        span: Span::dummy(),
    };
    assert_eq!(err1, err2, "identical TypeMismatch should be equal");
}

#[test]
fn test_type_mismatch_inequality() {
    let err1 = TypeMismatch {
        left: MonoType::Int(32),
        right: MonoType::String,
        span: Span::dummy(),
    };
    let err2 = TypeMismatch {
        left: MonoType::Bool,
        right: MonoType::Float(64),
        span: Span::dummy(),
    };
    assert_ne!(err1, err2, "different TypeMismatch should not be equal");
}

#[test]
fn test_type_mismatch_display_format() {
    let err = TypeMismatch {
        left: MonoType::Int(32),
        right: MonoType::String,
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(
        msg.contains("expected"),
        "display should contain 'expected'"
    );
    assert!(msg.contains("found"), "display should contain 'found'");
    assert!(msg.contains("int32"), "display should contain 'int32'");
    assert!(msg.contains("string"), "display should contain 'string'");
}

#[test]
fn test_type_mismatch_display_bool_vs_float() {
    let err = TypeMismatch {
        left: MonoType::Bool,
        right: MonoType::Float(64),
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("bool"), "display should contain 'bool'");
    assert!(msg.contains("float64"), "display should contain 'float64'");
}

#[test]
fn test_type_mismatch_debug_format() {
    let err = TypeMismatch {
        left: MonoType::Int(32),
        right: MonoType::String,
        span: Span::dummy(),
    };
    let debug = format!("{:?}", err);
    assert!(
        debug.contains("TypeMismatch"),
        "debug should contain type name"
    );
    assert!(debug.contains("Int"), "debug should contain variant names");
}

// ===== TypeConstraintError 测试 =====

#[test]
fn test_type_constraint_error_creation() {
    let inner = TypeMismatch {
        left: MonoType::Bool,
        right: MonoType::Int(64),
        span: Span::dummy(),
    };
    let err = TypeConstraintError {
        error: inner.clone(),
        span: Span::dummy(),
    };
    assert_eq!(err.error.left, MonoType::Bool, "inner left should be Bool");
    assert_eq!(
        err.error.right,
        MonoType::Int(64),
        "inner right should be Int(64)"
    );
}

#[test]
fn test_type_constraint_error_display() {
    let err = TypeConstraintError {
        error: TypeMismatch {
            left: MonoType::Bool,
            right: MonoType::Int(64),
            span: Span::dummy(),
        },
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(
        msg.contains("expected"),
        "display should contain 'expected'"
    );
    assert!(msg.contains("bool"), "display should contain 'bool'");
}

#[test]
fn test_type_constraint_error_debug_format() {
    let err = TypeConstraintError {
        error: TypeMismatch {
            left: MonoType::Int(32),
            right: MonoType::Float(64),
            span: Span::dummy(),
        },
        span: Span::dummy(),
    };
    let debug = format!("{:?}", err);
    assert!(
        debug.contains("TypeConstraintError"),
        "debug should contain type name"
    );
}

// ===== ConstEvalError 测试 =====

#[test]
fn test_const_eval_error_division_by_zero_display() {
    let err = ConstEvalError::DivisionByZero {
        span: Span::dummy(),
    };
    assert_eq!(format!("{}", err), "division by zero");
}

#[test]
fn test_const_eval_error_overflow_display() {
    let err = ConstEvalError::Overflow {
        value: "999".to_string(),
        ty: "Int8".to_string(),
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(
        msg.contains("overflow"),
        "display should contain 'overflow'"
    );
    assert!(msg.contains("999"), "display should contain value");
    assert!(msg.contains("Int8"), "display should contain type");
}

#[test]
fn test_const_eval_error_undefined_variable_display() {
    let err = ConstEvalError::UndefinedVariable {
        name: "x".to_string(),
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(
        msg.contains("undefined"),
        "display should contain 'undefined'"
    );
    assert!(msg.contains("x"), "display should contain variable name");
}

#[test]
fn test_const_eval_error_recursion_too_deep_display() {
    let err = ConstEvalError::RecursionTooDeep {
        depth: 100,
        max_depth: 50,
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("100"), "display should contain depth");
    assert!(msg.contains("50"), "display should contain max_depth");
    assert!(
        msg.contains("recursion"),
        "display should contain 'recursion'"
    );
}

#[test]
fn test_const_eval_error_type_mismatch_display() {
    let err = ConstEvalError::TypeMismatch {
        expected: "Int".to_string(),
        found: "String".to_string(),
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(
        msg.contains("expected"),
        "display should contain 'expected'"
    );
    assert!(msg.contains("Int"), "display should contain expected type");
    assert!(msg.contains("String"), "display should contain found type");
}

#[test]
fn test_const_eval_error_arg_count_mismatch_display() {
    let err = ConstEvalError::ArgCountMismatch {
        expected: 2,
        found: 3,
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("2"), "display should contain expected count");
    assert!(msg.contains("3"), "display should contain found count");
}

#[test]
fn test_const_eval_error_non_const_function_call_display() {
    let err = ConstEvalError::NonConstFunctionCall {
        func: "rand".to_string(),
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("rand"), "display should contain function name");
    assert!(
        msg.contains("non-const"),
        "display should contain 'non-const'"
    );
}

#[test]
fn test_const_eval_error_cannot_evaluate_display() {
    let err = ConstEvalError::CannotEvaluate {
        reason: "side effect".to_string(),
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("side effect"), "display should contain reason");
    assert!(
        msg.contains("cannot evaluate"),
        "display should contain 'cannot evaluate'"
    );
}

#[test]
fn test_const_eval_error_debug_format() {
    let err = ConstEvalError::DivisionByZero {
        span: Span::dummy(),
    };
    let debug = format!("{:?}", err);
    assert!(
        debug.contains("DivisionByZero"),
        "debug should contain variant name"
    );
}

#[test]
fn test_const_eval_error_clone() {
    let err = ConstEvalError::Overflow {
        value: "128".to_string(),
        ty: "Int8".to_string(),
        span: Span::dummy(),
    };
    let cloned = err.clone();
    assert_eq!(
        format!("{}", err),
        format!("{}", cloned),
        "cloned error should display the same"
    );
}
