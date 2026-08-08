//! 错误码定义
//!
//! E5xxx: 模块与导入阶段的错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E5xxx 错误码列表
pub static E5XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E5001",
        category: ErrorCategory::Module,
    },
    ErrorCodeDefinition {
        code: "E5002",
        category: ErrorCategory::Module,
    },
    ErrorCodeDefinition {
        code: "E5003",
        category: ErrorCategory::Module,
    },
    ErrorCodeDefinition {
        code: "E5004",
        category: ErrorCategory::Module,
    },
    ErrorCodeDefinition {
        code: "E5005",
        category: ErrorCategory::Module,
    },
    ErrorCodeDefinition {
        code: "E5006",
        category: ErrorCategory::Module,
    },
    ErrorCodeDefinition {
        code: "E5007",
        category: ErrorCategory::Module,
    },
];

// 快捷方法（code_helpers! 生成）
impl ErrorCodeDefinition {
    code_helpers! {
    /// E5001 模块未找到
    ("E5001", module_not_found(module: &str) => .param("module", module)),
    /// E5002 导入错误
    ("E5002", import_error(module: &str, reason: &str) => .param("module", module) .param("reason", reason)),
    /// E5003 导出未找到
    ("E5003", export_not_found(export: &str, module: &str) => .param("export", export) .param("module", module)),
    /// E5004 循环依赖
    ("E5004", circular_dependency(path: &str) => .param("path", path)),
    /// E5005 无效的模块路径
    ("E5005", invalid_module_path(path: &str) => .param("path", path)),
    /// E5006 重复导入
    ("E5006", duplicate_import(name: &str) => .param("name", name)),
    /// E5007 模块导出提示（用于辅助错误消息）
    ("E5007", module_exports_hint(module: &str, available: &str) => .param("module", module) .param("available", available)),
    }
}
