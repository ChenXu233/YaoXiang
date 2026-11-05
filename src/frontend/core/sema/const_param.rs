//! 编译期值参数判定 —— RFC-011 §4.1 两步判定的唯一实现
//!
//! 形态粗筛（classify_generic_params，ast.rs）产出 Const 候选；
//! 本模块的精筛步骤按「类型位置是否被引用」给出终态：
//! 命中 → const_binders（编译期代换）；未命中 → 落空（函数级退化为
//! 运行时值参数，构造器级报 E1094）。落空分流策略由调用语境决定，
//! 判定本身只在这里发生一次。

use std::collections::HashSet;

use crate::ast::{GenericParam, GenericParamKind, Param, Type};
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
        _ => None,
    };
    name.and_then(ConstKind::from_ast_type_name)
        .unwrap_or(ConstKind::Int(None))
}

/// 用途精筛的**扫描面**单点：声明标注（`type_annotation`）整棵树上的候选引用。
///
/// 「类型位置」= 声明中的全部 `Type` 子树（SPEC type-system §7.1 / RFC-011 §4.1）：
/// 形参标注、返回类型、内层 `Fn` 的形参与返回位、`Assert` 谓词、类型构造实参位
/// 都是同一棵树在不同语法位置上的实例，故一次递归遍历收口，**不按位置分情况**。
/// 只扫返回位、或只扫「返回位 + 返回位内层 Fn 的形参」，会漏掉本层形参自己的标注：
/// `f: (N: Int, M: N) -> Int` 里 `M: N` 的 `N` 从未计入用途 ⇒ `const_binders`
/// 为空 ⇒ `M` 的标注停在裸名 → E1012。
///
/// 三个调用点共用本函数——Registry 面（`TypeChecker::collect_signatures`）、
/// 签名面（`TypeChecker::collect_function_signature`）、函数体面
/// （`StatementChecker`）——「类型位置」这条判据因此只有一份实现。
///
/// 本函数**不**委托 `ast::name_used_as_type`：后者对 `ConstExpr` / `MetaType` /
/// `Paren` / `NamedParen` 落 `_ => false`（见该函数的「已知共同缺口」），直接换会
/// 把 `Assert(N > 0)` 判成落空。合并到唯一分类器须先补齐那四个变体。
pub fn used_as_const_in_annotation(
    annotation: &Type,
    candidates: &HashSet<String>,
    is_predicate: &dyn Fn(&str) -> bool,
) -> HashSet<String> {
    let mut used = HashSet::new();
    if !candidates.is_empty() {
        crate::frontend::core::sema::checker::collect_used_in_type(
            annotation,
            candidates,
            &mut used,
            is_predicate,
        );
    }
    used
}

/// 用途精筛：候选中被 `used_as_const` 引用者进 const_binders，
/// 其余作为落空者返回（携带原始 Param 以便并入参数链或报错）。
pub fn resolve_const_candidates(
    generic_params: &[GenericParam],
    decl_params: &[Param],
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
        } else if let Some(p) = decl_params.iter().find(|p| p.name == gp.name) {
            result.fallen.push(p.clone());
        }
    }
    result
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
