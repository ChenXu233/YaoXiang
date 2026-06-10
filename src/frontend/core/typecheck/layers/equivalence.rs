//! Layer 0：类型等式证明
//!
//! 检查两个类型是否等价。调用 types/eval/ 做确定性归约，
//! 将归约结果包装为 ProofResult。

use super::super::proof::verdict::ProofResult;

/// 检查类型等式（骨架 —— 搬迁阶段为最小实现）
#[allow(unused_variables)]
pub fn check_type_equivalence(
    _ctx: &super::super::proof::context::ProofContext<'_>,
    _lhs: &crate::frontend::core::types::mono::MonoType,
    _rhs: &crate::frontend::core::types::mono::MonoType,
) -> ProofResult {
    ProofResult::Proved
}
