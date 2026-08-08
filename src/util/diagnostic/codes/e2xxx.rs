//! 错误码定义
//!
//! E2xxx: 语义分析阶段的错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E2xxx 错误码列表
pub static E2XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E2001",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2002",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2003",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2010",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2011",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2012",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2013",
        category: ErrorCategory::Semantic,
    },
    // E2014: 使用已移动的变量
    ErrorCodeDefinition {
        code: "E2014",
        category: ErrorCategory::Semantic,
    },
    // E2016: 不可变赋值
    ErrorCodeDefinition {
        code: "E2016",
        category: ErrorCategory::Semantic,
    },
    // E2018: 可变/不可变借用冲突
    ErrorCodeDefinition {
        code: "E2018",
        category: ErrorCategory::Semantic,
    },
    // E2019: 双重释放
    ErrorCodeDefinition {
        code: "E2019",
        category: ErrorCategory::Semantic,
    },
    // E2020: 释放后使用
    ErrorCodeDefinition {
        code: "E2020",
        category: ErrorCategory::Semantic,
    },
    // E2027: unsafe 解引用
    ErrorCodeDefinition {
        code: "E2027",
        category: ErrorCategory::Semantic,
    },
    // E209x: 函数签名解析错误
    ErrorCodeDefinition {
        code: "E2090",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2091",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2092",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2093",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2094",
        category: ErrorCategory::Semantic,
    },
    ErrorCodeDefinition {
        code: "E2095",
        category: ErrorCategory::Semantic,
    },
];

// 快捷方法（code_helpers! 生成）
impl ErrorCodeDefinition {
    code_helpers! {
    /// E2001 变量不在作用域中
    ("E2001", variable_not_in_scope(name: &str) => .param("name", name)),
    /// E2002 重复定义
    ("E2002", duplicate_definition(name: &str) => .param("name", name)),
    /// E2003 所有权约束违反
    ("E2003", ownership_violation(reason: &str) => .param("reason", reason)),
    /// E2010 不可变赋值
    ("E2010", immutable_assignment(name: &str) => .param("name", name)),
    /// E2011 使用未初始化变量
    ("E2011", uninitialized_variable(name: &str) => .param("name", name)),
    /// E2012 可变性冲突
    ("E2012", mutability_conflict() => ),
    /// E2013 变量遮蔽
    ("E2013", variable_shadowing(name: &str) => .param("name", name)),
    /// E2014 使用已移动的变量
    ("E2014", use_after_move(name: &str) => .param("name", name)),
    /// E2016 不可变赋值（所有权检查器用）
    ("E2016", immutable_assign(name: &str) => .param("name", name)),
    /// E2018 可变/不可变借用冲突
    ("E2018", mutable_immutable_borrow_conflict(name: &str) => .param("name", name)),
    /// E2019 双重释放
    ("E2019", double_drop(name: &str) => .param("name", name)),
    /// E2020 释放后使用
    ("E2020", use_after_drop(name: &str) => .param("name", name)),
    /// E2027 unsafe 解引用
    ("E2027", unsafe_deref() => ),
    /// E2029 spawn 内 ref 循环
    ("E2029", spawn_ref_cycle(cycle: &str) => .param("cycle", cycle)),
    /// E2090 签名解析失败（通用）
    ("E2090", invalid_signature(reason: &str) => .param("reason", reason)),
    /// E2091 未知类型
    ("E2091", invalid_signature_unknown_type(type_name: &str) => .param("type_name", type_name)),
    /// E2092 缺少箭头
    ("E2092", invalid_signature_missing_arrow() => ),
    /// E2093 重复参数名
    ("E2093", invalid_signature_duplicate_param(name: &str) => .param("name", name)),
    /// E2094 泛型参数遮蔽
    ("E2094", invalid_signature_generic_shadows(name: &str) => .param("name", name)),
    /// E2095 参数名遮蔽泛型
    ("E2095", invalid_signature_param_shadows_generic(name: &str) => .param("name", name)),
    }
}
