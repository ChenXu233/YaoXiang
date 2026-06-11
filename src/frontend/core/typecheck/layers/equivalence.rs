//! Layer 0：类型等式证明
//!
//! 检查两个类型是否等价。先做结构等价快速路径，
//! 再调用 types/eval/ 做确定性归约，将归约结果包装为 ProofResult。

use crate::frontend::core::types::eval::evaluator::Evaluator;
use crate::frontend::core::types::mono::MonoType;
use super::super::proof::verdict::{DisproofModel, ProofResult, UnprovenReason};
use super::super::proof::context::ProofContext;

/// 结构等价快速路径：O(n) 递归比较两个类型结构
///
/// 不调用 Evaluator，纯结构比较。在调用昂贵的归约求值之前，
/// 先检查两个类型是否在结构上已经相同。
pub fn structurally_equal(
    lhs: &MonoType,
    rhs: &MonoType,
) -> bool {
    match (lhs, rhs) {
        // 基础类型
        (MonoType::Void, MonoType::Void) => true,
        (MonoType::Bool, MonoType::Bool) => true,
        (MonoType::Int(a), MonoType::Int(b)) => a == b,
        (MonoType::Float(a), MonoType::Float(b)) => a == b,
        (MonoType::Char, MonoType::Char) => true,
        (MonoType::String, MonoType::String) => true,
        (MonoType::Bytes, MonoType::Bytes) => true,
        // 类型引用
        (MonoType::TypeRef(a), MonoType::TypeRef(b)) => a == b,
        // 函数类型
        (
            MonoType::Fn {
                params: pa,
                return_type: ra,
            },
            MonoType::Fn {
                params: pb,
                return_type: rb,
            },
        ) => {
            pa.len() == pb.len()
                && pa
                    .iter()
                    .zip(pb.iter())
                    .all(|(a, b)| structurally_equal(a, b))
                && structurally_equal(ra, rb)
        }
        // 容器类型
        (MonoType::List(a), MonoType::List(b)) => structurally_equal(a, b),
        (MonoType::Option(a), MonoType::Option(b)) => structurally_equal(a, b),
        (MonoType::Tuple(a), MonoType::Tuple(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b.iter())
                    .all(|(x, y)| structurally_equal(x, y))
        }
        // Refined 只比较基类型（约束可能不同但基类型相同的视为结构等价）
        (MonoType::Refined { base: ba, .. }, MonoType::Refined { base: bb, .. }) => {
            structurally_equal(ba, bb)
        }
        // MetaType
        (
            MonoType::MetaType {
                universe_level: ua, ..
            },
            MonoType::MetaType {
                universe_level: ub, ..
            },
        ) => ua == ub,
        // Struct / Enum
        (MonoType::Struct(sa), MonoType::Struct(sb)) => sa == sb,
        (MonoType::Enum(ea), MonoType::Enum(eb)) => ea == eb,
        // 不匹配的结构
        _ => false,
    }
}

/// 检查类型等式
pub fn check_type_equivalence(
    ctx: &ProofContext<'_>,
    lhs: &MonoType,
    rhs: &MonoType,
) -> ProofResult {
    // 1. 结构等价快速路径
    if structurally_equal(lhs, rhs) {
        return ProofResult::Proved;
    }

    // 2. 确定性归约后比较
    let mut evaluator = Evaluator::new(ctx.env, &ctx.budget);
    match (evaluator.eval(lhs), evaluator.eval(rhs)) {
        (Ok(l), Ok(r)) if l == r => ProofResult::Proved,
        (Ok(l), Ok(r)) => ProofResult::Disproved(DisproofModel {
            assignments: vec![
                ("lhs".into(), format!("{:?}", l)),
                ("rhs".into(), format!("{:?}", r)),
            ],
        }),
        (Err(e), _) | (_, Err(e)) => ProofResult::Unproven {
            reason: UnprovenReason::BeyondKernel(format!("{:?}", e)),
            budget: ctx.budget.report(),
        },
    }
}
