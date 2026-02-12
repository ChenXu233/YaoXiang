//! 错误码定义
//!
//! E6xxx: 运行时错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E6xxx 错误码列表
pub static E6XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E6001",
        category: ErrorCategory::Runtime,
        message_template: "Division by zero in expression: {expr}",
    },
    ErrorCodeDefinition {
        code: "E6002",
        category: ErrorCategory::Runtime,
        message_template: "Null pointer dereference at {location}",
    },
    ErrorCodeDefinition {
        code: "E6003",
        category: ErrorCategory::Runtime,
        message_template: "Array index out of bounds: valid range is 0..{max}, found {index}",
    },
    ErrorCodeDefinition {
        code: "E6004",
        category: ErrorCategory::Runtime,
        message_template: "Stack overflow: recursion depth exceeded limit {limit}",
    },
    ErrorCodeDefinition {
        code: "E6005",
        category: ErrorCategory::Runtime,
        message_template: "Assertion failed: {condition}",
    },
];

// E6xxx 快捷方法
impl ErrorCodeDefinition {
    /// E6001 除零错误
    pub fn division_by_zero(expr: &str) -> DiagnosticBuilder {
        let def = Self::find("E6001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("expr", expr)
    }

    /// E6002 空指针解引用
    pub fn null_pointer_deref(location: &str) -> DiagnosticBuilder {
        let def = Self::find("E6002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("location", location)
    }

    /// E6003 数组索引越界（运行时）
    pub fn runtime_index_out_of_bounds(
        max: usize,
        index: usize,
    ) -> DiagnosticBuilder {
        let def = Self::find("E6003").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("max", max.to_string())
            .param("index", index.to_string())
    }

    /// E6004 栈溢出
    pub fn stack_overflow(limit: usize) -> DiagnosticBuilder {
        let def = Self::find("E6004").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("limit", limit.to_string())
    }

    /// E6005 断言失败
    pub fn assertion_failed(condition: &str) -> DiagnosticBuilder {
        let def = Self::find("E6005").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("condition", condition)
    }
}
