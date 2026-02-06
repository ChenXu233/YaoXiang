//! RFC-011 类型族实现
//!
//! 提供类型级基本类型族：
//! - Bool: `True`, `False` 布尔类型
//! - Nat: `Zero`, `Succ[N]` 自然数类型
//!
//! 示例：
//! ```yaoxiang
//! // Bool 类型族
//! type BoolTrue = True        // 真类型
//! type BoolFalse = False     // 假类型
//!
//! // Nat 类型族
//! type NatZero = Zero         // 零
//! type NatOne = Succ[Zero]   // 一 (Zero 的后继)
//! type NatTwo = Succ[Succ[Zero]]  // 二
//! ```

use crate::frontend::core::type_system::MonoType;
use super::TypeLevelResult;

/// Bool 类型族的变体
///
/// 用于表示编译期的布尔值：
/// - `True`: 真
/// - `False`: 假
///
/// # 示例
/// ```yaoxiang
/// type MyTrue = True
/// type MyFalse = False
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Bool {
    /// 真类型
    True,

    /// 假类型
    False,
}

impl Bool {
    /// 获取 Bool 类型的布尔值
    pub fn as_bool(&self) -> bool {
        match self {
            Bool::True => true,
            Bool::False => false,
        }
    }

    /// 从布尔值创建 Bool 类型
    pub fn from_bool(b: bool) -> Self {
        if b {
            Bool::True
        } else {
            Bool::False
        }
    }

    /// 评估 Bool 类型
    pub fn eval(&self) -> TypeLevelResult<Bool> {
        TypeLevelResult::Normalized(self.clone())
    }

    /// 检查是否为 True
    pub fn is_true(&self) -> bool {
        matches!(self, Bool::True)
    }

    /// 检查是否为 False
    pub fn is_false(&self) -> bool {
        matches!(self, Bool::False)
    }
}

/// Nat 类型族的变体
///
/// 用于表示编译期的自然数：
/// - `Zero`: 零
/// - `Succ[N]`: N 的后继
///
/// # 示例
/// ```yaoxiang
/// type Zero = Zero
/// type One = Succ[Zero]
/// type Two = Succ[Succ[Zero]]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Nat {
    /// 零
    Zero,

    /// 后继: Succ[N] = N + 1
    Succ(Box<Nat>),
}

impl Nat {
    /// 创建 Zero
    pub fn zero() -> Self {
        Nat::Zero
    }

    /// 创建后继
    pub fn succ(n: Nat) -> Self {
        Nat::Succ(Box::new(n))
    }

    /// 评估 Nat 类型
    pub fn eval(&self) -> TypeLevelResult<Nat> {
        TypeLevelResult::Normalized(self.clone())
    }

    /// 计算自然数的值（用于比较）
    pub fn to_usize(&self) -> usize {
        match self {
            Nat::Zero => 0,
            Nat::Succ(n) => n.to_usize() + 1,
        }
    }

    /// 尝试从 usize 创建 Nat
    pub fn from_usize(n: usize) -> Self {
        if n == 0 {
            Nat::Zero
        } else {
            Nat::Succ(Box::new(Nat::from_usize(n - 1)))
        }
    }

    /// 检查是否为 Zero
    pub fn is_zero(&self) -> bool {
        matches!(self, Nat::Zero)
    }
}

/// 条件类型 - IsTrue[C]
///
/// 检查类型 C 是否为 True
///
/// # 示例
/// ```yaoxiang
/// type Check1 = IsTrue[True]   // => True
/// type Check2 = IsTrue[False]  // => False
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IsTrue {
    /// 被检查的条件
    pub condition: Box<Bool>,
}

impl IsTrue {
    /// 创建新的 IsTrue 条件
    pub fn new(condition: Bool) -> Self {
        Self {
            condition: Box::new(condition),
        }
    }

    /// 评估 IsTrue 条件
    pub fn eval(&self) -> TypeLevelResult<Bool> {
        TypeLevelResult::Normalized(Bool::from_bool(self.condition.as_bool()))
    }
}

/// 条件类型 - IsFalse[C]
///
/// 检查类型 C 是否为 False
///
/// # 示例
/// ```yaoxiang
/// type Check1 = IsFalse[True]   // => False
/// type Check2 = IsFalse[False]  // => True
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IsFalse {
    /// 被检查的条件
    pub condition: Box<Bool>,
}

impl IsFalse {
    /// 创建新的 IsFalse 条件
    pub fn new(condition: Bool) -> Self {
        Self {
            condition: Box::new(condition),
        }
    }

