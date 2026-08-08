//! 警告码定义
//!
//! W1xxx: 死代码相关警告

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// W1xxx 警告码列表
pub static W1XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "W1001",
        category: ErrorCategory::Warning,
    },
    ErrorCodeDefinition {
        code: "W1002",
        category: ErrorCategory::Warning,
    },
    ErrorCodeDefinition {
        code: "W1003",
        category: ErrorCategory::Warning,
    },
    ErrorCodeDefinition {
        code: "W1004",
        category: ErrorCategory::Warning,
    },
    ErrorCodeDefinition {
        code: "W1005",
        category: ErrorCategory::Warning,
    },
];

// 快捷方法实现

// 快捷方法（code_helpers! 生成）
impl ErrorCodeDefinition {
    code_helpers! {
    /// W1001 未使用的导出函数
    ("W1001", unused_function(name: &str) => .param("name", name)),
    /// W1002 未使用的导出类型
    ("W1002", unused_type(name: &str) => .param("name", name)),
    /// W1003 未使用的导入
    ("W1003", unused_import(name: &str) => .param("name", name)),
    /// W1004 未使用的导出变量
    ("W1004", unused_variable(name: &str) => .param("name", name)),
    /// W1005 未使用的导出方法
    ("W1005", unused_method(name: &str) => .param("name", name)),
    }
}
