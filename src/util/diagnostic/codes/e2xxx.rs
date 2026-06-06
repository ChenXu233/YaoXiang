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
    // E2015: 移动后借用
    ErrorCodeDefinition {
        code: "E2015",
        category: ErrorCategory::Semantic,
    },
    // E2016: 不可变赋值
    ErrorCodeDefinition {
        code: "E2016",
        category: ErrorCategory::Semantic,
    },
    // E2017: 多重可变借用
    ErrorCodeDefinition {
        code: "E2017",
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
    // E2021: 释放已移动的值
    ErrorCodeDefinition {
        code: "E2021",
        category: ErrorCategory::Semantic,
    },
    // E2022: 不可变变异
    ErrorCodeDefinition {
        code: "E2022",
        category: ErrorCategory::Semantic,
    },
    // E2023: 不可变字段赋值
    ErrorCodeDefinition {
        code: "E2023",
        category: ErrorCategory::Semantic,
    },
    // E2024: 引用非所有者
    ErrorCodeDefinition {
        code: "E2024",
        category: ErrorCategory::Semantic,
    },
    // E2025: 重赋值非空变量
    ErrorCodeDefinition {
        code: "E2025",
        category: ErrorCategory::Semantic,
    },
    // E2026: 消费未返回
    ErrorCodeDefinition {
        code: "E2026",
        category: ErrorCategory::Semantic,
    },
    // E2027: unsafe 解引用
    ErrorCodeDefinition {
        code: "E2027",
        category: ErrorCategory::Semantic,
    },
    // E2028: 克隆已移动的值
    ErrorCodeDefinition {
        code: "E2028",
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

// E2xxx 快捷方法
impl ErrorCodeDefinition {
    /// E2001 变量不在作用域中
    pub fn variable_not_in_scope(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2001").unwrap();
        def.builder().param("name", name)
    }

    /// E2002 重复定义
    pub fn duplicate_definition(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2002").unwrap();
        def.builder().param("name", name)
    }

    /// E2003 所有权约束违反
    pub fn ownership_violation(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E2003").unwrap();
        def.builder().param("reason", reason)
    }

    /// E2010 不可变赋值
    pub fn immutable_assignment(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2010").unwrap();
        def.builder().param("name", name)
    }

    /// E2011 使用未初始化变量
    pub fn uninitialized_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2011").unwrap();
        def.builder().param("name", name)
    }

    /// E2012 可变性冲突
    pub fn mutability_conflict() -> DiagnosticBuilder {
        let def = Self::find("E2012").unwrap();
        def.builder()
    }

    /// E2013 变量遮蔽
    pub fn variable_shadowing(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2013").unwrap();
        def.builder().param("name", name)
    }

    /// E2014 使用已移动的变量
    pub fn use_after_move(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2014").unwrap();
        def.builder().param("name", name)
    }

    /// E2015 移动后借用
    pub fn borrow_after_move(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2015").unwrap();
        def.builder().param("name", name)
    }

    /// E2016 不可变赋值（所有权检查器用）
    pub fn immutable_assign(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2016").unwrap();
        def.builder().param("name", name)
    }

    /// E2017 多重可变借用
    pub fn mutable_borrow_conflict(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2017").unwrap();
        def.builder().param("name", name)
    }

    /// E2018 可变/不可变借用冲突
    pub fn mutable_immutable_borrow_conflict(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2018").unwrap();
        def.builder().param("name", name)
    }

    /// E2019 双重释放
    pub fn double_drop(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2019").unwrap();
        def.builder().param("name", name)
    }

    /// E2020 释放后使用
    pub fn use_after_drop(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2020").unwrap();
        def.builder().param("name", name)
    }

    /// E2021 释放已移动的值
    pub fn drop_moved_value(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2021").unwrap();
        def.builder().param("name", name)
    }

    /// E2022 不可变变异
    pub fn immutable_mutation(name: &str, method: &str) -> DiagnosticBuilder {
        let def = Self::find("E2022").unwrap();
        def.builder().param("name", name).param("method", method)
    }

    /// E2023 不可变字段赋值
    pub fn immutable_field_assign(struct_name: &str, field: &str) -> DiagnosticBuilder {
        let def = Self::find("E2023").unwrap();
        def.builder().param("struct_name", struct_name).param("field", field)
    }

    /// E2024 引用非所有者
    pub fn ref_non_owner(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2024").unwrap();
        def.builder().param("name", name)
    }

    /// E2025 重赋值非空变量
    pub fn reassign_non_empty(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2025").unwrap();
        def.builder().param("name", name)
    }

    /// E2026 消费未返回
    pub fn consumed_not_returned(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2026").unwrap();
        def.builder().param("name", name)
    }

    /// E2027 unsafe 解引用
    pub fn unsafe_deref() -> DiagnosticBuilder {
        let def = Self::find("E2027").unwrap();
        def.builder()
    }

    /// E2028 克隆已移动的值
    pub fn clone_moved_value(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2028").unwrap();
        def.builder().param("name", name)
    }

    /// E2090 签名解析失败（通用）
    pub fn invalid_signature(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E2090").unwrap();
        def.builder().param("reason", reason)
    }

    /// E2091 未知类型
    pub fn invalid_signature_unknown_type(type_name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2091").unwrap();
        def.builder().param("type_name", type_name)
    }

    /// E2092 缺少箭头
    pub fn invalid_signature_missing_arrow() -> DiagnosticBuilder {
        let def = Self::find("E2092").unwrap();
        def.builder()
    }

    /// E2093 重复参数名
    pub fn invalid_signature_duplicate_param(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2093").unwrap();
        def.builder().param("name", name)
    }

    /// E2094 泛型参数遮蔽
    pub fn invalid_signature_generic_shadows(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2094").unwrap();
        def.builder().param("name", name)
    }

    /// E2095 参数名遮蔽泛型
    pub fn invalid_signature_param_shadows_generic(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2095").unwrap();
        def.builder().param("name", name)
    }
}
