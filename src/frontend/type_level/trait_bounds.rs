//! Trait 边界表示和处理
//!
//! 实现 RFC-011 Trait 系统的边界表示和约束求解：
//! - Trait 定义存储
//! - Trait 实现存储
//! - Trait 边界解析
//! - 约束求解

use std::collections::HashMap;
use crate::frontend::core::type_system::MonoType;

/// Trait 方法签名
#[derive(Debug, Clone)]
pub struct TraitMethodSignature {
    pub name: String,
    pub params: Vec<MonoType>,
    pub return_type: MonoType,
    pub is_static: bool,
}

/// Trait 定义
#[derive(Debug, Clone)]
pub struct TraitDefinition {
    pub name: String,
    /// 方法签名映射
    pub methods: HashMap<String, TraitMethodSignature>,
    /// 父 Trait 列表（用于继承）
    pub parent_traits: Vec<String>,
    /// 泛型参数
    pub generic_params: Vec<String>,
    /// Trait 定义的位置（用于错误信息）
    pub span: Option<crate::util::span::Span>,
}

/// Trait 边界（用于泛型约束）
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TraitBound {
    pub trait_name: String,
    /// 约束的 Self 类型（通常是类型变量）
    pub self_type: MonoType,
}

/// Trait 边界列表
pub type TraitBounds = Vec<TraitBound>;

/// Trait 表 - 存储所有已解析的 Trait 定义
#[derive(Debug, Clone, Default)]
pub struct TraitTable {
    /// Trait 定义存储: name -> TraitDefinition
    traits: HashMap<String, TraitDefinition>,
    /// Trait 实现存储: (trait_name, for_type) -> TraitImplementation
    implementations: HashMap<(String, String), TraitImplementation>,
    /// Trait 边界求解缓存: (trait_name, type_str) -> bool
    solver_cache: HashMap<String, bool>,
}

impl TraitTable {
    /// 创建新的 Trait 表
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加 Trait 定义
    pub fn add_trait(
        &mut self,
        definition: TraitDefinition,
    ) {
        self.traits.insert(definition.name.clone(), definition);
    }

    /// 获取 Trait 定义
    pub fn get_trait(
        &self,
        name: &str,
    ) -> Option<&TraitDefinition> {
        self.traits.get(name)
    }

    /// 检查 Trait 是否已定义
    pub fn has_trait(
        &self,
        name: &str,
    ) -> bool {
        self.traits.contains_key(name)
    }

    /// 检查类型是否实现了 Trait
    pub fn has_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> bool {
        self.implementations
            .contains_key(&(trait_name.to_string(), for_type.to_string()))
    }

    /// 获取 Trait 实现
    pub fn get_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> Option<&TraitImplementation> {
        self.implementations
            .get(&(trait_name.to_string(), for_type.to_string()))
    }

    /// 添加 Trait 实现
    pub fn add_impl(
        &mut self,
        impl_: TraitImplementation,
    ) {
        let key = (impl_.trait_name.clone(), impl_.for_type_name.clone());
        self.implementations.insert(key, impl_);
    }

    /// 获取类型的方法实现
    pub fn get_method_impl(
        &self,
        trait_name: &str,
        for_type: &str,
        method_name: &str,
    ) -> Option<&MonoType> {
        self.implementations
            .get(&(trait_name.to_string(), for_type.to_string()))
            .and_then(|impl_| impl_.methods.get(method_name))
    }

    /// 获取所有 Trait 名称
    pub fn trait_names(&self) -> impl Iterator<Item = &String> {
        self.traits.keys()
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.solver_cache.clear();
    }
}

/// Trait 实现
#[derive(Debug, Clone)]
pub struct TraitImplementation {
    pub trait_name: String,
    pub for_type_name: String,
    /// 方法签名映射: method_name -> MonoType
    pub methods: HashMap<String, MonoType>,
}

/// Trait 求解器
#[derive(Debug, Default)]
pub struct TraitSolver {
    /// 待求解的约束列表
    constraints: Vec<TraitBound>,
    /// Trait 表
    trait_table: TraitTable,
    /// 已求解的约束
    resolved: HashSet<(String, String)>,
}

