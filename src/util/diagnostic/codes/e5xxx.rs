//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(E5XXX, {
    // E5001 模块未找到
    ("E5001", Module, false, module_not_found(module: &str) => .param("module", module)),
    // E5002 导入错误
    ("E5002", Module, false, import_error(module: &str, reason: &str) => .param("module", module) .param("reason", reason)),
    // E5003 导出未找到
    ("E5003", Module, false, export_not_found(export: &str, module: &str) => .param("export", export) .param("module", module)),
    // E5004 循环依赖
    ("E5004", Module, false, circular_dependency(path: &str) => .param("path", path)),
    // E5005 无效的模块路径
    ("E5005", Module, false, invalid_module_path(path: &str) => .param("path", path)),
    // E5006 重复导入
    ("E5006", Module, false, duplicate_import(name: &str) => .param("name", name)),
    // E5007 模块导出提示（用于辅助错误消息）
    ("E5007", Module, false, module_exports_hint(module: &str, available: &str) => .param("module", module) .param("available", available)),
});
