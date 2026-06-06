//! 错误码定义
//!
//! E1xxx: 类型检查阶段的错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E1xxx 错误码列表
pub static E1XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E1001",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1002",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1003",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1010",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1011",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1012",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1013",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1020",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1021",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1030",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1031",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1040",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1041",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1042",
        category: ErrorCategory::TypeCheck,
    },
    // === 表达式类型检查 ===
    ErrorCodeDefinition {
        code: "E1050",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1051",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1052",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1053",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1054",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1055",
        category: ErrorCategory::TypeCheck,
    },
    // === 泛型实例化 ===
    ErrorCodeDefinition {
        code: "E1060",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1061",
        category: ErrorCategory::TypeCheck,
    },
    // === 控制流 ===
    ErrorCodeDefinition {
        code: "E1070",
        category: ErrorCategory::TypeCheck,
    },
    // === RFC-001/008: 并发语义约束 ===
    ErrorCodeDefinition {
        code: "E1080",
        category: ErrorCategory::TypeCheck,
    },
    // === RFC-001: Result/? 错误传播 ===
    ErrorCodeDefinition {
        code: "E1081",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1082",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1083",
        category: ErrorCategory::TypeCheck,
    },
    // === RFC-010: Type 元类型 ===
    // E1090: Type: Type = Type 彩蛋 (Note 级别)
    ErrorCodeDefinition {
        code: "E1090",
        category: ErrorCategory::TypeCheck,
    },
    // E1091: Type: Type[T] = ... 泛型元类型自指错误
    ErrorCodeDefinition {
        code: "E1091",
        category: ErrorCategory::TypeCheck,
    },
];

// 快捷方法实现
impl ErrorCodeDefinition {
    /// E1001 未知变量
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        def.builder().param("name", name)
    }

    /// E1002 类型不匹配
    pub fn type_mismatch(
        expected: &str,
        found: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1002").unwrap();
        def.builder()
            .param("expected", expected)
            .param("found", found)
    }

    /// E1003 未知类型
    pub fn unknown_type(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1003").unwrap();
        def.builder().param("type", type_)
    }

    /// E1010 参数数量不匹配
    pub fn argument_count_mismatch(
        func: &str,
        expected: usize,
        found: usize,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1010").unwrap();
        def.builder()
            .param("func", func)
            .param("expected", expected.to_string())
            .param("found", found.to_string())
    }

    /// E1011 参数类型不匹配
    pub fn parameter_type_mismatch(
        expected: &str,
        found: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1011").unwrap();
        def.builder()
            .param("expected", expected)
            .param("found", found)
    }

    /// E1012 返回类型不匹配
    pub fn return_type_mismatch(
        expected: &str,
        found: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1012").unwrap();
        def.builder()
            .param("expected", expected)
            .param("found", found)
    }

    /// E1013 函数未找到
    pub fn function_not_found(func: &str) -> DiagnosticBuilder {
        let def = Self::find("E1013").unwrap();
        def.builder().param("func", func)
    }

    /// E1020 无法推断类型
    pub fn cannot_infer_type(expr: &str) -> DiagnosticBuilder {
        let def = Self::find("E1020").unwrap();
        def.builder().param("expr", expr)
    }

    /// E1021 类型推断冲突
    pub fn type_inference_conflict(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E1021").unwrap();
        def.builder().param("reason", reason)
    }

    /// E1030 模式穷举不足
    pub fn pattern_non_exhaustive(patterns: &str) -> DiagnosticBuilder {
        let def = Self::find("E1030").unwrap();
        def.builder().param("patterns", patterns)
    }

    /// E1031 不可达模式
    pub fn unreachable_pattern(pattern: &str) -> DiagnosticBuilder {
        let def = Self::find("E1031").unwrap();
        def.builder().param("pattern", pattern)
    }

    /// E1040 不支持的操作
    pub fn unsupported_operation(
        op: &str,
        type_: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1040").unwrap();
        def.builder()
            .param("op", op)
            .param("type", type_)
    }

    /// E1041 数组越界
    pub fn index_out_of_bounds(
        max: usize,
        index: usize,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1041").unwrap();
        def.builder()
            .param("max", max.to_string())
            .param("index", index.to_string())
    }

    /// E1042 字段未找到
    pub fn field_not_found(
        field: &str,
        struct_: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1042").unwrap();
        def.builder()
            .param("field", field)
            .param("struct", struct_)
    }

    /// E1050 逻辑运算需要布尔操作数
    pub fn logical_operand_type_mismatch(
        left: &str,
        right: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1050").unwrap();
        def.builder()
            .param("left", left)
            .param("right", right)
    }

    /// E1051 逻辑 NOT 需要布尔操作数
    pub fn logical_not_type_mismatch(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1051").unwrap();
        def.builder().param("type", type_)
    }

    /// E1052 不能解引用非指针类型
    pub fn invalid_deref(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1052").unwrap();
        def.builder().param("type", type_)
    }

    /// E1053 不能在非结构体类型上访问字段
    pub fn field_access_on_non_struct(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1053").unwrap();
        def.builder().param("type", type_)
    }

    /// E1054 条件必须是布尔类型
    pub fn condition_type_mismatch(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1054").unwrap();
        def.builder().param("type", type_)
    }

    /// E1055 约束类型只能在泛型上下文中使用
    pub fn constraint_not_in_generic(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1055").unwrap();
        def.builder().param("type", type_)
    }

    /// E1060 类型参数数量不匹配
    pub fn type_argument_count_mismatch(
        expected: usize,
        found: usize,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1060").unwrap();
        def.builder()
            .param("expected", expected.to_string())
            .param("found", found.to_string())
    }

    /// E1061 无法实例化泛型类型
    pub fn cannot_instantiate_generic() -> DiagnosticBuilder {
        let def = Self::find("E1061").unwrap();
        def.builder()
    }

    /// E1070 未知标签
    pub fn unknown_label(label: &str) -> DiagnosticBuilder {
        let def = Self::find("E1070").unwrap();
        def.builder().param("label", label)
    }

    /// E1080 spawn 仅允许在 @block 作用域内使用
    pub fn spawn_only_allowed_in_block(mode: &str) -> DiagnosticBuilder {
        let def = Self::find("E1080").unwrap();
        def.builder().param("mode", mode)
    }

    /// E1081 `?` 仅允许在返回 Result 的函数内使用
    pub fn try_only_allowed_in_result() -> DiagnosticBuilder {
        let def = Self::find("E1081").unwrap();
        def.builder()
    }

    /// E1082 `?` 只能用于 Result 表达式
    pub fn try_requires_result(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1082").unwrap();
        def.builder().param("type", type_)
    }

    /// E1083 `?` 的错误类型不匹配
    pub fn try_error_type_mismatch(
        expected: &str,
        found: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1083").unwrap();
        def.builder()
            .param("expected", expected)
            .param("found", found)
    }

    /// E1091 泛型元类型自指错误
    pub fn invalid_generic_self_reference(decl: &str) -> DiagnosticBuilder {
        let def = Self::find("E1091").unwrap();
        def.builder().param("decl", decl)
    }

    /// E1090 彩蛋（返回占位符，由 i18n 的 zen_message 提供实际消息）
    pub fn type_self_reference_easter_egg() -> DiagnosticBuilder {
        let def = Self::find("E1090").unwrap();
        def.builder()
    }
}
