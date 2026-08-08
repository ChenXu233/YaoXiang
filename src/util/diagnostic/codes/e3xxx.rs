//! 错误码定义
//!
//! E3xxx: 代码生成阶段的错误码
//!
//! E3004-E3005: IR 生成（ir_gen）
//! E3014/E3017: 字节码生成（codegen）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E3xxx 错误码列表
pub static E3XXX: &[ErrorCodeDefinition] = &[
    // === E3004-E3005: IR 生成 ===
    ErrorCodeDefinition {
        code: "E3004",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3005",
        category: ErrorCategory::Codegen,
    },
    // === E3014/E3017: 字节码生成 ===
    ErrorCodeDefinition {
        code: "E3014",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3017",
        category: ErrorCategory::Codegen,
    },
];

// E3xxx 快捷方法
impl ErrorCodeDefinition {
    /// E3004 不支持的迭代器类型
    pub fn ir_unsupported_iterator(iter_type: &str) -> DiagnosticBuilder {
        let def = Self::find("E3004").unwrap();
        def.builder().param("iter_type", iter_type)
    }

    /// E3005 IR 内部错误
    pub fn ir_internal_error(message: &str) -> DiagnosticBuilder {
        let def = Self::find("E3005").unwrap();
        def.builder().param("message", message)
    }

    // === 字节码生成 ===

    /// E3014 寄存器溢出
    pub fn register_overflow(
        id: &str,
        limit: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E3014").unwrap();
        def.builder().param("id", id).param("limit", limit)
    }

    /// E3017 无效操作数（代码生成）
    pub fn codegen_invalid_operand(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E3017").unwrap();
        def.builder().param("reason", reason)
    }
}
