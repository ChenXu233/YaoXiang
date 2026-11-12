//! Layer 0：类型等式证明（6.9.3b：三层判等，各只有一个实现）
//!
//! ## 内部组织
//!
//! ### 纯助手（无 ProofContext）
//! - `is_subtype`
//!
//! ### 判等入口（需要 ProofContext）
//! - `normalize_type`         ← 唯一类型归一化（包 `Evaluator` + ι + 实参递归）
//! - `defeq`                  ← 唯一等价判定（`Refined` 特判 → 同一性 → 归一化比较）
//! - `check_type_equivalence` ← `defeq` 的兼容入口（生产暂无调用者）
//!
//! ## 三层且各只有一个实现（t47 契约）
//!
//! 1. `syntactic_eq` = `MonoType` 派生 `PartialEq`（同一性；服务缓存键/实例键）；
//! 2. `normalize_type` = 唯一类型归一化；
//! 3. `defeq` = 归一化后比同一性。
//!
//! 6.9.3b 删除了 `structurally_equal`——它是派生 `PartialEq` 的**第二份**结构
//! 等价，且 `Refined` 臂只比 `base`（不同谓词判 true ⇒ 假等价，t47 E3）。

use crate::frontend::core::sema::environment::TypeEnvironment;
use crate::frontend::core::types::eval::evaluator::{EvalError, Evaluator};
use crate::frontend::core::types::mono::MonoType;
use super::super::proof::verdict::{
    DisproofKind, DisproofModel, ProofFailure, ProofResult, UnprovenReason,
};
use super::super::proof::context::ProofContext;

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
        // 容器协变（同名 Generic：List/Option 等；#299 List(a) <: List(b) 当 a <: b）
        (MonoType::Generic { name: na, args: aa }, MonoType::Generic { name: nb, args: ab })
            if na == nb && aa.len() == ab.len() && aa.len() == 1 =>
        {
            is_subtype(&aa[0], &ab[0], env)
        }
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
        // 6.8.7：单值类型 <: 基类型（`int64::3 <: int64`）。递归到底层基类型再判，
        // 因此只可能比基类型更宽、不会退化成「任意类型都是子类型」。
        // **反向没有对应臂**：基类型不保证等于那个唯一值。
        (MonoType::Literal { base_type, .. }, sup) => is_subtype(base_type, sup, env),
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

/// 类型归一的**唯一入口**（6.9.3b）：包既有 `Evaluator`，不新写求值器。
///
/// 为什么不能只调 `Evaluator::eval`（对 t51 盘点结论的正面补强）：
/// - `Generic` 实参**不**归约 ⇒ 这里递归进 `args`，再整体求值一次；
/// - `Literal` 没有 ι ⇒ 这里用 [`MonoType::literal_of_value`] 重建**规范形态**
///   （只由 `value` 决定，不读 `name`——派生 `PartialEq`/`Hash` 会读 `name`）；
/// - `Refined` 只归约 base 并**丢弃** `constraint` ⇒ 这里保留约束、只归约 base
///   （否则任何经本函数的使用者都会静默丢信息）。
///
/// 失败侧一律 `Unproven`（归约失败是「此刻算不出」，不是「算出不等」），
/// 映射见 [`eval_error_to_failure`]——**单一**映射，没有第二处。
pub fn normalize_type(
    ty: &MonoType,
    ctx: &ProofContext<'_>,
) -> Result<MonoType, ProofFailure> {
    let mut evaluator = Evaluator::new(ctx.env, &ctx.budget, &ctx.dep_env);
    normalize_type_with(&mut evaluator, ty, ctx)
}

/// [`normalize_type`] 的递归体（`evaluator` 由外层复用，缓存/预算同一份）。
fn normalize_type_with(
    evaluator: &mut Evaluator<'_>,
    ty: &MonoType,
    ctx: &ProofContext<'_>,
) -> Result<MonoType, ProofFailure> {
    // 预算：**一类型节点一步**（`Evaluator::eval` 的类型路径自己不吃预算——只有
    // `eval_expr` 吃，见 t47 E5）⇒ 归一化入口自己扣，耗尽即 `Unproven{BudgetExceeded}`。
    if !ctx.budget.spend() {
        return Err(ProofFailure::Unproven {
            reason: UnprovenReason::BudgetExceeded,
            proof_calls: vec![],
            budget: ctx.budget.report(),
        });
    }
    match ty {
        // ι：规范形态只由 `value` 决定（`name` 可能是任意来源的渲染文本）
        MonoType::Literal { value, .. } => {
            Ok(MonoType::literal_of_value(value).unwrap_or_else(|| ty.clone()))
        }
        // Evaluator 不碰 `Generic` 实参：逐个归约后整体再求值一次（实参变化
        // 可能让外层类型族/别名命中 δ/β）。
        MonoType::Generic { name, args } => {
            let mut normalized_args = Vec::with_capacity(args.len());
            for arg in args {
                normalized_args.push(normalize_type_with(evaluator, arg, ctx)?);
            }
            let rebuilt = MonoType::Generic {
                name: name.clone(),
                args: normalized_args,
            };
            evaluator
                .eval(&rebuilt)
                .map_err(|error| eval_error_to_failure(error, ctx))
        }
        // Evaluator 的 `Refined` 臂只归约 base 并丢约束；归一化不得丢信息
        MonoType::Refined { base, constraint } => Ok(MonoType::Refined {
            base: Box::new(normalize_type_with(evaluator, base, ctx)?),
            constraint: constraint.clone(),
        }),
        _ => evaluator
            .eval(ty)
            .map_err(|error| eval_error_to_failure(error, ctx)),
    }
}

