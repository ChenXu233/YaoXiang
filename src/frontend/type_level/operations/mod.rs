//! RFC-011 类型级运算
//!
//! 提供类型级算术、比较和逻辑运算。
//!
//! 这些运算用于条件类型和Const泛型：
//! - 算术运算: Add, Sub, Mul, Div, Mod
//! - 比较运算: Eq, Neq, Lt, Gt, Lte, Gte
//! - 逻辑运算: And, Or, Not

use crate::frontend::core::type_system::MonoType;

pub mod arithmetic;
pub mod comparison;
pub mod logic;

// 重新导出主要类型
pub use arithmetic::{TypeArithmetic, ArithOp};
pub use comparison::{TypeComparison, CmpOp};
pub use logic::{TypeLogic, LogicOp};

/// 类型级运算的结果类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeLevelValue {
    /// 布尔值
    Bool(bool),

    /// 整数值
    Int(i128),

    /// 类型
    Type(MonoType),
}

/// 类型级运算 trait
pub trait TypeLevelOps {
    /// 执行运算
    fn op(
        &self,
        lhs: &TypeLevelValue,
        rhs: Option<&TypeLevelValue>,
    ) -> Option<TypeLevelValue>;
}

/// 预定义的类型级常量
pub mod constants {
    use super::*;

    /// True
    pub const TRUE: TypeLevelValue = TypeLevelValue::Bool(true);

    /// False
    pub const FALSE: TypeLevelValue = TypeLevelValue::Bool(false);

    /// Zero
    pub const ZERO: TypeLevelValue = TypeLevelValue::Int(0);

    /// One
    pub const ONE: TypeLevelValue = TypeLevelValue::Int(1);
}
