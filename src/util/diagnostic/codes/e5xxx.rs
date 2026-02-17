//! 错误码定义
//!
//! E5xxx: 模块与导入阶段的错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E5xxx 错误码列表
pub static E5XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E5001",
        category: ErrorCategory::Module,
        message_template: "Module '{module}' not found",
    },
    ErrorCodeDefinition {
        code: "E5002",
        category: ErrorCategory::Module,
        message_template: "Failed to import module '{module}': {reason}",
    },
    ErrorCodeDefinition {
        code: "E5003",
        category: ErrorCategory::Module,
        message_template: "Export '{export}' not found in module '{module}'",
    },
    ErrorCodeDefinition {
        code: "E5004",
        category: ErrorCategory::Module,
        message_template: "Circular dependency detected: {path}",
    },
    ErrorCodeDefinition {
        code: "E5005",
        category: ErrorCategory::Module,
        message_template: "Invalid module path: '{path}'",
    },
    ErrorCodeDefinition {
        code: "E5006",
        category: ErrorCategory::Module,
        message_template: "Duplicate import: '{name}' is already imported",
    },
    ErrorCodeDefinition {
        code: "E5007",
        category: ErrorCategory::Module,
        message_template: "Module '{module}' exports: {available}",
    },
];

// E5xxx 快捷方法
impl ErrorCodeDefinition {
    /// E5001 模块未找到
    pub fn module_not_found(module: &str) -> DiagnosticBuilder {
        let def = Self::find("E5001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("module", module)
    }

    /// E5002 导入错误
    pub fn import_error(
        module: &str,
        reason: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E5002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("module", module)
            .param("reason", reason)
    }

    /// E5003 导出未找到
    pub fn export_not_found(
        export: &str,
        module: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E5003").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("export", export)
            .param("module", module)
    }

    /// E5004 循环依赖
    pub fn circular_dependency(path: &str) -> DiagnosticBuilder {
        let def = Self::find("E5004").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("path", path)
    }

    /// E5005 无效的模块路径
    pub fn invalid_module_path(path: &str) -> DiagnosticBuilder {
        let def = Self::find("E5005").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("path", path)
    }

    /// E5006 重复导入
    pub fn duplicate_import(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E5006").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// E5007 模块导出提示（用于辅助错误消息）
    pub fn module_exports_hint(
        module: &str,
        available: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E5007").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("module", module)
            .param("available", available)
    }
}
