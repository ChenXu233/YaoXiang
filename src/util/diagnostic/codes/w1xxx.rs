//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(W1XXX, {
    // W1001 未使用的导出函数
    ("W1001", Warning, false, unused_function(name: &str) => .param("name", name)),
    // W1002 未使用的导出类型
    ("W1002", Warning, false, unused_type(name: &str) => .param("name", name)),
    // W1003 未使用的导入
    ("W1003", Warning, false, unused_import(name: &str) => .param("name", name)),
    // W1004 未使用的导出变量
    ("W1004", Warning, false, unused_variable(name: &str) => .param("name", name)),
    // W1005 未使用的导出方法
    ("W1005", Warning, false, unused_method(name: &str) => .param("name", name)),
    // W1063
    ("W1063", Warning, false, const_generic_unevaluable(constraint: &str) => .param("constraint", constraint)),
    // W1080 编译期无法证明约束，已降级为运行时检查
    ("W1080", Warning, false, constraint_demoted() => ),
});
