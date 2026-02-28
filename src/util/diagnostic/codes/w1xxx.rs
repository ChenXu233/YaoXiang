//! 警告码定义
//!
//! W1xxx: 死代码相关警告

use super::{ErrorCategory, ErrorCodeDefinition};

/// W1xxx 警告码列表
pub static W1XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "W1001",
        category: ErrorCategory::Warning,
        message_template: "Unused exported function: '{name}'",
    },
    ErrorCodeDefinition {
        code: "W1002",
        category: ErrorCategory::Warning,
        message_template: "Unused exported type: '{name}'",
    },
    ErrorCodeDefinition {
        code: "W1003",
        category: ErrorCategory::Warning,
        message_template: "Unused import: '{name}'",
    },
    ErrorCodeDefinition {
        code: "W1004",
        category: ErrorCategory::Warning,
        message_template: "Unused exported variable: '{name}'",
    },
    ErrorCodeDefinition {
        code: "W1005",
        category: ErrorCategory::Warning,
        message_template: "Unused exported method: '{name}'",
    },
];
