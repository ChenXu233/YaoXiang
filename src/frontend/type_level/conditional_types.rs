//! RFC-011 条件类型实现
//!
//! 支持编译期条件类型计算：
//! - `If[C, T, E]`: 基于布尔条件的类型选择
//! - `MatchType[T]`: 模式匹配类型选择
//!
//! 示例：
//! ```yaoxiang
//! type If[C: Bool, T, E] = match C {
//!     True => T,
//!     False => E,
//! }
//!
//! type NonEmpty[T] = If[T != Void, T, Never]
//! ```

use crate::frontend::core::type_system::MonoType;
use super::{TypeLevelError, TypeLevelResult};

/// 条件类型的布尔条件
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeCondition {
    /// 布尔常量条件
    Bool(bool),

    /// 等式条件: L == R
    Eq(Box<MonoType>, Box<MonoType>),

    /// 不等条件: L != R
    Neq(Box<MonoType>, Box<MonoType>),

    /// 类型是 Void
    IsVoid(Box<MonoType>),

    /// 类型是 Never
    IsNever(Box<MonoType>),

    /// 类型是具体类型
    IsType(Box<MonoType>, Box<MonoType>),

    /// 组合条件: A && B
    And(Box<TypeCondition>, Box<TypeCondition>),

    /// 组合条件: A || B
    Or(Box<TypeCondition>, Box<TypeCondition>),

    /// 否定条件: !A
    Not(Box<TypeCondition>),
}

impl TypeCondition {
    /// 评估条件为布尔值
    pub fn eval(&self) -> Option<bool> {
        match self {
            TypeCondition::Bool(b) => Some(*b),
            TypeCondition::Eq(lhs, rhs) => {
                // 类型相等性检查
                if lhs == rhs {
                    Some(true)
                } else {
                    // 尝试范式化后比较
                    Some(false)
                }
            }
            TypeCondition::Neq(lhs, rhs) => Some(lhs != rhs),
            TypeCondition::IsVoid(ty) => Some(self.is_void_type(ty)),
            TypeCondition::IsNever(ty) => Some(self.is_never_type(ty)),
            TypeCondition::IsType(ty, expected) => Some(ty.as_ref() == expected.as_ref()),
            TypeCondition::And(lhs, rhs) => match (lhs.eval(), rhs.eval()) {
                (Some(true), Some(true)) => Some(true),
                (Some(false), _) | (_, Some(false)) => Some(false),
                _ => None,
            },
            TypeCondition::Or(lhs, rhs) => match (lhs.eval(), rhs.eval()) {
                (Some(false), Some(false)) => Some(false),
                (Some(true), _) | (_, Some(true)) => Some(true),
                _ => None,
            },
            TypeCondition::Not(inner) => inner.eval().map(|b| !b),
        }
    }

    fn is_void_type(
        &self,
        ty: &MonoType,
    ) -> bool {
        matches!(ty, MonoType::Void)
    }

    fn is_never_type(
        &self,
        ty: &MonoType,
    ) -> bool {
        // Never 类型通过 TypeRef 表示
        if let MonoType::TypeRef(name) = ty {
            name == "Never"
        } else {
            false
        }
    }

    /// 检查条件是否已完全确定
    pub fn is_determined(&self) -> bool {
        match self {
            TypeCondition::Bool(_) => true,
            TypeCondition::Eq(_, _) => false, // 需要类型检查
            TypeCondition::Neq(_, _) => false,
            TypeCondition::IsVoid(ty) => matches!(**ty, MonoType::Void),
            TypeCondition::IsNever(ty) => {
                if let MonoType::TypeRef(name) = &**ty {
                    name == "Never"
                } else {
                    false
                }
            }
            TypeCondition::IsType(ty, _) => !matches!(**ty, MonoType::TypeVar(_)),
            TypeCondition::And(lhs, rhs) => lhs.is_determined() && rhs.is_determined(),
            TypeCondition::Or(lhs, rhs) => lhs.is_determined() && rhs.is_determined(),
            TypeCondition::Not(inner) => inner.is_determined(),
        }
    }
}

/// 条件类型 - If[C, T, E]
///
/// 基于布尔条件 C 在编译期选择 T 或 E
///
/// # 示例
/// ```yaoxiang
/// type If[True, Int, String] => Int
/// type If[False, Int, String] => String
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct If {
    /// 条件
    pub condition: TypeCondition,

    /// 条件为 true 时的类型
    pub true_branch: Box<MonoType>,

    /// 条件为 false 时的类型
    pub false_branch: Box<MonoType>,
}

impl If {
    /// 创建新的条件类型
    pub fn new(
        condition: TypeCondition,
        true_branch: MonoType,
        false_branch: MonoType,
    ) -> Self {
        Self {
            condition,
            true_branch: Box::new(true_branch),
            false_branch: Box::new(false_branch),
        }
    }