impl TraitSolver {
    /// 创建新的求解器
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            trait_table: TraitTable::default(),
            resolved: HashSet::new(),
        }
    }

    /// 设置 Trait 表
    pub fn set_trait_table(
        &mut self,
        table: TraitTable,
    ) {
        self.trait_table = table;
    }

    /// 添加约束
    pub fn add_constraint(
        &mut self,
        bound: TraitBound,
    ) {
        self.constraints.push(bound);
    }

    /// 添加多个约束
    pub fn add_constraints(
        &mut self,
        bounds: impl IntoIterator<Item = TraitBound>,
    ) {
        self.constraints.extend(bounds);
    }

    /// 求解所有约束
    pub fn solve(&mut self) -> Result<(), TraitSolverError> {
        // 取出所有约束以避免借用冲突
        let constraints = std::mem::take(&mut self.constraints);
        for bound in constraints {
            self.solve_bound(bound)?;
        }
        Ok(())
    }

    /// 求解单个边界
    fn solve_bound(
        &mut self,
        bound: TraitBound,
    ) -> Result<(), TraitSolverError> {
        let type_name = bound.self_type.type_name();
        let cache_key = format!("{}:{}", bound.trait_name, type_name);

        // 检查是否已在求解中（循环检测）
        if let Some(&cached) = self.trait_table.solver_cache.get(&cache_key) {
            if !cached {
                return Err(TraitSolverError::UnsatisfiedConstraint {
                    trait_name: bound.trait_name,
                    type_name,
                });
            }
            return Ok(());
        }

        // 检查 Trait 是否存在
        if !self.trait_table.has_trait(&bound.trait_name) {
            return Err(TraitSolverError::UndefinedTrait {
                trait_name: bound.trait_name,
            });
        }

        // 检查类型是否实现了 Trait
        if !self.trait_table.has_impl(&bound.trait_name, &type_name) {
            return Err(TraitSolverError::MissingImpl {
                trait_name: bound.trait_name,
                type_name,
            });
        }

        // 标记为已求解
        self.resolved.insert((bound.trait_name, type_name));

        Ok(())
    }

    /// 检查类型是否满足 Trait 约束
    pub fn satisfies_trait(
        &self,
        trait_name: &str,
        ty: &MonoType,
    ) -> bool {
        let type_name = ty.type_name();
        self.trait_table.has_impl(trait_name, &type_name)
    }

    /// 获取未满足的约束
    pub fn unsatisfied_constraints(&self) -> Vec<&TraitBound> {
        self.constraints
            .iter()
            .filter(|b| {
                !self
                    .resolved
                    .contains(&(b.trait_name.clone(), b.self_type.type_name()))
            })
            .collect()
    }
}

/// Trait 求解错误
#[derive(Debug, Clone)]
pub enum TraitSolverError {
    UndefinedTrait {
        trait_name: String,
    },
    MissingImpl {
        trait_name: String,
        type_name: String,
    },
    UnsatisfiedConstraint {
        trait_name: String,
        type_name: String,
    },
    CyclicInheritance {
        trait_name: String,
    },
    MethodNotFound {
        trait_name: String,
        method_name: String,
    },
}

impl std::fmt::Display for TraitSolverError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::UndefinedTrait { trait_name } => {
                write!(f, "Undefined trait: `{}`", trait_name)
            }
            Self::MissingImpl {
                trait_name,
                type_name,
            } => {
                write!(
                    f,
                    "Type `{}` does not implement trait `{}`",
                    type_name, trait_name
                )
            }
            Self::UnsatisfiedConstraint {
                trait_name,
                type_name,
            } => {
                write!(
                    f,
                    "Constraint `{}: {}` is not satisfied",
                    type_name, trait_name
                )
            }
            Self::CyclicInheritance { trait_name } => {
                write!(f, "Cyclic inheritance detected in trait: {}", trait_name)
            }
            Self::MethodNotFound {
                trait_name,
                method_name,
            } => {
                write!(
                    f,
                    "Method `{}` not found in trait `{}`",
                    method_name, trait_name
                )
            }
        }
    }
}

impl std::error::Error for TraitSolverError {}

// 需要的导入
use std::hash::Hash;
use std::collections::HashSet;
