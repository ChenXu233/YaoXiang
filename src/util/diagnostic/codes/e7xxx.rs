//! 错误码定义
//!
//! E7xxx: I/O 与系统错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E7xxx 错误码列表
pub static E7XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E7001",
        category: ErrorCategory::Io,
    },
    ErrorCodeDefinition {
        code: "E7002",
        category: ErrorCategory::Io,
    },
    ErrorCodeDefinition {
        code: "E7003",
        category: ErrorCategory::Io,
    },
    ErrorCodeDefinition {
        code: "E7004",
        category: ErrorCategory::Io,
    },
];

// 快捷方法（code_helpers! 生成）
impl ErrorCodeDefinition {
    code_helpers! {
    /// E7001 文件未找到
    ("E7001", file_not_found(path: &str) => .param("path", path)),
    /// E7002 权限被拒绝
    ("E7002", permission_denied(path: &str) => .param("path", path)),
    /// E7003 I/O 错误
    ("E7003", io_error(reason: &str) => .param("reason", reason)),
    /// E7004 网络错误
    ("E7004", network_error(reason: &str) => .param("reason", reason)),
    }
}
