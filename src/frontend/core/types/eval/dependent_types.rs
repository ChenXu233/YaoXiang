//! RFC-011 依赖类型支持
//!
//! 提供有限的依赖类型能力：
//! - 类型族: 类型依赖于类型参数
//! - 关联类型: 类型中定义的类型成员
//!
//! RFC-011 设计中，YaoXiang 的依赖类型主要用于：
//! ```yaoxiang
//! type Iterator[T] = {
//!     Item: T,           # 关联类型
//!     next: (Self) -> Option[T],
//! }
//!
//! type Container[T] = {
//!     Item: T,
//!     IteratorType: Iterator[T],  # 关联类型依赖于 T
//! }
//! ```
use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::eval::type_families::Nat;
use std::collections::HashMap;

/// 关联类型定义
///
/// 表示类型中定义的关联类型成员
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssociatedType {
    /// 关联类型名称
    pub name: String,

    /// 关联类型的定义（可能是类型参数）
    pub definition: AssociatedTypeDef,
}

impl AssociatedType {
    /// 创建新的关联类型
    pub fn new(
        name: String,
        definition: AssociatedTypeDef,
    ) -> Self {
        Self { name, definition }
    }
}

/// 关联类型定义
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssociatedTypeDef {
    /// 直接类型
    Direct(MonoType),

    /// 类型参数引用
    TypeParam(String),

    /// 类型参数上的变换
    /// 例如: `Iterator[T]` 表示对类型参数 T 应用 Iterator
    Applied {
        /// 类型构造器名称
        constructor: String,

        /// 类型参数
        arguments: Vec<AssociatedTypeDef>,
    },

    /// 模式匹配类型族
    /// 根据实际参数匹配对应的结果类型（如 IsTrue 族）
    Match {
        /// 要匹配的参数索引
        arg_index: usize,
        /// 匹配分支 (pattern, result_type)
        arms: Vec<(MonoType, MonoType)>,
    },

    /// 类型级递归定义
    /// 通过 Nat 类型参数的结构分解来实现递归
    /// 例如: factorial(Zero) = 1; factorial(Succ(n)) = Succ(n) * factorial(n)
    Recursive {
        /// 递归参数索引
        arg_index: usize,
        /// 递归分支
        arms: Vec<RecursiveArm>,
    },
}

impl AssociatedTypeDef {
    /// 检查是否包含未绑定的类型参数
    pub fn has_unbound_params(
        &self,
        bound_params: &[String],
    ) -> bool {
        match self {
            AssociatedTypeDef::Direct(_) => false,
            AssociatedTypeDef::TypeParam(name) => !bound_params.contains(name),
            AssociatedTypeDef::Applied { arguments, .. } => arguments
                .iter()
                .any(|arg| arg.has_unbound_params(bound_params)),
            AssociatedTypeDef::Match { .. } => false,
            AssociatedTypeDef::Recursive { .. } => false,
        }
    }

    /// 替换类型参数
    pub fn substitute(
        &self,
        substitutions: &HashMap<String, MonoType>,
    ) -> Self {
        match self {
            AssociatedTypeDef::Direct(ty) => {
                // 替换 MonoType 中的类型引用
                if let MonoType::TypeRef(type_name) = ty {
                    // 先尝试精确匹配整个类型名称
                    if let Some(replacement) = substitutions.get(type_name) {
                        return AssociatedTypeDef::Direct(replacement.clone());
                    }
                    // 再尝试在类型名称字符串内进行文本替换
                    // 例如 "IsTrue(cond)" → "IsTrue(true)" 当 cond 被替换为 true 时
                    let mut new_name = type_name.clone();
                    let mut substituted = false;
                    for (param, replacement) in substitutions.iter() {
                        if let MonoType::TypeRef(repl_name) = replacement {
                            if new_name.contains(param.as_str()) {
                                new_name = new_name.replace(param.as_str(), repl_name);
                                substituted = true;
                            }
                        }
                    }
                    if substituted {
                        return AssociatedTypeDef::Direct(MonoType::TypeRef(new_name));
                    }
                }
                AssociatedTypeDef::Direct(ty.clone())
            }
            AssociatedTypeDef::TypeParam(name) => {
                if let Some(replacement) = substitutions.get(name) {
                    AssociatedTypeDef::Direct(replacement.clone())
                } else {
                    AssociatedTypeDef::TypeParam(name.clone())
                }
            }
            AssociatedTypeDef::Applied {
                constructor,
                arguments,
            } => AssociatedTypeDef::Applied {
                constructor: constructor.clone(),
                arguments: arguments
                    .iter()
                    .map(|arg| arg.substitute(substitutions))
                    .collect(),
            },
            AssociatedTypeDef::Match { arg_index, arms } => AssociatedTypeDef::Match {
                arg_index: *arg_index,
                arms: arms.clone(),
            },
            AssociatedTypeDef::Recursive { arg_index, arms } => AssociatedTypeDef::Recursive {
                arg_index: *arg_index,
                arms: arms
                    .iter()
                    .map(|arm| {
                        let substituted_result = substitute_type_params(&arm.result, substitutions);
                        RecursiveArm {
                            pattern: arm.pattern.clone(),
                            result: substituted_result,
                        }
                    })
                    .collect(),
            },
        }
    }

