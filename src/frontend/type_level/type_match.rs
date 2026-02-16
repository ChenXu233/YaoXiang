//! RFC-011 类型级 Match 表达式实现
//!
//! 提供类型级的模式匹配能力：
//! - `MatchPattern`: 匹配模式（字面量、构造器、通配符）
//! - `PatternMatcher`: 模式匹配引擎
//! - `MatchType`: 完整的类型匹配结构
//!
//! 示例：
//! ```yaoxiang
//! type Add[A: Nat, B: Nat] = match (A, B) {
//!     (Zero, B) => B,
//!     (Succ(A'), B) => Succ(Add(A', B)),
//! }
//! ```

use crate::frontend::core::type_system::MonoType;
use super::{TypeLevelError, TypeLevelResult};

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
}

/// 匹配分支
///
/// 将模式与结果类型关联：
/// ```yaoxiang
/// type Add[A: Nat, B: Nat] = match (A, B) {
///     (Zero, B) => B,          // <- MatchArm
///     (Succ(A'), B) => Succ(...),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchArm {
    /// 匹配模式
    pub pattern: MatchPattern,
    /// 匹配成功时的结果类型
    pub result: MonoType,
}

impl MatchArm {
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

/// 类型级 Match 表达式
///
/// 基于模式匹配选择类型：
/// ```yaoxiang
/// type Add[A: Nat, B: Nat] = match (A, B) {
///     (Zero, B) => B,
///     (Succ(A'), B) => Succ(Add(A', B)),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchType {
    /// 被匹配的目标类型或元组
    pub subject: MonoType,

    /// 匹配分支列表
    pub arms: Vec<MatchArm>,
}

impl MatchType {
    /// 创建新的类型匹配
    pub fn new(
        subject: MonoType,
        arms: Vec<MatchArm>,
    ) -> Self {
        Self { subject, arms }
    }

    /// 创建基于单目标类型的匹配
    pub fn on<T: Into<MonoType>>(
        subject: T,
        arms: Vec<MatchArm>,
    ) -> Self {
        Self::new(subject.into(), arms)
    }

    /// 添加匹配分支
    pub fn add_arm(
        &mut self,
        pattern: MatchPattern,
        result: MonoType,
    ) {
        self.arms.push(MatchArm::new(pattern, result));
    }

    /// 添加通配符分支
    pub fn with_wildcard(
        &mut self,
        result: MonoType,
    ) {
        self.arms.push(MatchArm::wildcard(result));
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
        // 首先尝试方括号格式：Some[T]
        if let Some(start) = type_name.find('[') {
            if let Some(end) = type_name.find(']') {
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

        // 其次尝试圆括号格式：Succ(Zero)
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

/// 便利构造函数
impl MatchPattern {
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

/// 模式构建器
///
/// 提供流式 API 构建复杂模式：
/// ```rust,ignore
/// use crate::frontend::core::type_system::MonoType;
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
/// 展示如何使用 MatchType 实现类型级加法：
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
    ) -> MatchType {
        MatchType::new(
            MonoType::Tuple(vec![a, b]),
            vec![
                // (Zero, B) => B
                MatchArm::new(
                    MatchPattern::tuple(vec![
                        MatchPattern::zero(),
                        MatchPattern::wildcard_named("b"),
                    ]),
                    MonoType::TypeRef("b".to_string()),
                ),
                // (Succ(A'), B) => Succ(Add(A', B))
                MatchArm::new(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matcher_literal() {
        let matcher = PatternMatcher::new();
        let target = MonoType::TypeRef("Zero".to_string());
        let pattern = MatchPattern::Literal(MonoType::TypeRef("Zero".to_string()));

        assert!(matcher.matches(&target, &pattern));
    }

    #[test]
    fn test_pattern_matcher_wildcard() {
        let matcher = PatternMatcher::new();
        let target = MonoType::TypeRef("Succ(Zero)".to_string());
        let pattern = MatchPattern::wildcard();

        assert!(matcher.matches(&target, &pattern));
    }

    #[test]
    fn test_pattern_matcher_constructor() {
        let matcher = PatternMatcher::new();
        let target = MonoType::TypeRef("Succ(Zero)".to_string());
        let pattern = MatchPattern::constructor("Succ", vec![]);

        // 简化匹配
        assert!(matcher.matches(&target, &pattern));
    }

    #[test]
    fn test_match_type_basic() {
        let match_type = MatchType::new(
            MonoType::TypeRef("Zero".to_string()),
            vec![
                MatchArm::new(MatchPattern::zero(), MonoType::TypeRef("Nat".to_string())),
                MatchArm::wildcard(MonoType::TypeRef("Unknown".to_string())),
            ],
        );

        let result = match_type.eval();
        assert!(result.is_normalized());
    }

    #[test]
    fn test_pattern_builder() {
        let pattern = PatternBuilder::new().wildcard(Some("x")).named("y").tuple();

        assert!(matches!(pattern, MatchPattern::Tuple(_)));
    }
}
