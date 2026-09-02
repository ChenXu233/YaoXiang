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
        code: "E4014",
        category: ErrorCategory::Generic,
    },
    // === E401x: 终止检查 ===
    // === E401x: 精化谓词证明 ===
    ErrorCodeDefinition {
        code: "E4018",
        category: ErrorCategory::Generic,
    },
    ErrorCodeDefinition {
        code: "E4019",
        category: ErrorCategory::Generic,
    },
    // E4020: 编译期证明需要证明函数（RFC-027，#322 收敛时新增）
    ErrorCodeDefinition {
        code: "E4020",
        category: ErrorCategory::Generic,
    },
];

// 快捷方法（code_helpers! 生成）
impl ErrorCodeDefinition {
    code_helpers! {
    /// E4001 类型不满足特质约束
    ("E4001", trait_bound_not_satisfied(type_: &str, trait_: &str) => .param("type", type_).param("trait", trait_)),
    /// E4002 特质未找到
    ("E4002", trait_not_found(trait_: &str) => .param("trait", trait_)),
    /// E4003 特质实现缺失
    ("E4003", missing_trait_impl(trait_: &str, type_: &str) => .param("trait", trait_).param("type", type_)),
    /// E4004 特质实现冲突
    ("E4004", conflicting_trait_impls(trait_: &str) => .param("trait", trait_)),
    /// E4005 关联类型未找到
    ("E4005", associated_type_not_found(assoc_type: &str, container: &str) => .param("assoc_type", assoc_type) .param("container", container)),
    /// E4010 常量除零
    ("E4010", const_division_by_zero() => ),
    /// E4011 常量溢出
    ("E4011", const_overflow() => ),
    /// E4012 常量递归过深
    ("E4012", const_recursion_too_deep(limit: usize) => .param("limit", limit.to_string())),
    /// E4014 常量求值失败
    ("E4014", const_eval_failed(reason: &str) => .param("reason", reason)),
    /// E4018 精化谓词违反
    ("E4018", refinement_violated(constraint: &str) => .param("constraint", constraint)),
    /// E4019 类型等式不成立（证明管道内）
    ("E4019", type_mismatch_in_proof(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    /// E4020 需要证明函数来验证约束
    ("E4020", proof_function_required() => ),
    }
}