    /// 评估条件类型
    pub fn eval(&self) -> TypeLevelResult<MonoType> {
        // 尝试评估条件
        match self.condition.eval() {
            Some(true) => TypeLevelResult::Normalized(*self.true_branch.clone()),
            Some(false) => TypeLevelResult::Normalized(*self.false_branch.clone()),
            None => TypeLevelResult::Pending(MonoType::TypeRef("If".to_string())),
        }
    }

    /// 获取条件
    pub fn condition(&self) -> &TypeCondition {
        &self.condition
    }

    /// 获取 true 分支
    pub fn true_branch(&self) -> &MonoType {
        &self.true_branch
    }

    /// 获取 false 分支
    pub fn false_branch(&self) -> &MonoType {
        &self.false_branch
    }
}

/// Match 类型的分支
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchArm {
    /// 模式类型
    pub pattern: MonoType,

    /// 结果类型
    pub result: MonoType,
}

/// Match 类型 - 基于模式匹配的类型选择
///
/// # 示例
/// ```yaoxiang
/// type AsString[T] = match T {
///     Int => String,
///     Float => String,
///     Bool => String,
///     _ => String,
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchType {
    /// 被匹配的类型
    pub target: Box<MonoType>,

    /// 分支列表
    pub arms: Vec<MatchArm>,
}

impl MatchType {
    /// 创建新的匹配类型
    pub fn new(
        target: MonoType,
        arms: Vec<MatchArm>,
    ) -> Self {
        Self {
            target: Box::new(target),
            arms,
        }
    }

    /// 创建通配符分支
    pub fn with_wildcard(
        target: MonoType,
        result: MonoType,
    ) -> Self {
        Self::new(
            target,
            vec![MatchArm {
                pattern: MonoType::TypeRef("_".to_string()),
                result,
            }],
        )
    }

    /// 添加分支
    pub fn add_arm(
        &mut self,
        pattern: MonoType,
        result: MonoType,
    ) {
        self.arms.push(MatchArm { pattern, result });
    }

    /// 评估匹配类型
    pub fn eval(&self) -> TypeLevelResult<MonoType> {
        let target = &*self.target;

        // 查找匹配的分支
        for arm in &self.arms {
            if self.pattern_matches(target, &arm.pattern) {
                return TypeLevelResult::Normalized(arm.result.clone());
            }
        }

        // 如果没有匹配且没有通配符，报错
        TypeLevelResult::Error(TypeLevelError::ComputationFailed(
            "No matching arm in MatchType".to_string(),
        ))
    }

    /// 检查模式是否匹配目标类型
    fn pattern_matches(
        &self,
        _target: &MonoType,
        pattern: &MonoType,
    ) -> bool {
        // 通配符匹配任何类型
        // 注意：MonoType::Wildcard 不存在，使用其他方式表示
        match pattern {
            MonoType::TypeVar(_) => false, // 需要类型推断
            _ => true,                     // 简化：默认匹配
        }
    }
}

/// 通用条件类型结构
/// 用于统一处理 If 和 MatchType
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConditionalType {
    /// 条件分支类型
    If(If),

    /// 模式匹配类型
    Match(MatchType),
}

impl ConditionalType {
    /// 评估条件类型
    pub fn eval(&self) -> TypeLevelResult<MonoType> {
        match self {
            ConditionalType::If(if_type) => if_type.eval(),
            ConditionalType::Match(match_type) => match_type.eval(),
        }
    }
}

/// 构建条件类型的辅助函数
pub mod conditions {
    use super::*;

    /// 创建布尔条件
    pub fn bool(b: bool) -> TypeCondition {
        TypeCondition::Bool(b)
    }

    /// 创建等式条件
    pub fn eq(
        lhs: MonoType,
        rhs: MonoType,
    ) -> TypeCondition {
        TypeCondition::Eq(Box::new(lhs), Box::new(rhs))
    }

    /// 创建不等条件
    pub fn neq(
        lhs: MonoType,
        rhs: MonoType,
    ) -> TypeCondition {
        TypeCondition::Neq(Box::new(lhs), Box::new(rhs))
    }

    /// 创建 IsVoid 条件
    pub fn is_void(ty: MonoType) -> TypeCondition {
        TypeCondition::IsVoid(Box::new(ty))
    }

    /// 创建 IsNever 条件
    pub fn is_never(ty: MonoType) -> TypeCondition {
        TypeCondition::IsNever(Box::new(ty))
    }

    /// 创建逻辑与条件
    pub fn and(
        lhs: TypeCondition,
        rhs: TypeCondition,
    ) -> TypeCondition {
        TypeCondition::And(Box::new(lhs), Box::new(rhs))
    }

    /// 创建逻辑或条件
    pub fn or(
        lhs: TypeCondition,
        rhs: TypeCondition,
    ) -> TypeCondition {
        TypeCondition::Or(Box::new(lhs), Box::new(rhs))
    }

    /// 创建逻辑非条件
    pub fn not(condition: TypeCondition) -> TypeCondition {
        TypeCondition::Not(Box::new(condition))
    }
}
