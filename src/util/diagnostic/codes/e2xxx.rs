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
}
