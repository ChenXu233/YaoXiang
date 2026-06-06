//! 错误码定义
//!
//! E3xxx: 代码生成阶段的错误码
//!
//! E3001-E3009: IR 生成（ir_gen）
//! E3010-E3019: 字节码生成（codegen）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E3xxx 错误码列表
pub static E3XXX: &[ErrorCodeDefinition] = &[
    // === E3001-E3009: IR 生成 ===
    ErrorCodeDefinition {
        code: "E3001",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3002",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3003",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3004",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3005",
        category: ErrorCategory::Codegen,
    },
    // === E3010-E3019: 字节码生成 ===
    ErrorCodeDefinition {
        code: "E3010",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3011",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3012",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3013",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3014",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3015",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3016",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3017",
        category: ErrorCategory::Codegen,
    },
    ErrorCodeDefinition {
        code: "E3018",
        category: ErrorCategory::Codegen,
    },
];

// E3xxx 快捷方法
impl ErrorCodeDefinition {
    // === IR 生成 ===

    /// E3001 未实现的表达式类型（IR 生成）
    pub fn ir_unimplemented_expr(expr_type: &str) -> DiagnosticBuilder {
        let def = Self::find("E3001").unwrap();
        def.builder().param("expr_type", expr_type)
    }

    /// E3002 未实现的语句类型（IR 生成）
    pub fn ir_unimplemented_stmt(stmt_type: &str) -> DiagnosticBuilder {
        let def = Self::find("E3002").unwrap();
        def.builder().param("stmt_type", stmt_type)
    }

    /// E3003 无效操作数（IR 生成）
    pub fn ir_invalid_operand(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E3003").unwrap();
        def.builder().param("reason", reason)
    }

    /// E3004 不支持的迭代器类型
    pub fn ir_unsupported_iterator(iter_type: &str) -> DiagnosticBuilder {
        let def = Self::find("E3004").unwrap();
        def.builder().param("iter_type", iter_type)
    }

    /// E3005 IR 内部错误
    pub fn ir_internal_error(message: &str) -> DiagnosticBuilder {
        let def = Self::find("E3005").unwrap();
        def.builder().param("message", message)
    }

    // === 字节码生成 ===

    /// E3010 未实现的表达式类型（代码生成）
    pub fn codegen_unimplemented_expr(expr_type: &str) -> DiagnosticBuilder {
        let def = Self::find("E3010").unwrap();
        def.builder().param("expr_type", expr_type)
    }

    /// E3011 未实现的语句类型（代码生成）
    pub fn codegen_unimplemented_stmt(stmt_type: &str) -> DiagnosticBuilder {
        let def = Self::find("E3011").unwrap();
        def.builder().param("stmt_type", stmt_type)
    }

    /// E3012 未实现的调用类型（代码生成）
    pub fn codegen_unimplemented_call(call_type: &str) -> DiagnosticBuilder {
        let def = Self::find("E3012").unwrap();
        def.builder().param("call_type", call_type)
    }

    /// E3013 符号未找到（代码生成）
    pub fn codegen_symbol_not_found(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E3013").unwrap();
        def.builder().param("name", name)
    }

    /// E3014 寄存器溢出
    pub fn register_overflow(
        id: &str,
        limit: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E3014").unwrap();
        def.builder().param("id", id).param("limit", limit)
    }

    /// E3015 无效赋值目标
    pub fn codegen_invalid_assignment_target(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E3015").unwrap();
        def.builder().param("reason", reason)
    }

    /// E3016 类型不匹配（代码生成）
    pub fn codegen_type_mismatch(
        expected: &str,
        found: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E3016").unwrap();
        def.builder()
            .param("expected", expected)
            .param("found", found)
    }

    /// E3017 无效操作数（代码生成）
    pub fn codegen_invalid_operand(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E3017").unwrap();
        def.builder().param("reason", reason)
    }

    /// E3018 翻译错误
    pub fn translation_error(message: &str) -> DiagnosticBuilder {
        let def = Self::find("E3018").unwrap();
        def.builder().param("message", message)
    }
}
