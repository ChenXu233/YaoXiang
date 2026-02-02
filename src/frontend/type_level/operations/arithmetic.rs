//! RFC-011 类型级算术运算
//!
//! 提供类型级的算术运算支持，用于Const泛型和条件类型。
//!
//! 支持的运算：
//! - Add: 加法 `A + B`
//! - Sub: 减法 `A - B`
//! - Mul: 乘法 `A * B`
//! - Div: 除法 `A / B`
//! - Mod: 取模 `A % B`

use super::TypeLevelValue;

/// 算术运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArithOp {
    /// 加法: +
    Add,

    /// 减法: -
    Sub,

    /// 乘法: *
    Mul,

    /// 除法: /
    Div,

    /// 取模: %
    Mod,
}

impl ArithOp {
    /// 执行二元运算
    pub fn apply(
        self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        match (self, lhs, rhs) {
            (ArithOp::Add, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Int(a.saturating_add(*b)))
            }
            (ArithOp::Sub, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Int(a.saturating_sub(*b)))
            }
            (ArithOp::Mul, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Int(a.saturating_mul(*b)))
            }
            (ArithOp::Div, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                if *b != 0 {
                    Some(TypeLevelValue::Int(a.saturating_div(*b)))
                } else {
                    None
                }
            }
            (ArithOp::Mod, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                if *b != 0 {
                    Some(TypeLevelValue::Int(a % b))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 获取运算符名称
    pub fn name(&self) -> &'static str {
        match self {
            ArithOp::Add => "Add",
            ArithOp::Sub => "Sub",
            ArithOp::Mul => "Mul",
            ArithOp::Div => "Div",
            ArithOp::Mod => "Mod",
        }
    }
}

/// 类型级算术运算器
#[derive(Debug, Clone, Default)]
pub struct TypeArithmetic;

impl TypeArithmetic {
    /// 创建新的算术运算器
    pub fn new() -> Self {
        Self
    }

    /// 执行加法
    pub fn add(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        ArithOp::Add.apply(lhs, rhs)
    }

    /// 执行减法
    pub fn sub(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        ArithOp::Sub.apply(lhs, rhs)
    }

    /// 执行乘法
    pub fn mul(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        ArithOp::Mul.apply(lhs, rhs)
    }

    /// 执行除法
    pub fn div(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        if matches!(rhs, TypeLevelValue::Int(0)) {
            return None;
        }
        ArithOp::Div.apply(lhs, rhs)
    }

    /// 执行取模
    pub fn rem(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        if matches!(rhs, TypeLevelValue::Int(0)) {
            return None;
        }
        ArithOp::Mod.apply(lhs, rhs)
    }

    /// 执行一元运算（取负）
    pub fn neg(
        &self,
        val: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        match val {
            TypeLevelValue::Int(n) => Some(TypeLevelValue::Int(-*n)),
            _ => None,
        }
    }

    /// 执行二元运算
    pub fn binary_op(
        &self,
        op: ArithOp,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        op.apply(lhs, rhs)
    }

    /// 执行一元运算
    pub fn unary_op(
        &self,
        op: ArithOp,
        val: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        match op {
            ArithOp::Sub => self.neg(val),
            _ => None,
        }
    }
}