    /// 评估 IsFalse 条件
    pub fn eval(&self) -> TypeLevelResult<Bool> {
        TypeLevelResult::Normalized(Bool::from_bool(!self.condition.as_bool()))
    }
}

/// 条件类型 - IsZero[N]
///
/// 检查自然数 N 是否为 Zero
///
/// # 示例
/// ```yaoxiang
/// type Check1 = IsZero[Zero]        // => True
/// type Check2 = IsZero[Succ[Zero]]  // => False
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IsZero {
    /// 被检查的自然数
    pub nat: Box<Nat>,
}

impl IsZero {
    /// 创建新的 IsZero 条件
    pub fn new(nat: Nat) -> Self {
        Self { nat: Box::new(nat) }
    }

    /// 评估 IsZero 条件
    pub fn eval(&self) -> TypeLevelResult<Bool> {
        TypeLevelResult::Normalized(Bool::from_bool(self.nat.is_zero()))
    }
}

/// 条件类型 - IsSucc[N]
///
/// 检查自然数 N 是否为 Succ 变体（即非 Zero）
///
/// # 示例
/// ```yaoxiang
/// type Check1 = IsSucc[Zero]        // => False
/// type Check2 = IsSucc[Succ[Zero]]  // => True
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IsSucc {
    /// 被检查的自然数
    pub nat: Box<Nat>,
}

impl IsSucc {
    /// 创建新的 IsSucc 条件
    pub fn new(nat: Nat) -> Self {
        Self { nat: Box::new(nat) }
    }

    /// 评估 IsSucc 条件
    pub fn eval(&self) -> TypeLevelResult<Bool> {
        TypeLevelResult::Normalized(Bool::from_bool(!self.nat.is_zero()))
    }
}

/// 类型族 trait - 用于统一处理不同类型族的操作
pub trait TypeFamily {
    /// 关联的类型
    type Value;

    /// 评估类型
    fn eval(&self) -> TypeLevelResult<Self::Value>;
}

impl TypeFamily for Bool {
    type Value = Bool;

    fn eval(&self) -> TypeLevelResult<Self::Value> {
        // Bool 类型已经是范式，直接返回
        TypeLevelResult::Normalized(self.clone())
    }
}

impl TypeFamily for Nat {
    type Value = Nat;

    fn eval(&self) -> TypeLevelResult<Self::Value> {
        // Nat 类型已经是范式，直接返回
        TypeLevelResult::Normalized(self.clone())
    }
}

/// 类型族值的扩展 trait
pub trait TypeFamilyOps {
    /// 转换为 MonoType
    fn to_mono_type(&self) -> MonoType;
}

impl TypeFamilyOps for Bool {
    fn to_mono_type(&self) -> MonoType {
        MonoType::TypeRef(if self.is_true() {
            "True".to_string()
        } else {
            "False".to_string()
        })
    }
}

impl TypeFamilyOps for Nat {
    fn to_mono_type(&self) -> MonoType {
        match self {
            Nat::Zero => MonoType::TypeRef("Zero".to_string()),
            Nat::Succ(n) => MonoType::TypeRef(format!("Succ[{}]", n.to_usize())),
        }
    }
}

/// Bool 类型族的辅助函数
pub mod bool_family {
    use super::*;

    /// 创建 True
    pub fn true_() -> Bool {
        Bool::True
    }

    /// 创建 False
    pub fn false_() -> Bool {
        Bool::False
    }

    /// 创建 IsTrue 条件
    pub fn is_true(condition: Bool) -> IsTrue {
        IsTrue::new(condition)
    }

    /// 创建 IsFalse 条件
    pub fn is_false(condition: Bool) -> IsFalse {
        IsFalse::new(condition)
    }
}

/// Nat 类型族的辅助函数
pub mod nat_family {
    use super::*;

    /// 创建 Zero
    pub fn zero() -> Nat {
        Nat::Zero
    }

    /// 创建后继
    pub fn succ(n: Nat) -> Nat {
        Nat::Succ(Box::new(n))
    }

    /// 创建 IsZero 条件
    pub fn is_zero(nat: Nat) -> IsZero {
        IsZero::new(nat)
    }

    /// 创建 IsSucc 条件
    pub fn is_succ(nat: Nat) -> IsSucc {
        IsSucc::new(nat)
    }

    /// 从 usize 转换为 Nat
    pub fn from_usize(n: usize) -> Nat {
        Nat::from_usize(n)
    }

    /// 计算自然数的值
    pub fn to_usize(nat: &Nat) -> usize {
        nat.to_usize()
    }
}
