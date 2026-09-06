//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(E4XXX, {
    // E4001 类型不满足特质约束
    ("E4001", Generic, false, trait_bound_not_satisfied(type_: &str, trait_: &str) => .param("type", type_).param("trait", trait_)),
    // E4002 特质未找到
    ("E4002", Generic, false, trait_not_found(trait_: &str) => .param("trait", trait_)),
    // E4003 特质实现缺失
    ("E4003", Generic, false, missing_trait_impl(trait_: &str, type_: &str) => .param("trait", trait_).param("type", type_)),
    // E4004 特质实现冲突
    ("E4004", Generic, false, conflicting_trait_impls(trait_: &str) => .param("trait", trait_)),
    // E4005 关联类型未找到
    ("E4005", Generic, false, associated_type_not_found(assoc_type: &str, container: &str) => .param("assoc_type", assoc_type) .param("container", container)),
    // E4010 常量除零
    ("E4010", Generic, false, const_division_by_zero() => ),
    // E4011 常量溢出
    ("E4011", Generic, false, const_overflow() => ),
    // E4012 常量递归过深
    ("E4012", Generic, false, const_recursion_too_deep(limit: usize) => .param("limit", limit.to_string())),
    // E4014 常量求值失败
    ("E4014", Generic, true, const_eval_failed(reason: &str) => .param("reason", reason)),
    // E4018 精化谓词违反
    ("E4018", Generic, true, refinement_violated(constraint: &str) => .param("constraint", constraint)),
    // E4019 类型等式不成立（证明管道内）
    ("E4019", Generic, false, type_mismatch_in_proof(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    // E4020 需要证明函数来验证约束
    ("E4020", Generic, false, proof_function_required() => ),
});
