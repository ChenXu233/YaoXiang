//! 错误码定义
//!
//! E8xxx: 内部编译器错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E8xxx 错误码列表
pub static E8XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E8001",
        category: ErrorCategory::Internal,
        message_template: "Internal compiler error: {message}",
    },
    ErrorCodeDefinition {
        code: "E8002",
        category: ErrorCategory::Internal,
        message_template: "Unexpected compiler panic: {reason}",
    },
    ErrorCodeDefinition {
        code: "E8003",
        category: ErrorCategory::Internal,
        message_template: "Compiler phase error: {phase} - {message}",
    },
];

// E8xxx 快捷方法
impl ErrorCodeDefinition {
    /// E8001 内部编译器错误
    pub fn internal_error(message: &str) -> DiagnosticBuilder {
        let def = Self::find("E8001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("message", message)
    }

    /// E8002 意外 panic
    pub fn unexpected_panic(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E8002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("reason", reason)
    }

    /// E8003 编译器阶段错误
    pub fn compiler_phase_error(
        phase: &str,
        message: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E8003").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("phase", phase)
            .param("message", message)
    }
}