    /// 转换为 MonoType
    pub fn into_type(self) -> MonoType {
        match self {
            AssociatedTypeDef::Direct(ty) => ty,
            AssociatedTypeDef::TypeParam(_) => MonoType::TypeRef("Unknown".to_string()),
            AssociatedTypeDef::Applied { constructor, .. } => MonoType::TypeRef(constructor),
            AssociatedTypeDef::Match { .. } => MonoType::TypeRef("Unknown".to_string()),
            AssociatedTypeDef::Recursive { .. } => MonoType::TypeRef("Unknown".to_string()),
        }
    }
}

/// 递归匹配模式
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RecursivePattern {
    /// 零基情况: factorial(Zero) → ...
    Zero,
    /// 递推情况: factorial(Succ(n)) → ...
    /// 包含归纳假设变量名
    Succ(String),
}

/// 递归分支
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecursiveArm {
    pub pattern: RecursivePattern,
    pub result: MonoType,
}

/// 替换 MonoType 中的类型参数引用
///
/// 将 body 中所有匹配替换映射的类型引用替换为对应的值。
/// 也支持在 TypeRef 名称字符串内进行替换（例如 "IsTrue(cond)" → "IsTrue(true)"）。
fn substitute_type_params(
    body: &MonoType,
    substitutions: &HashMap<String, MonoType>,
) -> MonoType {
    match body {
        MonoType::TypeRef(name) => {
            // 尝试精确匹配整个名称
            if let Some(replacement) = substitutions.get(name) {
                return replacement.clone();
            }
            // 尝试在名称字符串内进行文本替换（如 "IsTrue(cond)" → "IsTrue(true)"）
            // 按参数名长度降序排序，避免 "n" 替换到 "ih_n" 内部
            let mut sorted_params: Vec<&String> = substitutions.keys().collect();
            sorted_params.sort_by(|a, b| b.len().cmp(&a.len()));
            let mut new_name = name.clone();
            for param in sorted_params {
                if let Some(MonoType::TypeRef(repl_name)) = substitutions.get(param) {
                    new_name = new_name.replace(param.as_str(), repl_name);
                }
            }
            if new_name != *name {
                MonoType::TypeRef(new_name)
            } else {
                body.clone()
            }
        }
        // 其他类型直接返回
        _ => body.clone(),
    }
}

/// 类型族
///
/// 表示一个依赖于类型参数的泛型类型定义
///
/// # 示例
/// ```yaoxiang
/// type Result[T, E] = Ok(T) | Err(E)  # Result 是 T 和 E 的类型族
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeFamily {
    /// 类型族名称
    pub name: String,

    /// 类型参数
    pub type_params: Vec<String>,

    /// 关联类型
    pub associated_types: Vec<AssociatedType>,

    /// 定义（类型表达式）
    pub definition: AssociatedTypeDef,
}

impl TypeFamily {
    /// 创建新的类型族
    pub fn new(
        name: String,
        type_params: Vec<String>,
        associated_types: Vec<AssociatedType>,
        definition: AssociatedTypeDef,
    ) -> Self {
        Self {
            name,
            type_params,
            associated_types,
            definition,
        }
    }

