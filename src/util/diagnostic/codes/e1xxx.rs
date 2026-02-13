//! 错误码定义
//!
//! E1xxx: 类型检查阶段的错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E1xxx 错误码列表
pub static E1XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E1001",
        category: ErrorCategory::TypeCheck,
        message_template: "Unknown variable: '{name}'",
    },
    ErrorCodeDefinition {
        code: "E1002",
        category: ErrorCategory::TypeCheck,
        message_template: "Expected type '{expected}', found type '{found}'",
    },
    ErrorCodeDefinition {
        code: "E1003",
        category: ErrorCategory::TypeCheck,
        message_template: "Unknown type: '{type}'",
    },
    ErrorCodeDefinition {
        code: "E1010",
        category: ErrorCategory::TypeCheck,
        message_template: "Function '{func}' expects {expected} arguments, found {found}",
    },
    ErrorCodeDefinition {
        code: "E1011",
        category: ErrorCategory::TypeCheck,
        message_template: "Parameter type mismatch: expected '{expected}', found '{found}'",
    },
    ErrorCodeDefinition {
        code: "E1012",
        category: ErrorCategory::TypeCheck,
        message_template: "Return type mismatch: expected '{expected}', found '{found}'",
    },
    ErrorCodeDefinition {
        code: "E1013",
        category: ErrorCategory::TypeCheck,
        message_template: "Function not found: '{func}'",
    },
    ErrorCodeDefinition {
        code: "E1020",
        category: ErrorCategory::TypeCheck,
        message_template: "Cannot infer type for '{expr}'",
    },
    ErrorCodeDefinition {
        code: "E1021",
        category: ErrorCategory::TypeCheck,
        message_template: "Type inference conflict: {reason}",
    },
    ErrorCodeDefinition {
        code: "E1030",
        category: ErrorCategory::TypeCheck,
        message_template: "Pattern non-exhaustive: missing patterns {patterns}",
    },
    ErrorCodeDefinition {
        code: "E1031",
        category: ErrorCategory::TypeCheck,
        message_template: "Unreachable pattern: '{pattern}'",
    },
    ErrorCodeDefinition {
        code: "E1040",
        category: ErrorCategory::TypeCheck,
        message_template: "Operation '{op}' is not supported for type '{type}'",
    },
    ErrorCodeDefinition {
        code: "E1041",
        category: ErrorCategory::TypeCheck,
        message_template: "Index out of bounds: valid range is 0..{max}, found {index}",
    },
    ErrorCodeDefinition {
        code: "E1042",
        category: ErrorCategory::TypeCheck,
        message_template: "Field '{field}' not found in struct '{struct}'",
    },
    // === 表达式类型检查 ===
    ErrorCodeDefinition {
        code: "E1050",
        category: ErrorCategory::TypeCheck,
        message_template:
            "Logical operation requires boolean operands, found '{left}' and '{right}'",
    },
    ErrorCodeDefinition {
        code: "E1051",
        category: ErrorCategory::TypeCheck,
        message_template: "Logical NOT requires boolean operand, found '{type}'",
    },
    ErrorCodeDefinition {
        code: "E1052",
        category: ErrorCategory::TypeCheck,
        message_template: "Cannot dereference type '{type}', expected pointer type",
    },
    ErrorCodeDefinition {
        code: "E1053",
        category: ErrorCategory::TypeCheck,
        message_template: "Cannot access field on non-struct type '{type}'",
    },
    ErrorCodeDefinition {
        code: "E1054",
        category: ErrorCategory::TypeCheck,
        message_template: "Condition must be boolean, found '{type}'",
    },
    ErrorCodeDefinition {
        code: "E1055",
        category: ErrorCategory::TypeCheck,
        message_template: "Constraint type '{type}' can only be used in generic context",
    },
    // === 泛型实例化 ===
    ErrorCodeDefinition {
        code: "E1060",
        category: ErrorCategory::TypeCheck,
        message_template: "Expected {expected} type argument(s), found {found}",
    },
    ErrorCodeDefinition {
        code: "E1061",
        category: ErrorCategory::TypeCheck,
        message_template: "Cannot instantiate generic type with given arguments",
    },
    // === 控制流 ===
    ErrorCodeDefinition {
        code: "E1070",
        category: ErrorCategory::TypeCheck,
        message_template: "Unknown label: '{label}'",
    },
    // === RFC-010: Type 元类型 ===
    // E1090: Type: Type = Type 彩蛋 (Note 级别)
    ErrorCodeDefinition {
        code: "E1090",
        category: ErrorCategory::TypeCheck,
        message_template: "", // 消息在 i18n 的 zen_message 中
    },
    // E1091: Type: Type[T] = ... 泛型元类型自指错误
    ErrorCodeDefinition {
        code: "E1091",
        category: ErrorCategory::TypeCheck,
        message_template: "Generic meta-type self-reference is not allowed: '{decl}'",
    },
];

