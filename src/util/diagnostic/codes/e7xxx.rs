//! 错误码定义
//!
//! E7xxx: I/O 与系统错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E7xxx 错误码列表
pub static E7XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E7001",
        category: ErrorCategory::Io,
        message_template: "File not found: '{path}'",
    },
    ErrorCodeDefinition {
        code: "E7002",
        category: ErrorCategory::Io,
        message_template: "Permission denied: '{path}'",
    },
    ErrorCodeDefinition {
        code: "E7003",
        category: ErrorCategory::Io,
        message_template: "I/O error: {reason}",
    },
    ErrorCodeDefinition {
        code: "E7004",
        category: ErrorCategory::Io,
        message_template: "Network error: {reason}",
    },
];

// E7xxx 快捷方法
impl ErrorCodeDefinition {
    /// E7001 文件未找到
    pub fn file_not_found(path: &str) -> DiagnosticBuilder {
        let def = Self::find("E7001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("path", path)
    }

    /// E7002 权限被拒绝
    pub fn permission_denied(path: &str) -> DiagnosticBuilder {
        let def = Self::find("E7002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("path", path)
    }

    /// E7003 I/O 错误
    pub fn io_error(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E7003").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("reason", reason)
    }

    /// E7004 网络错误
    pub fn network_error(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E7004").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("reason", reason)
    }
}