    /// 实例化类型族
    pub fn instantiate(
        &self,
        args: &[MonoType],
    ) -> Option<AssociatedTypeDef> {
        // 检查参数数量是否匹配
        if self.type_params.len() != args.len() {
            return None;
        }

        // 构建替换映射
        let substitutions: HashMap<String, MonoType> = self
            .type_params
            .iter()
            .zip(args.iter())
            .map(|(param, arg)| (param.clone(), arg.clone()))
            .collect();

        // 根据定义类型分派
        match &self.definition {
            AssociatedTypeDef::Match { arg_index, arms } => {
                let scrutinee = args.get(*arg_index)?;
                for (pattern, result) in arms {
                    if scrutinee == pattern {
                        return Some(AssociatedTypeDef::Direct(
                            self.substitute_params(result, args)?,
                        ));
                    }
                }
                None
            }
            AssociatedTypeDef::Recursive { arg_index, arms } => {
                let scrutinee = args.get(*arg_index)?;
                // 解析被匹配类型为 Nat
                let nat_val = parse_nat_from_type(scrutinee)?;
                match nat_val {
                    Nat::Zero => {
                        // 查找 Zero 分支
                        let zero_arm = arms
                            .iter()
                            .find(|arm| arm.pattern == RecursivePattern::Zero)?;
                        Some(AssociatedTypeDef::Direct(
                            self.substitute_params(&zero_arm.result, args)?,
                        ))
                    }
                    Nat::Succ(inner) => {
                        // 查找 Succ 分支并提取归纳假设变量名（使用 filter_map 避免 unreachable!）
                        let (hyp_var, succ_result) = arms
                            .iter()
                            .filter_map(|arm| {
                                if let RecursivePattern::Succ(var) = &arm.pattern {
                                    Some((var.clone(), &arm.result))
                                } else {
                                    None
                                }
                            })
                            .next()?;
                        // 构建替换映射：type_params → args
                        let mut all_substitutions: HashMap<String, MonoType> = self
                            .type_params
                            .iter()
                            .zip(args.iter())
                            .map(|(param, arg)| (param.clone(), arg.clone()))
                            .collect();
                        // 归纳假设变量绑定到 inner 的 Nat 编码（如 Succ(Zero)）
                        // 这样 factorial(ih) 中的 ih 会被替换为 Succ(Zero)，产生 factorial(Succ(Zero))
                        // 求值器随后会进一步归约这个递归调用
                        let inner_type = nat_to_type(&inner);
                        all_substitutions.insert(hyp_var, inner_type);
                        let substituted = substitute_type_params(succ_result, &all_substitutions);
                        Some(AssociatedTypeDef::Direct(substituted))
                    }
                }
            }
            _ => {
                // 替换定义中的类型参数（Direct、TypeParam、Applied）
                Some(self.definition.substitute(&substitutions))
            }
        }
    }

    /// 在 MonoType 中替换类型参数
    ///
    /// 将 body 中的类型参数名替换为对应的实际参数值
    pub fn substitute_params(
        &self,
        body: &MonoType,
        args: &[MonoType],
    ) -> Option<MonoType> {
        if self.type_params.len() != args.len() {
            return None;
        }
        let substitutions: HashMap<String, MonoType> = self
            .type_params
            .iter()
            .zip(args.iter())
            .map(|(param, arg)| (param.clone(), arg.clone()))
            .collect();
        Some(substitute_type_params(body, &substitutions))
    }

    /// 获取所有类型参数
    pub fn type_params(&self) -> &[String] {
        &self.type_params
    }

    /// 获取关联类型
    pub fn get_associated_type(
        &self,
        name: &str,
    ) -> Option<&AssociatedType> {
        self.associated_types.iter().find(|at| at.name == name)
    }

    /// 实例化关联类型
    pub fn instantiate_associated_type(
        &self,
        name: &str,
        args: &[MonoType],
    ) -> Option<MonoType> {
        let associated = self.get_associated_type(name)?;
        let substitutions: HashMap<String, MonoType> = self
            .type_params
            .iter()
            .zip(args.iter())
            .map(|(param, arg)| (param.clone(), arg.clone()))
            .collect();

        Some(associated.definition.substitute(&substitutions).into_type())
    }
}

// ============ 递归辅助函数 ============

