//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(E3XXX, {
    // E3004 不支持的迭代器类型
    ("E3004", Codegen, false, ir_unsupported_iterator(iter_type: &str) => .param("iter_type", iter_type)),
    // E3005 IR 内部错误
    ("E3005", Codegen, false, ir_internal_error(message: &str) => .param("message", message)),
    // E3006 未解析变量（#271 静默归零清单 #3：typecheck 漏网变量不再静默 Load 0）
    ("E3006", Codegen, false, unresolved_variable(name: &str) => .param("name", name)),
    // E3007 顶层绑定初始化非编译期常量（#271 清单 #2：折叠不到不再静默填 0）
    ("E3007", Codegen, false, top_level_init_not_const(name: &str) => .param("name", name)),
    // E3014 寄存器溢出
    ("E3014", Codegen, false, register_overflow(id: &str, limit: &str) => .param("id", id).param("limit", limit)),
    // E3017 无效操作数（代码生成）
    ("E3017", Codegen, false, codegen_invalid_operand(reason: &str) => .param("reason", reason)),
});
