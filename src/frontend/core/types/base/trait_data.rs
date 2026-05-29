//! Trait 数据定义
//!
//! 核心 trait 系统的数据结构和存储。仅包含数据定义，不包含求解逻辑。
//! 求解器逻辑位于 `typecheck/traits/solver.rs`。

use std::collections::HashMap;
use crate::frontend::core::types::base::MonoType;

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
    /// 是否为标记 trait（无方法，仅作为类型级标记）
    pub is_marker: bool,
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

/// Trait 实现
#[derive(Debug, Clone)]
pub struct TraitImplementation {
    pub trait_name: String,
    pub for_type_name: String,
    /// 方法签名映射: method_name -> MonoType
    pub methods: HashMap<String, MonoType>,
}

/// Trait 表 - 存储所有已解析的 Trait 定义和实现
#[derive(Debug, Clone, Default)]
pub struct TraitTable {
    /// Trait 定义存储: name -> TraitDefinition
    traits: HashMap<String, TraitDefinition>,
    /// Trait 实现存储: (trait_name, for_type) -> TraitImplementation
    implementations: HashMap<(String, String), TraitImplementation>,
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
}
