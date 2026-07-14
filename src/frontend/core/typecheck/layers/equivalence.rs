//! Layer 0：类型等式证明
//!
//! ## 内部组织
//!
//! ### 纯助手（无 ProofContext）
//! - `structurally_equal`
//! - `is_subtype`            ← 新增
//!
//! ### 证明入口（需要 ProofContext）
//! - `check_type_equivalence`
//!
//! 检查两个类型是否等价。先做结构等价快速路径，
//! 再调用 types/eval/ 做确定性归约，将归约结果包装为 ProofResult。

use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::types::eval::evaluator::Evaluator;
use crate::frontend::core::types::mono::MonoType;
use super::super::proof::verdict::{DisproofKind, DisproofModel, ProofResult, UnprovenReason};
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
        (MonoType::Never, MonoType::Never) => true,
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

/// 检查 A 是否为 B 的子类型（纯函数）
///
/// Layer 0 的纯助手。不需要 ProofContext。
/// env=None 时不查鸭子类型；env=Some(...) 时支持方法绑定查询。
pub fn is_subtype(
    sub: &MonoType,
    sup: &MonoType,
    env: Option<&TypeEnvironment>,
) -> bool {
    match (sub, sup) {
        (a, b) if a == b => true,
        // Never <: T 对所有 T 成立（爆炸原理）
        (MonoType::Never, _) => true,
        // MetaType 层级弱检查：
        // Typeₙ <: Typeₘ 当且仅当 n ≤ m
        (
            MonoType::MetaType {
                universe_level: la, ..
            },
            MonoType::MetaType {
                universe_level: lb, ..
            },
        ) => la.le(lb),
        // List 协变
        (MonoType::List(a), MonoType::List(b)) => is_subtype(a, b, env),
        // 函数：参数逆变 + 返回值协变
        (
            MonoType::Fn {
                params: a_params,
                return_type: a_ret,
            },
            MonoType::Fn {
                params: b_params,
                return_type: b_ret,
            },
        ) => {
            let params_ok = a_params.len() == b_params.len()
                && a_params
                    .iter()
                    .zip(b_params.iter())
                    .all(|(a, b)| is_subtype(b, a, env));
            let ret_ok = is_subtype(a_ret, b_ret, env);
            params_ok && ret_ok
        }
        // 结构体：若 sup 是约束类型，走鸭子类型
        (MonoType::Struct(_), MonoType::Struct(_)) if sup.is_constraint() => {
            satisfies_constraint(sub, sup, env)
        }
        (MonoType::Struct(a), MonoType::Struct(b)) => {
            if a.name != b.name || a.fields.len() != b.fields.len() {
                return false;
            }
            a.fields
                .iter()
                .zip(b.fields.iter())
                .all(|(af, bf)| af.0 == bf.0 && is_subtype(&af.1, &bf.1, env))
        }
        // 非结构体对约束类型：尝试鸭子类型
        (_, MonoType::Struct(_)) if sup.is_constraint() => satisfies_constraint(sub, sup, env),
        _ => false,
    }
}

/// 检查具体类型是否满足约束类型（接口）的方法要求
///
/// 鸭子类型：约束类型的每个函数字段都必须在 sub 中存在且签名兼容。
fn satisfies_constraint(
    sub: &MonoType,
    constraint: &MonoType,
    _env: Option<&TypeEnvironment>,
) -> bool {
    let constraint_fields = match constraint {
        MonoType::Struct(s) => &s.fields,
        _ => return false,
    };
    let sub_fn_fields: Vec<&(String, MonoType)> = match sub {
        MonoType::Struct(s) => s
            .fields
            .iter()
            .filter(|(_, t)| matches!(t, MonoType::Fn { .. }))
            .collect(),
        _ => return false,
    };
    for (field_name, constraint_fn) in constraint_fields {
        let found = sub_fn_fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, t)| t);
        match (found, None::<&MonoType>) {
            (Some(found_fn), _) => {
                if !fn_signatures_match(found_fn, constraint_fn) {
                    return false;
                }
            }
            (None, _) => return false,
        }
    }
    true
}

/// 检查两个函数签名是否精确匹配（私有助手，仅满足约束用）
fn fn_signatures_match(
    a: &MonoType,
    b: &MonoType,
) -> bool {
    match (a, b) {
        (
            MonoType::Fn {
                params: a_params,
                return_type: a_ret,
            },
            MonoType::Fn {
                params: b_params,
                return_type: b_ret,
            },
        ) => {
            a_params.len() == b_params.len()
                && a_params.iter().zip(b_params.iter()).all(|(x, y)| x == y)
                && a_ret == b_ret
        }
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

    // 2. 子类型快速路径：双向子类型 ⇒ 等价
    if is_subtype(lhs, rhs, Some(ctx.env)) && is_subtype(rhs, lhs, Some(ctx.env)) {
        return ProofResult::Proved;
    }

    // 3. 确定性归约后比较
    let mut evaluator = Evaluator::new(ctx.env, &ctx.budget, &ctx.dep_env);
    match (evaluator.eval(lhs), evaluator.eval(rhs)) {
        (Ok(l), Ok(r)) if l == r => ProofResult::Proved,
        (Ok(l), Ok(r)) => ProofResult::Disproved(DisproofModel {
            kind: DisproofKind::TypeMismatch,
            assignments: vec![
                ("expected".into(), format!("{:?}", l)),
                ("found".into(), format!("{:?}", r)),
            ],
            constraint: format!("{} == {}", l, r),
            span: None,
            predicate_span: None,
        }),
        (Err(e), _) | (_, Err(e)) => ProofResult::Unproven {
            reason: UnprovenReason::BeyondKernel(format!("{:?}", e)),
            proof_calls: vec![],
            budget: ctx.budget.report(),
        },
    }
}
