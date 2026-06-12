//! RFC-011 条件类型与模式匹配实现
//!
//! 支持编译期条件类型计算：
//! - `If[C, T, E]`: 基于布尔条件的类型选择
//! - `MatchType[T]`: 模式匹配类型选择
//! - `MatchPattern`: 匹配模式（字面量、构造器、通配符）
//! - `PatternMatcher`: 模式匹配引擎
//!
//! 示例：
//! ```yaoxiang
//! type If[C: Bool, T, E] = match C {
//!     True => T,
//!     False => E,
//! }
//!
//! type NonEmpty[T] = If[T != Void, T, Never]
//!
//! type Add[A: Nat, B: Nat] = match (A, B) {
//!     (Zero, B) => B,
//!     (Succ(A'), B) => Succ(Add(A', B)),
//! }
//! ```

use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::eval::{TypeLevelError, TypeLevelResult};

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

/// Match 类型的分支（条件类型版本，使用 MonoType 作为 pattern）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchArm {
    /// 模式类型
    pub pattern: MonoType,

    /// 结果类型
    pub result: MonoType,
}

/// Match 类型 - 基于模式匹配的类型选择（条件类型版本）
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

// ============================================================================
// From type_match.rs — 模式匹配引擎
// ============================================================================

/// 类型级模式
///
/// 支持三种模式：
/// - `LiteralPattern`: 字面量类型模式（如 `Zero`、`True`、`5`）
/// - `ConstructorPattern`: 构造器模式（如 `Succ(Zero)`）
/// - `WildcardPattern`: 通配符 `_`（匹配任何类型）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MatchPattern {
    /// 字面量类型模式
    ///
    /// 匹配具体的类型字面量：
    /// - 类型构造器（Zero, Succ, True, False）
    /// - 数值字面量
    /// - 类型引用
    Literal(MonoType),

    /// 构造器模式
    ///
    /// 匹配带参数的构造器：
    /// - `Succ(Zero)` 匹配 Successor 类型
    /// - `Some(T)` 匹配 Option 的 Some 分支
    /// - `Node(Value, Left, Right)` 匹配递归类型
    Constructor {
        /// 构造器名称
        name: String,
        /// 构造器参数模式列表
        arguments: Vec<MatchPattern>,
    },

    /// 元组模式
    ///
    /// 匹配多元素模式：
    /// - `(A, B)` 用于多参数匹配
    /// - 支持嵌套
    Tuple(Vec<MatchPattern>),

    /// 通配符模式
    ///
    /// 匹配任何类型，不绑定值：
    /// - `_` 匹配任意类型
    /// - `_name` 绑定匹配的值到名称
    Wildcard(Option<String>),
}

impl MatchPattern {
    /// 创建字面量模式
    pub fn literal(ty: MonoType) -> Self {
        MatchPattern::Literal(ty)
    }

    /// 创建构造器模式
    pub fn constructor(
        name: impl Into<String>,
        arguments: Vec<MatchPattern>,
    ) -> Self {
        MatchPattern::Constructor {
            name: name.into(),
            arguments,
        }
    }

    /// 创建元组模式
    pub fn tuple(patterns: Vec<MatchPattern>) -> Self {
        MatchPattern::Tuple(patterns)
    }

    /// 创建通配符模式
    pub fn wildcard() -> Self {
        MatchPattern::Wildcard(None)
    }

    /// 创建命名通配符模式
    pub fn wildcard_named(name: impl Into<String>) -> Self {
        MatchPattern::Wildcard(Some(name.into()))
    }

    /// 创建简单的字面量构造器模式
    pub fn named(name: impl Into<String>) -> Self {
        MatchPattern::constructor(name, vec![])
    }

    /// 检查模式是否为通配符
    pub fn is_wildcard(&self) -> bool {
        matches!(self, MatchPattern::Wildcard(_))
    }

    /// 获取模式作为字面量类型（如果是的话）
    pub fn as_literal(&self) -> Option<&MonoType> {
        match self {
            MatchPattern::Literal(ty) => Some(ty),
            _ => None,
        }
    }

    /// 创建 Succ 模式
    pub fn succ(arg: MatchPattern) -> Self {
        MatchPattern::constructor("Succ", vec![arg])
    }

    /// 创建 Zero 模式
    pub fn zero() -> Self {
        MatchPattern::named("Zero")
    }

    /// 创建 True 模式
    pub fn t() -> Self {
        MatchPattern::Literal(MonoType::TypeRef("True".to_string()))
    }

    /// 创建 False 模式
    pub fn f() -> Self {
        MatchPattern::Literal(MonoType::TypeRef("False".to_string()))
    }
}

