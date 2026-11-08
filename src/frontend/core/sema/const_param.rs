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

/// 用途精筛的**唯一入口**：类型位置上对候选名的引用扫描（整棵 `Type` 子树）。
///
/// 「类型位置」= 声明中的全部 `Type` 子树（SPEC type-system §7.1 / RFC-011 §4.1）：
/// 形参标注、返回类型、内层 `Fn` 的形参与返回位、`Assert` 谓词、类型构造实参位、
/// 类型体字段类型都是同一棵树在不同语法位置上的实例，故一次递归遍历收口，
/// **不按位置分情况**。只扫返回位、或只扫「返回位 + 返回位内层 Fn 的形参」，会漏掉
/// 本层形参自己的标注：`f: (N: Int, M: N) -> Int` 里 `M: N` 的 `N` 从未计入用途 ⇒
/// `const_binders` 为空 ⇒ `M` 的标注停在裸名 → E1012。
///
/// 判定顺序与 `ast::classify_compile_time_value_params` 同序同义（SPEC §7.1）：
///
/// 1. **命名空间解析**（`is_type_name` 注入）：解析到**已声明类型** ⇒ 与形参无关，
///    候选落空——形参名与类型名同形不构成遮蔽（`(Int: Int)` 的两个 `Int` 分属
///    值/类型命名空间）；
/// 2. **用途精筛**（`ast::name_used_as_type`）：名字在任一 `Type` 子树被引用 ⇒ 命中。
///
/// 扫描面本身委托 `ast::name_used_as_type`（全仓唯一实现）；本函数只叠加候选集与
/// 命名空间两层注入。四个调用点共用它——Registry 面
/// （`TypeChecker::collect_signatures`）、签名面
/// （`TypeChecker::collect_function_signature`）、函数体面（`StatementChecker`）、
/// 类型构造器面（`TypeChecker::add_type_definition`）——「类型位置」这条判据因此
/// 只有一份实现。
///
/// **`is_type_name` 的口径（四处同源）**：四个调用点注入的都是「**已声明类型**」
/// 集合——`env.types` 的键（本地类型定义 + `use` 导入的类型导出），与唯一分类器
/// `ast::classify_compile_time_value_params` 的 `ParamClassifier::is_type_name` 同一
/// 集合。checker 三面经 `TypeChecker::is_declared_type_name`、函数体面经
/// `StatementChecker::type_names`，回答同一问题。
///
/// 它**不是** `TypeEnvironment::resolves_type_name`：后者回答「这名字能不能当类型
/// 标注用」（含内置名 / 编译器容器 / 接口名 / std 导出类型），问题不同。此处按
/// SPEC §7.1 第 2 条的字面口径（「解析到**已声明类型**」）取窄。
///
/// ⚠ **未裁决的设计问题**：「已声明类型」是否应含内置名（`Int`/`Bool`…）——SPEC 的
/// 例子（`(Int: Int) -> Int` 的两个 `Int` 分属不同命名空间）字面上要求内置名也算类型，
/// 但本轮（6.8.10 丙案）按字面只取 `env.types`：放宽会让 `(Bool: Int) -> Vec(Bool)`
/// 这类形态在 sema 面放行、而 IR 面（`ast::extract_generic_param_names` /
/// `ir_gen::declaration_runtime_param_slots` 的命名空间问答）仍按编译期值参数出 IR，
/// 变成「编译通过 + 运行期崩」。放宽须先补齐 IR 面通道，属另一裁决。
pub fn used_as_const_in_annotation(
    annotation: &Type,
    candidates: &HashSet<String>,
    is_type_name: &dyn Fn(&str) -> bool,
    is_predicate: &dyn Fn(&str) -> bool,
) -> HashSet<String> {
    candidates
        .iter()
        .filter(|name| {
            !is_type_name(name) && crate::ast::name_used_as_type(name, annotation, is_predicate)
        })
        .cloned()
        .collect()
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
