//! 警告码定义
//!
//! W1xxx: 死代码相关警告

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

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

// 快捷方法实现
impl ErrorCodeDefinition {
    /// W1001 未使用的导出函数
    pub fn unused_function(name: &str) -> DiagnosticBuilder {
        let def = Self::find("W1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// W1002 未使用的导出类型
    pub fn unused_type(name: &str) -> DiagnosticBuilder {
        let def = Self::find("W1002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// W1003 未使用的导入
    pub fn unused_import(name: &str) -> DiagnosticBuilder {
        let def = Self::find("W1003").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// W1004 未使用的导出变量
    pub fn unused_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("W1004").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// W1005 未使用的导出方法
    pub fn unused_method(name: &str) -> DiagnosticBuilder {
        let def = Self::find("W1005").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }
}
