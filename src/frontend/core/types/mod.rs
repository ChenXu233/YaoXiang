//! 类型系统 IR
//!
//! 定义类型的数据结构和对类型的纯操作。
//! 不包含程序上下文、路径条件、证明逻辑。

pub mod const_data;
pub mod constraint;
pub mod error;
pub mod mono;
pub mod solver;
pub mod substitute;
pub mod trait_data;
pub mod var;

pub mod eval; // 替代原 computation 模块

// 向后兼容：重新导出原 base/ 公开的类型
pub use const_data::{BinOp, ConstExpr, ConstKind, ConstValue, ConstVarDef, UnOp};
pub use constraint::TypeConstraint;
pub use error::TypeConstraintError;
pub use mono::{
    get_ast_type_universe_level, calculate_meta_type_level, ast_type_to_poly_type, EnumType,
    MonoType, PolyType, StructType, TypeBinding, UniverseLevel,
};
pub use solver::TypeConstraintSolver;
pub use substitute::{Substituter, Substitution};
pub use trait_data::{
    TraitBound, TraitBounds, TraitDefinition, TraitImplementation, TraitMethodSignature, TraitTable,
};
pub use var::{ConstVar, TypeVar};

#[cfg(test)]
mod tests;
