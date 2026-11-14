//! 类型推断模块
//!
//! 实现 Hindley-Milner 类型推断算法
//! 统一的类型推断入口，合并了原 checking/ 和 inference/ 模块

// ✅ 核心模块
// #452 / #454 / t12-F1：命名实参槽位绑定判据（唯一权威点，三处调用点共用）
pub mod arg_binding;
pub mod call_ownership;
pub mod expressions;
pub mod scope;
pub mod statements;
pub mod types;

// ✅ 从 checking/ 移入的模块
pub mod bounds;

pub mod patterns;

// RFC-011a §6 存在类型强制点收集
pub mod existential;

// 重新导出核心类型
pub use scope::ScopeManager;
pub use types::TypeSystem;
pub use statements::StatementChecker;
pub use expressions::ExpressionInferrer;
pub use bounds::BoundsChecker;
pub use patterns::PatternInferrer;

// 向后兼容别名
pub use expressions::ExprInferrer;

pub use crate::frontend::core::types::{MonoType, PolyType, TypeConstraintSolver};

/// 类型级 witness-bearing 判定（6.9.5-B.1c / 5A 的唯一读取点）。
///
/// 记录是否 witness-bearing 由 `types/mono.rs` 的 `is_witness_bearing` 回答；
/// 本函数只做**类型形状归一**：剥 `Ref` 层，`Struct` 直接读字段表，
/// `TypeRef` 经类型定义表解析一层（与 `StatementChecker::resolve_type_ref_type`
/// 的口径相同，不做别名链递归）。泛型记录实例不在本批展开——witness 的产出
/// 入 B.2，泛型记录面留 B.4。
pub(super) fn mono_type_is_witness_bearing(
    ty: &MonoType,
    type_defs: &std::collections::HashMap<String, PolyType>,
) -> bool {
    use crate::frontend::core::types::mono::is_witness_bearing;
    let mut current = ty.clone();
    while let MonoType::Ref { inner, .. } = current {
        current = *inner;
    }
    match current {
        MonoType::Struct(struct_type) => is_witness_bearing(&struct_type.fields),
        MonoType::TypeRef(name) => match type_defs.get(&name).map(|poly| &poly.body) {
            Some(MonoType::Struct(struct_type)) => is_witness_bearing(&struct_type.fields),
            _ => false,
        },
        _ => false,
    }
}

pub use crate::util::diagnostic::{Diagnostic, Result};

// ============ 测试模块 ============

#[cfg(test)]
mod tests;
