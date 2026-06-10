//! Layer 2：终止性证明
//!
//! 检查循环和递归函数是否在有限步内终止。
//! 搬迁阶段：骨架。后续步骤迁移 termination/ 逻辑至此。

use super::super::proof::verdict::ProofResult;

/// 检查终止性（骨架）
#[allow(unused_variables)]
pub fn check_termination(_ctx: &super::super::proof::context::ProofContext<'_>) -> ProofResult {
    ProofResult::Proved
}
