//! Layer 1：所有权/令牌证明
//!
//! 委托 middle/passes/lifetime/ 执行所有权检查，
//! 将结果统一为 ProofResult。

use super::super::proof::verdict::ProofResult;

/// 检查所有权无冲突（骨架）
#[allow(unused_variables)]
pub fn check_ownership(_ctx: &super::super::proof::context::ProofContext<'_>) -> ProofResult {
    ProofResult::Proved
}
