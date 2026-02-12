//! 错误码定义
//!
//! E4xxx: 泛型与特质阶段的错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E4xxx 错误码列表
pub static E4XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E4001",
        category: ErrorCategory::Generic,
        message_template: "Type '{type}' does not satisfy the trait bound '{trait}'",
    },
    ErrorCodeDefinition {
        code: "E4002",
        category: ErrorCategory::Generic,
        message_template: "Trait '{trait}' not found",
    },
    ErrorCodeDefinition {
        code: "E4003",
        category: ErrorCategory::Generic,
        message_template: "Missing implementation for trait '{trait}' for type '{type}'",
    },
    ErrorCodeDefinition {
        code: "E4004",
        category: ErrorCategory::Generic,
        message_template: "Conflicting trait implementations for '{trait}'",
    },
    ErrorCodeDefinition {
        code: "E4005",
        category: ErrorCategory::Generic,
        message_template: "Associated type '{assoc_type}' not found in '{container}'",
    },
];

// E4xxx 快捷方法
impl ErrorCodeDefinition {
    /// E4001 类型不满足特质约束
    pub fn trait_bound_not_satisfied(
        type_: &str,
        trait_: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E4001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("type", type_)
            .param("trait", trait_)
    }

    /// E4002 特质未找到
    pub fn trait_not_found(trait_: &str) -> DiagnosticBuilder {
        let def = Self::find("E4002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("trait", trait_)
    }

    /// E4003 特质实现缺失
    pub fn missing_trait_impl(
        trait_: &str,
        type_: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E4003").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("trait", trait_)
            .param("type", type_)
    }

    /// E4004 特质实现冲突
    pub fn conflicting_trait_impls(trait_: &str) -> DiagnosticBuilder {
        let def = Self::find("E4004").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template).param("trait", trait_)
    }

    /// E4005 关联类型未找到
    pub fn associated_type_not_found(
        assoc_type: &str,
        container: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E4005").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("assoc_type", assoc_type)
            .param("container", container)
    }
}
