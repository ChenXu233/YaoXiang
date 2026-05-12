//! 类型系统子模块
//!
//! 将类型系统拆分为多个子模块以提高可维护性：
//! - var: 类型变量定义
//! - const: Const值和表达式
//! - mono: 单态类型定义
//! - substitute: 统一的类型替换算法
//! - constraint: 类型约束
//! - solver: 类型约束求解器
//! - error: 类型错误定义

pub mod const_data;
pub mod constraint;
pub mod error;
pub mod mono;
pub mod solver;
pub mod substitute;
pub mod var;

// 重新导出主要类型
pub use var::{TypeVar, ConstVar};
pub use const_data::{ConstValue, ConstExpr, ConstKind, ConstVarDef, BinOp, UnOp};
pub use mono::{
    TypeBinding, MonoType, StructType, EnumType, PolyType, UniverseLevel,
    get_ast_type_universe_level, calculate_meta_type_level,
};
pub use substitute::{Substitution, Substituter};
pub use constraint::{TypeConstraint, SendSyncConstraint, SendSyncSolver};
pub use solver::TypeConstraintSolver;
pub use error::{TypeMismatch, TypeConstraintError, ConstEvalError};
