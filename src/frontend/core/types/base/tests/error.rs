//! 类型错误测试 — 基于语言规范 §3
//!
//! §3: 类型分类与错误定义
//! TypeConstraintError 的创建、Display 格式

use crate::frontend::core::types::base::TypeConstraintError;
use crate::util::diagnostic::ErrorCodeDefinition;
use crate::util::span::Span;

// ===== TypeConstraintError 测试 =====

#[test]
fn test_type_constraint_error_creation() {
    let inner = ErrorCodeDefinition::type_mismatch("bool", "int64").build();
    let err = TypeConstraintError {
        error: inner,
        span: Span::dummy(),
    };
    assert!(
        err.error.message.contains("bool"),
        "error message should contain 'bool'"
    );
    assert!(
        err.error.message.contains("int64"),
        "error message should contain 'int64'"
    );
}

#[test]
fn test_type_constraint_error_display() {
    let inner = ErrorCodeDefinition::type_mismatch("bool", "int64").build();
    let err = TypeConstraintError {
        error: inner,
        span: Span::dummy(),
    };
    let msg = format!("{}", err);
    assert!(msg.contains("bool"), "display should contain 'bool'");
}

#[test]
fn test_type_constraint_error_debug_format() {
    let inner = ErrorCodeDefinition::type_mismatch("int32", "float64").build();
    let err = TypeConstraintError {
        error: inner,
        span: Span::dummy(),
    };
    let debug = format!("{:?}", err);
    assert!(
        debug.contains("TypeConstraintError"),
        "debug should contain type name"
    );
}
