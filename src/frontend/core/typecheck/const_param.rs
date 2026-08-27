//! 编译期值参数判定 —— RFC-011 §4.1 两步判定的唯一实现
//!
//! 形态粗筛（classify_generic_params，ast.rs）产出 Const 候选；
//! 本模块的精筛步骤按「类型位置是否被引用」给出终态：
//! 命中 → const_binders（编译期代换）；未命中 → 落空（函数级退化为
//! 运行时值参数，构造器级报 E1094）。落空分流策略由调用语境决定，
//! 判定本身只在这里发生一次。

use std::collections::HashSet;

use crate::frontend::core::parser::ast::{GenericParam, GenericParamKind, Param, Type};
use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef};

/// 精筛结果：命中者与落空者。
pub struct ResolvedConstParams {
    /// 用途命中的编译期值参数（index 连续，基于 n_type_params 起编）
    pub const_binders: Vec<ConstVarDef>,
    /// 落空候选对应的原始签名参数（普通值参数 / E1094 报错对象）
    pub fallen: Vec<Param>,
}

/// annotation 类型 → ConstKind（收编原先三处逐字重复的 match 块）。
pub fn ast_type_to_const_kind(ty: &Type) -> ConstKind {
    let name = match ty {
        Type::Name { name, .. } => Some(name.as_str()),
        Type::Int(_) => Some("Int"),
        Type::Float(_) => Some("Float"),
        Type::Bool => Some("Bool"),
        _ => None,
    };
    name.and_then(ConstKind::from_ast_type_name)
        .unwrap_or(ConstKind::Int(None))
}

/// 用途精筛：候选中被 `used_as_const` 引用者进 const_binders，
/// 其余作为落空者返回（携带原始 Param 以便并入参数链或报错）。
pub fn resolve_const_candidates(
    generic_params: &[GenericParam],
    signature_params: &[Param],
    used_as_const: &HashSet<String>,
    n_type_params: usize,
) -> ResolvedConstParams {
    let mut result = ResolvedConstParams {
        const_binders: Vec::new(),
        fallen: Vec::new(),
    };
    for gp in generic_params
        .iter()
        .filter(|p| matches!(p.kind, GenericParamKind::Const { .. }))
    {
        if used_as_const.contains(&gp.name) {
            if let GenericParamKind::Const { const_type } = &gp.kind {
                let idx = n_type_params + result.const_binders.len();
                result.const_binders.push(ConstVarDef::new(
                    gp.name.clone(),
                    ast_type_to_const_kind(const_type),
                    idx,
                ));
            }
        } else if let Some(p) = signature_params.iter().find(|p| p.name == gp.name) {
            result.fallen.push(p.clone());
        }
    }
    result
}
