//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(W1XXX, {
    // W1001 未使用的私有函数（pub = 对外接口永不报，#321 定案 B）
    ("W1001", Warning, false, unused_function(name: &str) => .param("name", name)),
    // W1002 未使用的私有类型
    ("W1002", Warning, false, unused_type(name: &str) => .param("name", name)),
    // W1003 未使用的导入（typecheck use elaboration 检出）
    ("W1003", Warning, false, unused_import(name: &str) => .param("name", name)),
    // W1004 未使用的私有变量
    ("W1004", Warning, false, unused_variable(name: &str) => .param("name", name)),
    // W1005 未使用的私有方法
    ("W1005", Warning, false, unused_method(name: &str) => .param("name", name)),
    // W1063
    ("W1063", Warning, false, const_generic_unevaluable(constraint: &str) => .param("constraint", constraint)),
    // W1080 编译期无法证明约束，已降级为运行时检查
    ("W1080", Warning, false, constraint_demoted() => ),
});
