//! 类型错误测试 — 基于语言规范 §3
//!
//! §3: 类型分类与错误定义
//! TypeMismatch, TypeConstraintError 的创建、相等性、Display 格式

use crate::frontend::core::types::base::{MonoType, TypeConstraintError, TypeMismatch};
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