/// 匹配分支（模式匹配版本，使用 MatchPattern）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatternMatchArm {
    /// 匹配模式
    pub pattern: MatchPattern,
    /// 匹配成功时的结果类型
    pub result: MonoType,
}

impl PatternMatchArm {
    /// 创建新的匹配分支
    pub fn new(
        pattern: MatchPattern,
        result: MonoType,
    ) -> Self {
        Self { pattern, result }
    }

    /// 创建通配符分支
    pub fn wildcard(result: MonoType) -> Self {
        Self::new(MatchPattern::wildcard(), result)
    }
}

/// 类型级 Match 表达式（模式匹配版本）
///
/// 基于模式匹配选择类型：
/// ```yaoxiang
/// type Add[A: Nat, B: Nat] = match (A, B) {
///     (Zero, B) => B,
///     (Succ(A'), B) => Succ(Add(A', B)),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatternMatchType {
    /// 被匹配的目标类型或元组
    pub subject: MonoType,

    /// 匹配分支列表
    pub arms: Vec<PatternMatchArm>,
}

impl PatternMatchType {
    /// 创建新的类型匹配
    pub fn new(
        subject: MonoType,
        arms: Vec<PatternMatchArm>,
    ) -> Self {
        Self { subject, arms }
    }

    /// 创建基于单目标类型的匹配
    pub fn on<T: Into<MonoType>>(
        subject: T,
        arms: Vec<PatternMatchArm>,
    ) -> Self {
        Self::new(subject.into(), arms)
    }

    /// 添加匹配分支
    pub fn add_arm(
        &mut self,
        pattern: MatchPattern,
        result: MonoType,
    ) {
        self.arms.push(PatternMatchArm::new(pattern, result));
    }

    /// 添加通配符分支
    pub fn with_wildcard(
        &mut self,
        result: MonoType,
    ) {
        self.arms.push(PatternMatchArm::wildcard(result));
    }

    /// 评估匹配类型
    pub fn eval(&self) -> TypeLevelResult<MonoType> {
        // 简化实现：查找第一个匹配的模式
        // 完整的实现需要 PatternMatcher 进行模式匹配
        let matcher = PatternMatcher::default();

        for arm in &self.arms {
            if matcher.matches(&self.subject, &arm.pattern) {
                return TypeLevelResult::Normalized(arm.result.clone());
            }
        }

        // 没有匹配的分支
        TypeLevelResult::Error(TypeLevelError::ComputationFailed(format!(
            "No pattern matched for type {:?}",
            self.subject
        )))
    }

    /// 检查是否有通配符分支
    pub fn has_wildcard(&self) -> bool {
        self.arms.iter().any(|arm| arm.pattern.is_wildcard())
    }

    /// 获取分支数量
    pub fn arm_count(&self) -> usize {
        self.arms.len()
    }
}

/// 模式匹配引擎
///
/// 执行实际的模式匹配逻辑：
/// - 支持字面量精确匹配
/// - 支持构造器模式匹配
/// - 支持元组解构
/// - 支持通配符绑定
#[derive(Debug, Default)]
pub struct PatternMatcher {}

impl PatternMatcher {
    /// 创建新的模式匹配器
    pub fn new() -> Self {
        Self {}
    }

    /// 检查目标类型是否匹配给定模式
    pub fn matches(
        &self,
        target: &MonoType,
        pattern: &MatchPattern,
    ) -> bool {
        match pattern {
            MatchPattern::Literal(pat) => self.match_literal(target, pat),
            MatchPattern::Constructor { name, arguments } => {
                self.match_constructor(target, name, arguments)
            }
            MatchPattern::Tuple(patterns) => self.match_tuple(target, patterns),
            MatchPattern::Wildcard(_) => true,
        }
    }

    /// 字面量模式匹配
    fn match_literal(
        &self,
        target: &MonoType,
        pattern: &MonoType,
    ) -> bool {
        // 精确类型匹配
        target == pattern
    }

    /// 构造器模式匹配
    fn match_constructor(
        &self,
        target: &MonoType,
        name: &str,
        arguments: &[MatchPattern],
    ) -> bool {
        match target {
            // 检查类型引用是否匹配构造器名称
            MonoType::TypeRef(type_name) => {
                // 处理参数化类型如 Some(T)
                if let Some((ctor, params)) = self.parse_type_app(type_name) {
                    ctor == name && (arguments.is_empty() || params.len() >= arguments.len())
                } else {
                    // 简单类型引用
                    type_name == name && arguments.is_empty()
                }
            }
            // 其他类型（列表、字典等）
            _ => false,
        }
    }

    /// 元组模式匹配
    fn match_tuple(
        &self,
        target: &MonoType,
        patterns: &[MatchPattern],
    ) -> bool {
        match target {
            MonoType::Tuple(elements) if elements.len() == patterns.len() => elements
                .iter()
                .zip(patterns.iter())
                .all(|(elem, pat)| self.matches(elem, pat)),
            _ => false,
        }
    }

