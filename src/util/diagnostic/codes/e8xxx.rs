//! 错误码定义
//!
//! E8xxx: 内部编译器错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E8xxx 错误码列表
pub static E8XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E8001",
        category: ErrorCategory::Internal,
    },
    ErrorCodeDefinition {
        code: "E8002",
        category: ErrorCategory::Internal,
    },
    ErrorCodeDefinition {
        code: "E8003",
        category: ErrorCategory::Internal,
    },
];

// 快捷方法（code_helpers! 生成）
impl ErrorCodeDefinition {
    code_helpers! {
    /// E8001 内部编译器错误
    ("E8001", internal_error(message: &str) => .param("message", message)),
    /// E8002 意外 panic
    ("E8002", unexpected_panic(reason: &str) => .param("reason", reason)),
    /// E8003 编译器阶段错误
    ("E8003", compiler_phase_error(phase: &str, message: &str) => .param("phase", phase) .param("message", message)),
    }
}