// 快捷方法实现
impl ErrorCodeDefinition {
    /// E1001 未知变量
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// E1002 类型不匹配
    pub fn type_mismatch(
        expected: &str,
        found: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("expected", expected)
            .param("found", found)
    }

    /// E1003 未知类型
    pub fn unknown_type(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1003").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("type", type_)
    }

    /// E1010 参数数量不匹配
    pub fn argument_count_mismatch(
        func: &str,
        expected: usize,
        found: usize,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1010").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
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
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("expected", expected)
            .param("found", found)
    }

    /// E1012 返回类型不匹配
    pub fn return_type_mismatch(
        expected: &str,
        found: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1012").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("expected", expected)
            .param("found", found)
    }

    /// E1013 函数未找到
    pub fn function_not_found(func: &str) -> DiagnosticBuilder {
        let def = Self::find("E1013").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("func", func)
    }

    /// E1020 无法推断类型
    pub fn cannot_infer_type(expr: &str) -> DiagnosticBuilder {
        let def = Self::find("E1020").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("expr", expr)
    }

    /// E1021 类型推断冲突
    pub fn type_inference_conflict(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E1021").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("reason", reason)
    }

    /// E1030 模式穷举不足
    pub fn pattern_non_exhaustive(patterns: &str) -> DiagnosticBuilder {
        let def = Self::find("E1030").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("patterns", patterns)
    }

    /// E1031 不可达模式
    pub fn unreachable_pattern(pattern: &str) -> DiagnosticBuilder {
        let def = Self::find("E1031").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("pattern", pattern)
    }

    /// E1040 不支持的操作
    pub fn unsupported_operation(
        op: &str,
        type_: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1040").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("op", op)
            .param("type", type_)
    }

    /// E1041 数组越界
    pub fn index_out_of_bounds(
        max: usize,
        index: usize,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1041").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("max", max.to_string())
            .param("index", index.to_string())
    }

    /// E1042 字段未找到
    pub fn field_not_found(
        field: &str,
        struct_: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1042").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("field", field)
            .param("struct", struct_)
    }

    /// E1050 逻辑运算需要布尔操作数
    pub fn logical_operand_type_mismatch(
        left: &str,
        right: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1050").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("left", left)
            .param("right", right)
    }

    /// E1051 逻辑 NOT 需要布尔操作数
    pub fn logical_not_type_mismatch(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1051").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("type", type_)
    }

    /// E1052 不能解引用非指针类型
    pub fn invalid_deref(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1052").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("type", type_)
    }

    /// E1053 不能在非结构体类型上访问字段
    pub fn field_access_on_non_struct(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1053").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("type", type_)
    }

    /// E1054 条件必须是布尔类型
    pub fn condition_type_mismatch(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1054").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("type", type_)
    }

    /// E1055 约束类型只能在泛型上下文中使用
    pub fn constraint_not_in_generic(type_: &str) -> DiagnosticBuilder {
        let def = Self::find("E1055").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("type", type_)
    }

    /// E1060 类型参数数量不匹配
    pub fn type_argument_count_mismatch(
        expected: usize,
        found: usize,
    ) -> DiagnosticBuilder {
        let def = Self::find("E1060").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("expected", expected.to_string())
            .param("found", found.to_string())
    }

    /// E1061 无法实例化泛型类型
    pub fn cannot_instantiate_generic() -> DiagnosticBuilder {
        let def = Self::find("E1061").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
    }

    /// E1070 未知标签
    pub fn unknown_label(label: &str) -> DiagnosticBuilder {
        let def = Self::find("E1070").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("label", label)
    }

    /// E1091 泛型元类型自指错误
    pub fn invalid_generic_self_reference(decl: &str) -> DiagnosticBuilder {
        let def = Self::find("E1091").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("decl", decl)
    }

    /// E1090 彩蛋（返回占位符，由 i18n 的 zen_message 提供实际消息）
    pub fn type_self_reference_easter_egg() -> DiagnosticBuilder {
        let def = Self::find("E1090").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
    }
}
