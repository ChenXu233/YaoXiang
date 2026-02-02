//! RFC-011 类型级比较运算
//!
//! 提供类型级的比较运算支持，用于条件类型和模式匹配。
//!
//! 支持的运算：
//! - Eq: 相等 `A == B`
//! - Neq: 不等 `A != B`
//! - Lt: 小于 `A < B`
//! - Gt: 大于 `A > B`
//! - Lte: 小于等于 `A <= B`
//! - Gte: 大于等于 `A >= B`

use super::TypeLevelValue;
use crate::frontend::core::type_system::MonoType;

/// 比较运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    /// 等于: ==
    Eq,

    /// 不等于: !=
    Neq,

    /// 小于: <
    Lt,

    /// 大于: >
    Gt,

    /// 小于等于: <=
    Lte,

    /// 大于等于: >=
    Gte,
}

impl CmpOp {
    /// 执行比较运算
    pub fn apply(
        self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        match (self, lhs, rhs) {
            // 相等比较
            (CmpOp::Eq, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a == b))
            }
            (CmpOp::Eq, TypeLevelValue::Bool(a), TypeLevelValue::Bool(b)) => {
                Some(TypeLevelValue::Bool(a == b))
            }
            // 不等比较
            (CmpOp::Neq, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a != b))
            }
            (CmpOp::Neq, TypeLevelValue::Bool(a), TypeLevelValue::Bool(b)) => {
                Some(TypeLevelValue::Bool(a != b))
            }
            // 小于比较
            (CmpOp::Lt, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a < b))
            }
            // 大于比较
            (CmpOp::Gt, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a > b))
            }
            // 小于等于比较
            (CmpOp::Lte, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a <= b))
            }
            // 大于等于比较
            (CmpOp::Gte, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a >= b))
            }
            _ => None,
        }
    }

    /// 获取运算符名称
    pub fn name(&self) -> &'static str {
        match self {
            CmpOp::Eq => "Eq",
            CmpOp::Neq => "Neq",
            CmpOp::Lt => "Lt",
            CmpOp::Gt => "Gt",
            CmpOp::Lte => "Lte",
            CmpOp::Gte => "Gte",
        }
    }
}

/// 类型级比较运算器
#[derive(Debug, Clone, Default)]
pub struct TypeComparison;

impl TypeComparison {
    /// 创建新的比较运算器
    pub fn new() -> Self {
        Self
    }

    /// 检查相等
    pub fn eq(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Eq.apply(lhs, rhs)
    }

    /// 检查不等
    pub fn neq(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Neq.apply(lhs, rhs)
    }

    /// 检查小于
    pub fn lt(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Lt.apply(lhs, rhs)
    }

    /// 检查大于
    pub fn gt(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Gt.apply(lhs, rhs)
    }

    /// 检查小于等于
    pub fn lte(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Lte.apply(lhs, rhs)
    }

    /// 检查大于等于
    pub fn gte(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Gte.apply(lhs, rhs)
    }

    /// 执行比较
    pub fn compare(
        &self,
        op: CmpOp,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        op.apply(lhs, rhs)
    }

    /// 类型相等性检查
    pub fn types_equal(
        &self,
        ty1: &MonoType,
        ty2: &MonoType,
    ) -> bool {
        ty1 == ty2
    }
}
