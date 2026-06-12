//! 错误码定义
//!
//! E4xxx: 泛型与特质阶段的错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E4xxx 错误码列表
pub static E4XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E4001",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4002",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4003",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4004",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4005",
        category: ErrorCategory::Generic,
    },
    // === E401x: 常量求值 ===
    ErrorCodeDefinition {
        code: "E4010",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4011",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4012",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4013",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4014",
        category: ErrorCategory::Generic,
    },
    // === E401x: 终止检查 ===
    ErrorCodeDefinition {
        code: "E4015",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4016",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4017",
        category: ErrorCategory::Generic,
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
        def.builder().param("type", type_).param("trait", trait_)
    }

    /// E4002 特质未找到
    pub fn trait_not_found(trait_: &str) -> DiagnosticBuilder {
        let def = Self::find("E4002").unwrap();
        def.builder().param("trait", trait_)
    }

    /// E4003 特质实现缺失
    pub fn missing_trait_impl(
        trait_: &str,
        type_: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E4003").unwrap();
        def.builder().param("trait", trait_).param("type", type_)
    }

    /// E4004 特质实现冲突
    pub fn conflicting_trait_impls(trait_: &str) -> DiagnosticBuilder {
        let def = Self::find("E4004").unwrap();
        def.builder().param("trait", trait_)
    }

    /// E4005 关联类型未找到
    pub fn associated_type_not_found(
        assoc_type: &str,
        container: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E4005").unwrap();
        def.builder()
            .param("assoc_type", assoc_type)
            .param("container", container)
    }

    // === 常量求值 ===

    /// E4010 常量除零
    pub fn const_division_by_zero() -> DiagnosticBuilder {
        let def = Self::find("E4010").unwrap();
        def.builder()
    }

    /// E4011 常量溢出
    pub fn const_overflow() -> DiagnosticBuilder {
        let def = Self::find("E4011").unwrap();
        def.builder()
    }

    /// E4012 常量递归过深
    pub fn const_recursion_too_deep(limit: usize) -> DiagnosticBuilder {
        let def = Self::find("E4012").unwrap();
        def.builder().param("limit", limit.to_string())
    }

    /// E4013 非常量函数
    pub fn const_non_const_function(func: &str) -> DiagnosticBuilder {
        let def = Self::find("E4013").unwrap();
        def.builder().param("func", func)
    }

    /// E4014 常量求值失败
    pub fn const_eval_failed(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E4014").unwrap();
        def.builder().param("reason", reason)
    }

    // === 终止检查 ===

    /// E4015 循环无法证明终止
    pub fn loop_may_not_terminate() -> DiagnosticBuilder {
        let def = Self::find("E4015").unwrap();
        def.builder()
    }

    /// E4016 递归无法证明终止
    pub fn recursion_may_not_terminate(func: &str) -> DiagnosticBuilder {
        let def = Self::find("E4016").unwrap();
        def.builder().param("func", func)
    }

    /// E4017 度量未严格递减
    pub fn measure_not_decreasing(measure: &str) -> DiagnosticBuilder {
        let def = Self::find("E4017").unwrap();
        def.builder().param("measure", measure)
    }
}