/// `EvalError → ProofFailure` 的**单一**映射（6.9.3b）。
///
/// 归约失败 = 「此刻算不出」，**不是**「算出不等」⇒ 一律 `Unproven`，
/// 绝不 `Disproved`（`Disproved` 只由 [`defeq`] 在两侧都成功归约且确实不等时给出）。
fn eval_error_to_failure(
    error: EvalError,
    ctx: &ProofContext<'_>,
) -> ProofFailure {
    let budget = ctx.budget.report();
    // `Evaluator` 把**预算耗尽**表达成 `MaxDepthExceeded`（与「结构过深」同名，见
    // t47 E5）——按预算是否真的耗尽分流，不把两者混成一类。
    if budget.steps_used >= budget.steps_limit {
        ProofFailure::Unproven {
            reason: UnprovenReason::BudgetExceeded,
            proof_calls: vec![],
            budget,
        }
    } else {
        ProofFailure::Unproven {
            reason: UnprovenReason::BeyondKernel(format!("{error:?}")),
            proof_calls: vec![],
            budget,
        }
    }
}

/// 类型等价的**唯一判定入口**（6.9.3b）。
///
/// 判定顺序（顺序本身是语义的一部分）：
/// 0. **`Refined` 先特判**：`Evaluator` 归约 `Refined` 会丢 `constraint`，先归一化
///    就会把任意同基精化类型判等（t47 E3 的假等价）。两侧 `Refined`：谓词相同 ⇒
///    归约基类型后判等；谓词不同、或一侧精化一侧不是 ⇒ `Unproven`（判等超出内核，
///    **不是**「不等」）。
/// 1. 派生 `PartialEq` 短路 = 同一性（`syntactic_eq`，与缓存键/实例键同一口径）。
/// 2. 两侧 [`normalize_type`]：都成功且相等 ⇒ `Proved`；都成功但不等 ⇒ `Disproved`；
///    任一侧 `Unproven` ⇒ **原样传播**（绝不降格成 `Disproved`）。
pub fn defeq(
    lhs: &MonoType,
    rhs: &MonoType,
    ctx: &ProofContext<'_>,
) -> ProofResult {
    // 0. 精化类型：先比谓词，再比归约后的基类型
    match (lhs, rhs) {
        (
            MonoType::Refined {
                base: lhs_base,
                constraint: lhs_constraint,
            },
            MonoType::Refined {
                base: rhs_base,
                constraint: rhs_constraint,
            },
        ) => {
            if lhs_constraint != rhs_constraint {
                return Err(ProofFailure::Unproven {
                    reason: UnprovenReason::BeyondKernel(format!(
                        "精化类型谓词不同，判等超出内核：{} vs {}",
                        lhs_constraint, rhs_constraint
                    )),
                    proof_calls: vec![],
                    budget: ctx.budget.report(),
                });
            }
            return defeq(lhs_base, rhs_base, ctx);
        }
        (MonoType::Refined { .. }, _) | (_, MonoType::Refined { .. }) => {
            return Err(ProofFailure::Unproven {
                reason: UnprovenReason::BeyondKernel(format!(
                    "精化类型与非精化类型判等超出内核：{} vs {}",
                    lhs, rhs
                )),
                proof_calls: vec![],
                budget: ctx.budget.report(),
            });
        }
        _ => {}
    }

    // 1. 同一性短路（派生 PartialEq，不经归约）
    if lhs == rhs {
        return Ok(());
    }

    // 2. 两侧归一化后比较
    match (normalize_type(lhs, ctx), normalize_type(rhs, ctx)) {
        (Ok(normalized_lhs), Ok(normalized_rhs)) if normalized_lhs == normalized_rhs => Ok(()),
        (Ok(normalized_lhs), Ok(normalized_rhs)) => Err(ProofFailure::Disproved(DisproofModel {
            kind: DisproofKind::TypeMismatch,
            assignments: vec![
                ("expected".into(), format!("{normalized_lhs:?}")),
                ("found".into(), format!("{normalized_rhs:?}")),
            ],
            constraint: format!("{} == {}", normalized_lhs, normalized_rhs),
            span: None,
            predicate_span: None,
        })),
        // 任一侧 `Unproven`：原样传播，**不得**降格成 `Disproved`
        (Err(failure), _) | (_, Err(failure)) => Err(failure),
    }
}

/// 检查类型等式（`defeq` 的兼容入口；生产暂无调用者，见 t47 E2）。
pub fn check_type_equivalence(
    ctx: &ProofContext<'_>,
    lhs: &MonoType,
    rhs: &MonoType,
) -> ProofResult {
    defeq(lhs, rhs, ctx)
}
