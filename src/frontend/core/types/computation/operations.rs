//! RFC-011 类型级运算
//!
//! 提供类型级算术、比较和逻辑运算。
//!
//! 这些运算用于条件类型和Const泛型：
//! - 算术运算: Add, Sub, Mul, Div, Mod
//! - 比较运算: Eq, Neq, Lt, Gt, Lte, Gte
//! - 逻辑运算: And, Or, Not

use crate::frontend::core::types::base::MonoType;

// ============================================================================
// 公共类型
// ============================================================================

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

// ============================================================================
// 算术运算
// ============================================================================

/// 算术运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
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
    pub fn new() -> Self {
        Self
    }

    pub fn add(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        ArithOp::Add.apply(lhs, rhs)
    }
    pub fn sub(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        ArithOp::Sub.apply(lhs, rhs)
    }
    pub fn mul(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        ArithOp::Mul.apply(lhs, rhs)
    }
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
    pub fn neg(
        &self,
        val: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        match val {
            TypeLevelValue::Int(n) => Some(TypeLevelValue::Int(-*n)),
            _ => None,
        }
    }
    pub fn binary_op(
        &self,
        op: ArithOp,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        op.apply(lhs, rhs)
    }
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

// ============================================================================
// 比较运算
// ============================================================================

/// 比较运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
}

impl CmpOp {
    pub fn apply(
        self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        match (self, lhs, rhs) {
            (CmpOp::Eq, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a == b))
            }
            (CmpOp::Eq, TypeLevelValue::Bool(a), TypeLevelValue::Bool(b)) => {
                Some(TypeLevelValue::Bool(a == b))
            }
            (CmpOp::Neq, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a != b))
            }
            (CmpOp::Neq, TypeLevelValue::Bool(a), TypeLevelValue::Bool(b)) => {
                Some(TypeLevelValue::Bool(a != b))
            }
            (CmpOp::Lt, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a < b))
            }
            (CmpOp::Gt, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a > b))
            }
            (CmpOp::Lte, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a <= b))
            }
            (CmpOp::Gte, TypeLevelValue::Int(a), TypeLevelValue::Int(b)) => {
                Some(TypeLevelValue::Bool(a >= b))
            }
            _ => None,
        }
    }

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
    pub fn new() -> Self {
        Self
    }

    pub fn eq(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Eq.apply(lhs, rhs)
    }
    pub fn neq(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Neq.apply(lhs, rhs)
    }
    pub fn lt(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Lt.apply(lhs, rhs)
    }
    pub fn gt(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Gt.apply(lhs, rhs)
    }
    pub fn lte(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Lte.apply(lhs, rhs)
    }
    pub fn gte(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        CmpOp::Gte.apply(lhs, rhs)
    }
    pub fn compare(
        &self,
        op: CmpOp,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        op.apply(lhs, rhs)
    }
    pub fn types_equal(
        &self,
        ty1: &MonoType,
        ty2: &MonoType,
    ) -> bool {
        ty1 == ty2
    }
}

// ============================================================================
// 逻辑运算
// ============================================================================

/// 逻辑运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicOp {
    And,
    Or,
    Not,
}

impl LogicOp {
    pub fn apply(
        self,
        lhs: Option<&TypeLevelValue>,
        rhs: Option<&TypeLevelValue>,
    ) -> Option<TypeLevelValue> {
        match (self, lhs, rhs) {
            (LogicOp::And, Some(TypeLevelValue::Bool(a)), Some(TypeLevelValue::Bool(b))) => {
                Some(TypeLevelValue::Bool(*a && *b))
            }
            (LogicOp::Or, Some(TypeLevelValue::Bool(a)), Some(TypeLevelValue::Bool(b))) => {
                Some(TypeLevelValue::Bool(*a || *b))
            }
            (LogicOp::Not, Some(TypeLevelValue::Bool(a)), None) => Some(TypeLevelValue::Bool(!*a)),
            _ => None,
        }
    }

    pub fn binary_op(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        LogicOp::And.apply(Some(lhs), Some(rhs))
    }
    pub fn unary_op(
        &self,
        val: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        LogicOp::Not.apply(Some(val), None)
    }

    pub fn name(&self) -> &'static str {
        match self {
            LogicOp::And => "And",
            LogicOp::Or => "Or",
            LogicOp::Not => "Not",
        }
    }
}

/// 类型级逻辑运算器
#[derive(Debug, Clone, Default)]
pub struct TypeLogic {
    short_circuit: bool,
}

impl TypeLogic {
    pub fn new() -> Self {
        Self {
            short_circuit: true,
        }
    }
    pub fn with_short_circuit(
        mut self,
        enabled: bool,
    ) -> Self {
        self.short_circuit = enabled;
        self
    }

    pub fn and(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        if self.short_circuit {
            if let TypeLevelValue::Bool(false) = lhs {
                return Some(TypeLevelValue::Bool(false));
            }
        }
        LogicOp::And.binary_op(lhs, rhs)
    }
    pub fn or(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        if self.short_circuit {
            if let TypeLevelValue::Bool(true) = lhs {
                return Some(TypeLevelValue::Bool(true));
            }
        }
        LogicOp::Or.binary_op(lhs, rhs)
    }
    pub fn not(
        &self,
        val: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        LogicOp::Not.unary_op(val)
    }
    pub fn op(
        &self,
        op: LogicOp,
        lhs: &TypeLevelValue,
        rhs: Option<&TypeLevelValue>,
    ) -> Option<TypeLevelValue> {
        match op {
            LogicOp::And => rhs.and_then(|r| self.and(lhs, r)),
            LogicOp::Or => rhs.and_then(|r| self.or(lhs, r)),
            LogicOp::Not => self.not(lhs),
        }
    }
}

/// 布尔类型构造器
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BoolType {
    False,
    True,
}

impl BoolType {
    pub fn from_value(val: &TypeLevelValue) -> Option<Self> {
        match val {
            TypeLevelValue::Bool(true) => Some(BoolType::True),
            TypeLevelValue::Bool(false) => Some(BoolType::False),
            _ => None,
        }
    }
    pub fn to_value(&self) -> TypeLevelValue {
        TypeLevelValue::Bool(match self {
            BoolType::False => false,
            BoolType::True => true,
        })
    }
}