/// 从 MonoType 解析 Nat 值
///
/// 支持以下格式：
/// - TypeRef("Zero") → Nat::Zero
/// - TypeRef("Succ(N)") → Nat::Succ(...)
/// - Int(n) → Nat::from_usize(n)
pub fn parse_nat_from_type(ty: &MonoType) -> Option<Nat> {
    match ty {
        MonoType::TypeRef(name) if name == "Zero" => Some(Nat::Zero),
        MonoType::TypeRef(name) if name.starts_with("Succ(") => {
            let inner = &name[5..name.len() - 1];
            let inner_nat = parse_nat_from_type(&MonoType::TypeRef(inner.to_string()))?;
            Some(Nat::succ(inner_nat))
        }
        MonoType::Int(n) => Some(Nat::from_usize(*n)),
        _ => None,
    }
}

/// 将 Nat 值转换为 MonoType 表示
pub fn nat_to_type(nat: &Nat) -> MonoType {
    match nat {
        Nat::Zero => MonoType::TypeRef("Zero".to_string()),
        Nat::Succ(inner) => {
            let inner_str = nat_to_type(inner);
            match inner_str {
                MonoType::TypeRef(name) => MonoType::TypeRef(format!("Succ({})", name)),
                _ => {
                    // 理论上不会发生，因为递归基总是 TypeRef("Zero")
                    MonoType::TypeRef(format!("Succ({:?})", inner_str))
                }
            }
        }
    }
}

/// 检查类型中是否包含对自身的递归调用
fn references_self_call(
    ty: &MonoType,
    family_name: &str,
) -> bool {
    match ty {
        MonoType::TypeRef(name) => name.starts_with(&format!("{}(", family_name)),
        _ => false,
    }
}

/// 检查递归调用的参数：必须严格是归纳假设变量（不能是 Succ(hyp_var)）
fn check_recursive_call_args(
    ty: &MonoType,
    family_name: &str,
    hyp_var: &str,
) -> Result<(), String> {
    match ty {
        MonoType::TypeRef(name) if name.starts_with(&format!("{}(", family_name)) => {
            let inner = &name[family_name.len() + 1..name.len() - 1];
            if inner == hyp_var {
                Ok(())
            } else {
                Err(format!(
                    "递归调用 {} 参数必须是归纳假设变量 {}，但实际为 {}",
                    name, hyp_var, inner
                ))
            }
        }
        _ => Ok(()),
    }
}

/// 检查结构性终止：
/// - Zero 分支不能包含自调用
/// - Succ(n) 分支的递归调用参数必须恰好是 n
pub fn check_structural_termination(
    family_name: &str,
    arms: &[RecursiveArm],
) -> Result<(), String> {
    let mut has_zero = false;
    let mut succ_hyp_var: Option<&str> = None;

    for arm in arms {
        match &arm.pattern {
            RecursivePattern::Zero => {
                has_zero = true;
                // Zero 分支不能包含自调用
                if references_self_call(&arm.result, family_name) {
                    return Err(format!(
                        "类型族 {} 的 Zero 分支包含不允许的自调用",
                        family_name
                    ));
                }
            }
            RecursivePattern::Succ(var) => {
                // 检查 Succ(n) 分支中的递归调用参数
                check_recursive_call_args(&arm.result, family_name, var)?;
                succ_hyp_var = Some(var.as_str());
            }
        }
    }

    if !has_zero {
        return Err(format!("类型族 {} 缺少基情况 (Zero) 分支", family_name));
    }

    if succ_hyp_var.is_none() {
        return Err(format!("类型族 {} 缺少递推 (Succ) 分支", family_name));
    }

    Ok(())
}

/// 依赖类型环境
///
/// 管理类型族和关联类型的注册与查找
#[derive(Debug, Clone, Default)]
pub struct DependentTypeEnv {
    /// 类型族映射
    type_families: HashMap<String, TypeFamily>,
}

impl DependentTypeEnv {
    /// 创建新的依赖类型环境
    pub fn new() -> Self {
        Self {
            type_families: HashMap::new(),
        }
    }

    /// 注册类型族
    pub fn register_type_family(
        &mut self,
        family: TypeFamily,
    ) {
        self.type_families.insert(family.name.clone(), family);
    }

    /// 查找类型族
    pub fn get_type_family(
        &self,
        name: &str,
    ) -> Option<&TypeFamily> {
        self.type_families.get(name)
    }

    /// 检查类型是否是类型族实例
    pub fn is_type_family_instance(
        &self,
        _ty: &MonoType,
    ) -> Option<&TypeFamily> {
        // 简化实现：暂不检查类型是否为类型族实例
        None
    }
}
