//! RFC-011 类型级逻辑运算
//!
//! 提供类型级的逻辑运算支持，用于条件类型的组合。
//!
//! 支持的运算：
//! - And: 逻辑与 `A && B`
//! - Or: 逻辑或 `A || B`
//! - Not: 逻辑非 `!A`
//!
//! # 示例
//! ```yaoxiang
//! type And[A: Bool, B: Bool] = match (A, B) {
//!     (True, True) => True,
//!     _ => False,
//! }
//!
//! type Or[A: Bool, B: Bool] = match (A, B) {
//!     (False, False) => False,
//!     _ => True,
//! }
//!
//! type Not[A: Bool] = match A {
//!     True => False,
//!     False => True,
//! }
//!
//! # 组合使用
//! type ComplexCondition[A, B] = And[A, Or[B, Not[A]]]
//! ```

use super::TypeLevelValue;

/// 逻辑运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicOp {
    /// 逻辑与: &&
    And,

    /// 逻辑或: ||
    Or,

    /// 逻辑非: !
    Not,
}

impl LogicOp {
    /// 执行逻辑运算
    pub fn apply(
        self,
        lhs: Option<&TypeLevelValue>,
        rhs: Option<&TypeLevelValue>,
    ) -> Option<TypeLevelValue> {
        match (self, lhs, rhs) {
            // 二元运算
            (LogicOp::And, Some(TypeLevelValue::Bool(a)), Some(TypeLevelValue::Bool(b))) => {
                Some(TypeLevelValue::Bool(*a && *b))
            }
            (LogicOp::Or, Some(TypeLevelValue::Bool(a)), Some(TypeLevelValue::Bool(b))) => {
                Some(TypeLevelValue::Bool(*a || *b))
            }

            // 一元运算
            (LogicOp::Not, Some(TypeLevelValue::Bool(a)), None) => Some(TypeLevelValue::Bool(!*a)),

            _ => None,
        }
    }

    /// 执行二元逻辑运算
    pub fn binary_op(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        LogicOp::And.apply(Some(lhs), Some(rhs))
    }

    /// 执行一元逻辑运算
    pub fn unary_op(
        &self,
        val: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        LogicOp::Not.apply(Some(val), None)
    }

    /// 获取运算符名称
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
    /// 短路求值
    short_circuit: bool,
}

impl TypeLogic {
    /// 创建新的逻辑运算器
    pub fn new() -> Self {
        Self {
            short_circuit: true,
        }
    }

    /// 启用/禁用短路求值
    pub fn with_short_circuit(
        mut self,
        enabled: bool,
    ) -> Self {
        self.short_circuit = enabled;
        self
    }

    /// 逻辑与
    pub fn and(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        if self.short_circuit {
            // 短路求值：如果左边为 false，直接返回 false
            if let TypeLevelValue::Bool(false) = lhs {
                return Some(TypeLevelValue::Bool(false));
            }
        }
        LogicOp::And.binary_op(lhs, rhs)
    }

    /// 逻辑或
    pub fn or(
        &self,
        lhs: &TypeLevelValue,
        rhs: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        if self.short_circuit {
            // 短路求值：如果左边为 true，直接返回 true
            if let TypeLevelValue::Bool(true) = lhs {
                return Some(TypeLevelValue::Bool(true));
            }
        }
        LogicOp::Or.binary_op(lhs, rhs)
    }

    /// 逻辑非
    pub fn not(
        &self,
        val: &TypeLevelValue,
    ) -> Option<TypeLevelValue> {
        LogicOp::Not.unary_op(val)
    }

    /// 执行逻辑运算
    pub fn op(
        &self,
        op: LogicOp,
        lhs: &TypeLevelValue,
        rhs: Option<&TypeLevelValue>,
    ) -> Option<TypeLevelValue> {
        match op {
            LogicOp::And => {
                if let Some(r) = rhs {
                    self.and(lhs, r)
                } else {
                    None
                }
            }
            LogicOp::Or => {
                if let Some(r) = rhs {
                    self.or(lhs, r)
                } else {
                    None
                }
            }
            LogicOp::Not => {
                if rhs.is_none() {
                    self.not(lhs)
                } else {
                    None
                }
            }
        }
    }
}

/// 布尔类型构造器
///
/// 用于实现布尔类型级的逻辑运算
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BoolType {
    /// False
    False,

    /// True
    True,
}

impl BoolType {
    /// 从 TypeLevelValue 创建
    pub fn from_value(val: &TypeLevelValue) -> Option<Self> {
        match val {
            TypeLevelValue::Bool(true) => Some(BoolType::True),
            TypeLevelValue::Bool(false) => Some(BoolType::False),
            _ => None,
        }
    }

    /// 转换为 TypeLevelValue
    pub fn to_value(&self) -> TypeLevelValue {
        TypeLevelValue::Bool(match self {
            BoolType::False => false,
            BoolType::True => true,
        })
    }
}
