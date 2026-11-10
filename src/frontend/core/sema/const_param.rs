//! 编译期值参数判定 —— RFC-011 §4.1 两步判定的唯一落地
//!
//! 形态粗筛（`classify_generic_params`，ast）产出 Const 候选；精筛按「类型位置是否
//! 被引用」给出终态。**精筛本身不在这里判定**：唯一判据是
//! `ast::classify_compile_time_value_params`，唯一解析源是
//! `ast::resolve_param_quantities`（把结论写成 `ast::Param::quantity == Some(Zero)`）。
//! 本模块只做两件事：
//!
//! 1. 把已解析的量词事实落成编译期代换绑定（[`resolve_const_candidates`]）；
//! 2. 回答「哪些 Const 候选**没有**成为 q=0 绑定」（`fallen_const_candidates`）——
//!    函数级把它们当普通运行期值参数，构造器级由调用方发 E1094。
//!
//! **已退休**（D62 / issue #533）：`ResolvedConstParams::fallen` 那条独立通道以前
//! 自己再扫一遍用途、把落空候选作为第二个返回值带出。落空不再是一个平行账本，
//! 而是「`quantity != Some(Zero)`」这一条事实的派生读法——两条通道曾对同一候选给出
//! 分叉结论的风险随之消失。

use crate::ast::{GenericParam, GenericParamKind, Param, Quantity, Type};
use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef};

/// 精筛结果：**只**保有命中的绑定（落空者由 `fallen_const_candidates` 从量词事实派生）。
pub struct ResolvedConstParams {
    /// 用途命中的编译期值参数（index 连续，基于 n_type_params 起编）
    pub const_binders: Vec<ConstVarDef>,
}

/// annotation 类型 → ConstKind（收编原先三处逐字重复的 match 块）。
pub fn ast_type_to_const_kind(ty: &Type) -> ConstKind {
    let name = match ty {
        Type::Name { name, .. } => Some(name.as_str()),
        _ => None,
    };
    name.and_then(ConstKind::from_ast_type_name)
        .unwrap_or(ConstKind::Int(None))
}

/// 该候选名是否已成为 **q=0 绑定**（量词事实的**唯一**读法）。
///
/// 声明内形参名唯一，故按名取；取不到 = 没有解析到该形参，按「未成为绑定」处理。
fn is_zero_binding(
    name: &str,
    resolved_params: &[Param],
) -> bool {
    resolved_params
        .iter()
        .any(|p| p.name == name && p.quantity == Some(Quantity::Zero))
}

/// 用途命中的候选进 const_binders（index 从 n_type_params 起连续编号）。
///
/// `resolved_params` 必须是 [`crate::ast::resolve_param_quantities`] 的产物：
/// 本函数**不**再自己判一次用途——那是签名面 / 函数体面 / 构造器面各自的
/// 注入问答（命名空间、谓词）该决定的事，判定只在唯一分类器里发生一次。
pub fn resolve_const_candidates(
    generic_params: &[GenericParam],
    resolved_params: &[Param],
    n_type_params: usize,
) -> ResolvedConstParams {
    let mut const_binders: Vec<ConstVarDef> = Vec::new();
    for gp in generic_params
        .iter()
        .filter(|p| matches!(p.kind, GenericParamKind::Const { .. }))
    {
        if !is_zero_binding(&gp.name, resolved_params) {
            continue;
        }
        if let GenericParamKind::Const { const_type } = &gp.kind {
            let idx = n_type_params + const_binders.len();
            const_binders.push(ConstVarDef::new(
                gp.name.clone(),
                ast_type_to_const_kind(const_type),
                idx,
            ));
        }
    }
    ResolvedConstParams { const_binders }
}

/// **落空的 Const 候选**（统一绑定检查的输入）：标注是 const 参数类型、但没成为
/// q=0 绑定。返回它们的**声明形参**（携带名字与 span，供诊断指向）。
///
/// 谁报错由调用语境决定（RFC-011 §4.1）：函数级这些位是**普通运行期值参数**，
/// 不报；构造器级实例化 arity 会对不上，必须报 E1094（#297 / F）。
///
/// 判据只有 [`is_zero_binding`] 一条：不再维护「落空通道」这份平行账本。
pub(crate) fn fallen_const_candidates<'a>(
    generic_params: &'a [GenericParam],
    resolved_params: &'a [Param],
) -> Vec<&'a Param> {
    generic_params
        .iter()
        .filter(|gp| matches!(gp.kind, GenericParamKind::Const { .. }))
        .filter(|gp| !is_zero_binding(&gp.name, resolved_params))
        .filter_map(|gp| resolved_params.iter().find(|p| p.name == gp.name))
        .collect()
}

/// 字面量 → 编译期常量值（**只认字面量**）。
///
/// 这是「可证明为常量」的最小判据，6.8.5-1 起被 const 实参特化与 6.8.7 的单值类型
/// 定型共用；放在本模块（const 参数判据的家）而不是散在推断器里，是为了让写入端
/// （值环境登记）与消费端（特化请求）用**同一份**判据，不出现两套口径。
pub fn literal_const_value(
    lit: &crate::frontend::core::lexer::tokens::Literal
) -> Option<crate::frontend::core::types::const_data::ConstValue> {
    use crate::frontend::core::lexer::tokens::Literal;
    use crate::frontend::core::types::const_data::ConstValue;
    match lit {
        Literal::Int(n) => Some(ConstValue::Int(*n)),
        Literal::Bool(b) => Some(ConstValue::Bool(*b)),
        Literal::Float(f) => Some(ConstValue::Float(*f as f32)),
        _ => None,
    }
}

/// 实参/初始化式表达式 → 编译期常量值（只认字面量）。
pub fn const_literal_of_expr(
    expr: &crate::ast::Expr
) -> Option<crate::frontend::core::types::const_data::ConstValue> {
    use crate::ast::Expr;
    let Expr::Lit(lit, _) = expr else {
        return None;
    };
    literal_const_value(lit)
}