    /// 解析类型应用（如 `Some(Int)` 或 `Succ(Zero)`）
    fn parse_type_app(
        &self,
        type_name: &str,
    ) -> Option<(String, Vec<String>)> {
        // 尝试圆括号格式：Succ(Zero)
        if let Some(start) = type_name.find('(') {
            if let Some(end) = type_name.find(')') {
                let ctor = type_name[..start].to_string();
                let params_str = &type_name[start + 1..end];
                let params: Vec<String> = params_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                return Some((ctor, params));
            }
        }

        None
    }
}

/// 匹配结果绑定
///
/// 用于从匹配中提取值：
/// - 通配符命名绑定
/// - 构造器参数绑定
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchBinding {
    /// 变量名 -> 类型
    bindings: Vec<(String, MonoType)>,
}

impl MatchBinding {
    /// 创建空绑定
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// 添加绑定
    pub fn bind(
        &mut self,
        name: impl Into<String>,
        ty: MonoType,
    ) {
        self.bindings.push((name.into(), ty));
    }

    /// 获取绑定的类型
    pub fn get(
        &self,
        name: &str,
    ) -> Option<&MonoType> {
        self.bindings
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, ty)| ty)
    }

    /// 合并绑定
    pub fn merge(
        &self,
        other: &MatchBinding,
    ) -> MatchBinding {
        let mut merged = self.clone();
        merged.bindings.extend(other.bindings.clone());
        merged
    }
}

impl Default for MatchBinding {
    fn default() -> Self {
        Self::new()
    }
}

/// 模式构建器
///
/// 提供流式 API 构建复杂模式：
/// ```rust,ignore
/// use crate::frontend::core::types::MonoType;
///
/// let pattern = PatternBuilder::new()
///     .wildcard("x")
///     .literal(MonoType::TypeRef("Int".to_string()))
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct PatternBuilder {
    patterns: Vec<MatchPattern>,
}

impl PatternBuilder {
    /// 创建新的模式构建器
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// 添加字面量模式
    pub fn literal(
        mut self,
        ty: MonoType,
    ) -> Self {
        self.patterns.push(MatchPattern::literal(ty));
        self
    }

    /// 添加构造器模式
    pub fn constructor(
        mut self,
        name: impl Into<String>,
        arguments: Vec<MatchPattern>,
    ) -> Self {
        self.patterns
            .push(MatchPattern::constructor(name, arguments));
        self
    }

    /// 添加通配符模式
    pub fn wildcard(
        mut self,
        name: Option<&str>,
    ) -> Self {
        self.patterns
            .push(MatchPattern::Wildcard(name.map(|s| s.to_string())));
        self
    }

    /// 添加命名通配符
    pub fn named(
        mut self,
        name: impl Into<String>,
    ) -> Self {
        self.patterns.push(MatchPattern::wildcard_named(name));
        self
    }

    /// 构建元组模式
    pub fn tuple(self) -> MatchPattern {
        MatchPattern::tuple(self.patterns)
    }

    /// 构建单个模式
    pub fn build(self) -> MatchPattern {
        if self.patterns.len() == 1 {
            self.patterns.into_iter().next().unwrap()
        } else {
            MatchPattern::tuple(self.patterns)
        }
    }
}

/// 自然数类型族的 Add 定义示例
///
/// 展示如何使用 PatternMatchType 实现类型级加法：
/// ```yaoxiang
/// type Add[A: Nat, B: Nat] = match (A, B) {
///     (Zero, B) => B,
///     (Succ(A'), B) => Succ(Add(A', B)),
/// }
/// ```
pub mod nat_examples {
    use super::*;

    /// 创建自然数加法匹配类型
    pub fn add_type(
        a: MonoType,
        b: MonoType,
    ) -> PatternMatchType {
        PatternMatchType::new(
            MonoType::Tuple(vec![a, b]),
            vec![
                // (Zero, B) => B
                PatternMatchArm::new(
                    MatchPattern::tuple(vec![
                        MatchPattern::zero(),
                        MatchPattern::wildcard_named("b"),
                    ]),
                    MonoType::TypeRef("b".to_string()),
                ),
                // (Succ(A'), B) => Succ(Add(A', B))
                PatternMatchArm::new(
                    MatchPattern::tuple(vec![
                        MatchPattern::succ(MatchPattern::wildcard_named("a'")),
                        MatchPattern::wildcard_named("b"),
                    ]),
                    MonoType::TypeRef("Succ".to_string()), // 简化：实际需要计算
                ),
            ],
        )
    }
}
