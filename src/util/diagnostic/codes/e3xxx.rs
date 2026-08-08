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

// 快捷方法（code_helpers! 生成）
impl ErrorCodeDefinition {
    code_helpers! {
    /// E3004 不支持的迭代器类型
    ("E3004", ir_unsupported_iterator(iter_type: &str) => .param("iter_type", iter_type)),
    /// E3005 IR 内部错误
    ("E3005", ir_internal_error(message: &str) => .param("message", message)),
    /// E3014 寄存器溢出
    ("E3014", register_overflow(id: &str, limit: &str) => .param("id", id).param("limit", limit)),
    /// E3017 无效操作数（代码生成）
    ("E3017", codegen_invalid_operand(reason: &str) => .param("reason", reason)),
    }
}
