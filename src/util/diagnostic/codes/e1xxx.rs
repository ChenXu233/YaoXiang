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
    // E1062: const 泛型约束失败（约束求值为 false）
    ErrorCodeDefinition {
        code: "E1062",
        category: ErrorCategory::TypeCheck,
    },
    // W1063: const 泛型约束无法求值（警告）
    ErrorCodeDefinition {
        code: "W1063",
        category: ErrorCategory::TypeCheck,
    },
    // === 控制流 ===
    ErrorCodeDefinition {
        code: "E1070",
        category: ErrorCategory::TypeCheck,
    },
    // E1071: 类型定义只能在模块级（#295：函数体内 TypeDefinition 曾静默跳过）
    ErrorCodeDefinition {
        code: "E1071",
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
    // === RFC-027: 精化类型实参 ===
    // E1092: 谓词/证明函数实参非编译期常量形态（#263：约束不得静默丢弃）
    ErrorCodeDefinition {
        code: "E1092",
        category: ErrorCategory::TypeCheck,
    },
    // E1093: 谓词/证明函数实参个数不匹配（#263）
    ErrorCodeDefinition {
        code: "E1093",
        category: ErrorCategory::TypeCheck,
    },
    ErrorCodeDefinition {
        code: "E1094",
        category: ErrorCategory::TypeCheck,
    },
];

// 快捷方法实现

// 快捷方法（code_helpers! 生成）
impl ErrorCodeDefinition {
    code_helpers! {
    /// E1001 未知变量
    ("E1001", unknown_variable(name: &str) => .param("name", name)),
    /// E1002 类型不匹配
    ("E1002", type_mismatch(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    /// E1003 未知类型
    ("E1003", unknown_type(type_: &str) => .param("type", type_)),
    /// E1010 参数数量不匹配
    ("E1010", argument_count_mismatch(func: &str, expected: usize, found: usize) => .param("func", func) .param("expected", expected.to_string()) .param("found", found.to_string())),
    /// E1011 参数类型不匹配
    ("E1011", parameter_type_mismatch(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    /// E1012 返回类型不匹配
    ("E1012", return_type_mismatch(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    /// E1013 函数未找到
    ("E1013", function_not_found(func: &str) => .param("func", func)),
    /// E1020 无法推断类型
    ("E1020", cannot_infer_type(expr: &str) => .param("expr", expr)),
    /// E1021 类型推断冲突
    ("E1021", type_inference_conflict(reason: &str) => .param("reason", reason)),
    /// E1030 模式穷举不足
    ("E1030", pattern_non_exhaustive(patterns: &str) => .param("patterns", patterns)),
    /// E1031 不可达模式
    ("E1031", unreachable_pattern(pattern: &str) => .param("pattern", pattern)),
    /// E1040 不支持的操作
    ("E1040", unsupported_operation(op: &str, type_: &str) => .param("op", op).param("type", type_)),
    /// E1041 数组越界
    ("E1041", index_out_of_bounds(max: usize, index: i64) => .param("max", max.to_string()) .param("index", index.to_string())),
    /// E1042 字段未找到
    ("E1042", field_not_found(field: &str, struct_: &str) => .param("field", field).param("struct", struct_)),
    /// E1050 逻辑运算需要布尔操作数
    ("E1050", logical_operand_type_mismatch(left: &str, right: &str) => .param("left", left).param("right", right)),
    /// E1051 逻辑 NOT 需要布尔操作数
    ("E1051", logical_not_type_mismatch(type_: &str) => .param("type", type_)),
    /// E1052 不能解引用非指针类型
    ("E1052", invalid_deref(type_: &str) => .param("type", type_)),
    /// E1053 不能在非结构体类型上访问字段
    ("E1053", field_access_on_non_struct(type_: &str) => .param("type", type_)),
    /// E1054 条件必须是布尔类型
    ("E1054", condition_type_mismatch(type_: &str) => .param("type", type_)),
    /// E1055 约束类型只能在泛型上下文中使用
    ("E1055", constraint_not_in_generic(type_: &str) => .param("type", type_)),
    /// E1060 类型参数数量不匹配
    ("E1060", type_argument_count_mismatch(expected: usize, found: usize) => .param("expected", expected.to_string()) .param("found", found.to_string())),
    /// E1061 无法实例化泛型类型
    ("E1061", cannot_instantiate_generic() => ),
    /// E1070 未知标签
    ("E1070", unknown_label(label: &str) => .param("label", label)),
    /// E1071 类型定义只能在模块级
    ("E1071", type_def_only_at_module_level(name: &str) => .param("name", name)),
    /// E1081 `?` 仅允许在返回 Result 的函数内使用
    ("E1081", try_only_allowed_in_result() => ),
    /// E1082 `?` 只能用于 Result 表达式
    ("E1082", try_requires_result(type_: &str) => .param("type", type_)),
    /// E1083 `?` 的错误类型不匹配
    ("E1083", try_error_type_mismatch(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    /// E1091 泛型元类型自指错误
    ("E1091", invalid_generic_self_reference(decl: &str) => .param("decl", decl)),
    /// E1092 精化类型实参形态非法（RFC-027，#263）
    ("E1092", refined_arg_not_const(name: &str) => .param("name", name)),
    /// E1093 精化类型实参个数不匹配（RFC-027，#263）
    /// E1093 精化类型参数数量不匹配
    ("E1093", refined_arity_mismatch(name: &str, expected: usize, found: usize) => .param("name", name) .param("expected", expected.to_string()) .param("found", found.to_string())),
    /// E1094 编译期值参数未在类型体引用（#297/F）
    ("E1094", unused_const_param(param: &str, type_: &str) => .param("param", param) .param("type", type_)),
    /// E1090 彩蛋（返回占位符，由 i18n 的 zen_message 提供实际消息）
    ("E1090", type_self_reference_easter_egg() => ),
    }
}
