//! 错误码定义
//!
//! E2xxx: 语义分析阶段的错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E2xxx 错误码列表
pub static E2XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E2001",
        category: ErrorCategory::Semantic,
        message_template: "Variable '{name}' is not in scope",
    },
    ErrorCodeDefinition {
        code: "E2002",
        category: ErrorCategory::Semantic,
        message_template: "Duplicate definition: '{name}' is already defined in this scope",
    },
    ErrorCodeDefinition {
        code: "E2003",
        category: ErrorCategory::Semantic,
        message_template: "Ownership constraint violated: {reason}",
    },
    ErrorCodeDefinition {
        code: "E2010",
        category: ErrorCategory::Semantic,
        message_template: "Cannot assign to immutable variable '{name}'",
    },
    ErrorCodeDefinition {
        code: "E2011",
        category: ErrorCategory::Semantic,
        message_template: "Use of uninitialized variable '{name}'",
    },
    ErrorCodeDefinition {
        code: "E2012",
        category: ErrorCategory::Semantic,
        message_template: "Mutability conflict: cannot use mutable reference in immutable context",
    },
    ErrorCodeDefinition {
        code: "E2013",
        category: ErrorCategory::Semantic,
        message_template: "Cannot shadow existing variable '{name}'",
    },
    // E2014: 顶层变量错误
    ErrorCodeDefinition {
        code: "E2014",
        category: ErrorCategory::Semantic,
        message_template: "Function calls are not allowed in top-level variable initializers",
    },
    // E209x: 函数签名解析错误
    ErrorCodeDefinition {
        code: "E2090",
        category: ErrorCategory::Semantic,
        message_template: "Invalid signature: {reason}",
    },
    ErrorCodeDefinition {
        code: "E2091",
        category: ErrorCategory::Semantic,
        message_template: "Invalid signature: unknown type '{type_name}'",
    },
    ErrorCodeDefinition {
        code: "E2092",
        category: ErrorCategory::Semantic,
        message_template: "Invalid signature: missing '->'",
    },
    ErrorCodeDefinition {
        code: "E2093",
        category: ErrorCategory::Semantic,
        message_template: "Invalid signature: duplicate parameter '{name}'",
    },
    ErrorCodeDefinition {
        code: "E2094",
        category: ErrorCategory::Semantic,
        message_template: "Invalid signature: generic '{name}' shadows outer generic",
    },
    ErrorCodeDefinition {
        code: "E2095",
        category: ErrorCategory::Semantic,
        message_template: "Invalid signature: parameter '{name}' shadows generic",
    },
];

// E2xxx 快捷方法
impl ErrorCodeDefinition {
    /// E2001 变量不在作用域中
    pub fn variable_not_in_scope(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// E2002 重复定义
    pub fn duplicate_definition(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// E2003 所有权约束违反
    pub fn ownership_violation(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E2003").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("reason", reason)
    }

    /// E2010 不可变赋值
    pub fn immutable_assignment(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2010").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// E2011 使用未初始化变量
    pub fn uninitialized_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2011").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// E2012 可变性冲突
    pub fn mutability_conflict() -> DiagnosticBuilder {
        let def = Self::find("E2012").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
    }

    /// E2013 变量遮蔽
    pub fn variable_shadowing(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2013").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// E2014 顶层变量不支持函数调用
    pub fn top_level_function_call() -> DiagnosticBuilder {
        let def = Self::find("E2014").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
    }

    /// E2090 签名解析失败（通用）
    pub fn invalid_signature(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E2090").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("reason", reason)
    }

    /// E2091 未知类型
    pub fn invalid_signature_unknown_type(type_name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2091").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("type_name", type_name)
    }

    /// E2092 缺少箭头
    pub fn invalid_signature_missing_arrow() -> DiagnosticBuilder {
        let def = Self::find("E2092").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
    }

    /// E2093 重复参数名
    pub fn invalid_signature_duplicate_param(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2093").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// E2094 泛型参数遮蔽
    pub fn invalid_signature_generic_shadows(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2094").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }

    /// E2095 参数名遮蔽泛型
    pub fn invalid_signature_param_shadows_generic(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E2095").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("name", name)
    }
}
