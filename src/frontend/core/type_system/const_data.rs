//! Const值和表达式定义
//!
//! 实现编译期常量：
//! - ConstValue: 编译期常量值
//! - ConstExpr: 编译期可求值表达式
//! - ConstKind: Const泛型变量的类型约束
//! - ConstVarDef: Const泛型变量定义

use std::fmt;
use std::hash::Hash;

/// Const值（编译期常量）
#[derive(Debug, Clone)]
pub enum ConstValue {
    /// 整数常量
    Int(i128),
    /// 布尔常量
    Bool(bool),
    /// 浮点常量
    Float(f32),
}

impl ConstValue {
    /// 检查是否是数值类型
    pub fn is_numeric(&self) -> bool {
        matches!(self, ConstValue::Int(_) | ConstValue::Float(_))
    }

    /// 获取Const值的类型
    pub fn kind(&self) -> ConstKind {
        match self {
            ConstValue::Int(_) => ConstKind::Int(None),
            ConstValue::Bool(_) => ConstKind::Bool,
            ConstValue::Float(_) => ConstKind::Float(None),
        }
    }

    /// 从字面量名称解析 ConstValue
    /// 例如: "5" -> ConstValue::Int(5), "3.14" -> ConstValue::Float(3.14), "true" -> ConstValue::Bool(true)
    pub fn from_literal_name(name: &str) -> Option<Self> {
        // Try to parse as integer first (to prioritize "5" over "5.0")
        if let Ok(n) = name.parse::<i128>() {
            return Some(ConstValue::Int(n));
        }
        // Try to parse as float (to handle cases like "3.14")
        if let Ok(f) = name.parse::<f64>() {
            return Some(ConstValue::Float(f as f32));
        }
        // Try to parse as boolean
        match name {
            "true" => Some(ConstValue::Bool(true)),
            "false" => Some(ConstValue::Bool(false)),
            _ => None,
        }
    }

    /// 检查名称是否是有效的字面量
    pub fn is_valid_literal_name(name: &str) -> bool {
        // 整数优先于浮点数（5.0 应该被解析为浮点数，但如果输入是 "5" 则解析为整数）
        name.parse::<i128>().is_ok()
            || name.parse::<f64>().is_ok()
            || name == "true"
            || name == "false"
    }
}

impl PartialEq for ConstValue {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        match (self, other) {
            (ConstValue::Int(a), ConstValue::Int(b)) => a == b,
            (ConstValue::Bool(a), ConstValue::Bool(b)) => a == b,
            (ConstValue::Float(a), ConstValue::Float(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }
}

impl Eq for ConstValue {}

impl Hash for ConstValue {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        match self {
            ConstValue::Int(n) => {
                // 使用整数哈希
                n.hash(state);
            }
            ConstValue::Bool(b) => {
                // 使用布尔哈希
                b.hash(state);
            }
            ConstValue::Float(f) => {
                // 使用浮点数的位模式哈希
                f.to_bits().hash(state);
            }
        }
    }
}

impl fmt::Display for ConstValue {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            ConstValue::Int(n) => write!(f, "{}", n),
            ConstValue::Bool(b) => write!(f, "{}", b),
            ConstValue::Float(v) => write!(f, "{}", v),
        }
    }
}

/// Const表达式（编译期可求值的表达式）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstExpr {
    /// 字面量常量
    Lit(ConstValue),
    /// Const变量引用
    Var(super::var::ConstVar),
    /// 二元运算
    BinOp {
        op: BinOp,
        left: Box<ConstExpr>,
        right: Box<ConstExpr>,
    },
    /// 一元运算
    UnOp { op: UnOp, expr: Box<ConstExpr> },
    /// 函数调用（仅限const函数）
    Call { func: String, args: Vec<ConstExpr> },
    /// 条件表达式
    If {
        condition: Box<ConstExpr>,
        then_branch: Box<ConstExpr>,
        else_branch: Box<ConstExpr>,
    },
    /// 范围表达式
    Range {
        start: Box<ConstExpr>,
        end: Box<ConstExpr>,
    },
}

/// 二元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    /// 算术运算
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    /// 比较运算
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// 逻辑运算
    And,
    Or,
    /// 位运算
    BitAnd,
    BitOr,
    BitXor,
    /// 左移/右移
    Shl,
    Shr,
}

impl BinOp {
    /// 检查是否是算术运算
    pub fn is_arithmetic(&self) -> bool {
        matches!(
            self,
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
        )
    }

    /// 检查是否是比较运算
    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
        )
    }

    /// 检查是否是逻辑运算
    pub fn is_logical(&self) -> bool {
        matches!(self, BinOp::And | BinOp::Or)
    }

    /// 检查是否是位运算
    pub fn is_bitwise(&self) -> bool {
        matches!(
            self,
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr
        )
    }
}

/// 一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// 正号
    Pos,
    /// 负号
    Neg,
    /// 逻辑非
    Not,
    /// 位反
    BitNot,
}

impl UnOp {
    /// 检查是否是正号或负号
    pub fn is_arithmetic(&self) -> bool {
        matches!(self, UnOp::Pos | UnOp::Neg)
    }

    /// 检查是否是逻辑非
    pub fn is_logical(&self) -> bool {
        matches!(self, UnOp::Not)
    }

    /// 检查是否是位运算
    pub fn is_bitwise(&self) -> bool {
        matches!(self, UnOp::BitNot)
    }
}

/// Const泛型变量的类型约束
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstKind {
    /// 整数类型（可带位宽约束）
    Int(Option<usize>),
    /// 布尔类型
    Bool,
    /// 浮点类型（可带位宽约束）
    Float(Option<usize>),
}

impl ConstKind {
    /// 检查是否匹配给定的ConstValue
    pub fn matches(
        &self,
        value: &ConstValue,
    ) -> bool {
        matches!(
            (self, value),
            (ConstKind::Int(_), ConstValue::Int(_))
                | (ConstKind::Bool, ConstValue::Bool(_))
                | (ConstKind::Float(_), ConstValue::Float(_))
        )
    }

    /// 获取类型的字符串表示
    pub fn type_name(&self) -> &'static str {
        match self {
            ConstKind::Int(_) => "Int",
            ConstKind::Bool => "Bool",
            ConstKind::Float(_) => "Float",
        }
    }
}

/// Const泛型变量（包含名称、类型约束和索引）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstVarDef {
    /// 变量名称
    pub name: String,
    /// 类型约束
    pub kind: ConstKind,
    /// 变量索引
    pub index: usize,
}

impl ConstVarDef {
    /// 创建新的Const变量定义
    pub fn new(
        name: String,
        kind: ConstKind,
        index: usize,
    ) -> Self {
        ConstVarDef { name, kind, index }
    }
}

impl fmt::Display for ConstVarDef {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
