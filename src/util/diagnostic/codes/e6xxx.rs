//! 错误码定义
//!
//! E6xxx: 运行时错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E6xxx 错误码列表
pub static E6XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E6001",
        category: ErrorCategory::Runtime,
    },
    ErrorCodeDefinition {
        code: "E6003",
        category: ErrorCategory::Runtime,
    },
    ErrorCodeDefinition {
        code: "E6004",
        category: ErrorCategory::Runtime,
    },
    ErrorCodeDefinition {
        code: "E6005",
        category: ErrorCategory::Runtime,
    },
    ErrorCodeDefinition {
        code: "E6006",
        category: ErrorCategory::Runtime,
    },
    ErrorCodeDefinition {
        code: "E6007",
        category: ErrorCategory::Runtime,
    },
    // #299 §4: Dict 键缺失（与索引越界语义不同类，独立码）
    ErrorCodeDefinition {
        code: "E6008",
        category: ErrorCategory::Runtime,
    },
];

// 快捷方法（code_helpers! 生成）
impl ErrorCodeDefinition {
    code_helpers! {
    /// E6001 除零错误
    ("E6001", division_by_zero(expr: &str) => .param("expr", expr)),
    /// E6003 数组索引越界（运行时）
    ("E6003", runtime_index_out_of_bounds(max: usize, index: i64) => .param("max", max.to_string()) .param("index", index.to_string())),
    /// E6004 栈溢出
    ("E6004", stack_overflow(limit: usize) => .param("limit", limit.to_string())),
    /// E6005 断言失败
    ("E6005", assertion_failed(condition: &str) => .param("condition", condition)),
    /// E6006 函数未找到（运行时）
    ("E6006", runtime_function_not_found(func: &str) => .param("func", func)),
    /// E6007 运行时错误（通用）
    ("E6007", runtime_error(message: &str) => .param("message", message)),
    /// E6008 键缺失（#299 §4）
    ("E6008", key_not_found(key: &str) => .param("key", key)),
    }
}
