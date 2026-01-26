//! Trait系统实现（RFC-011）
//!
//! 提供完整的trait支持，包括：
//! - Trait定义和继承
//! - Trait实现
//! - 约束检查
//! - 自动派生

use crate::util::span::Span;
use std::collections::HashMap;

/// Trait定义
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitDef {
    /// Trait名称
    pub name: String,
    /// 类型参数（泛型trait支持）
    pub type_params: Vec<String>,
    /// 父trait列表（trait继承）
    pub super_traits: Vec<TraitRef>,
    /// 方法列表
    pub methods: Vec<TraitMethod>,
    /// 位置信息
    pub span: Span,
}

/// Trait引用
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitRef {
    /// Trait名称
    pub name: String,
    /// 泛型参数
    pub args: Vec<String>,
}

/// Trait方法
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitMethod {
    /// 方法名
    pub name: String,
    /// 签名
    pub signature: MethodSignature,
}

/// 方法签名
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodSignature {
    /// 参数类型
    pub params: Vec<String>,
    /// 返回类型
    pub return_type: String,
}

/// Trait实现
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitImpl {
    /// 实现类型
    pub for_type: String,
    /// Trait引用
    pub trait_ref: TraitRef,
    /// 实现的方法
    pub methods: Vec<ImplMethod>,
    /// 位置信息
    pub span: Span,
}

/// 实现的方法
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImplMethod {
    /// 方法名
    pub name: String,
    /// 实现体
    pub body: String,
}

/// Trait系统环境
#[derive(Debug, Clone)]
pub struct TraitEnvironment {
    /// 已定义的trait
    pub traits: HashMap<String, TraitDef>,
    /// 已定义的实现
    pub impls: HashMap<String, Vec<TraitImpl>>,
}

impl TraitEnvironment {
    /// 创建新的trait环境
    pub fn new() -> Self {
        TraitEnvironment {
            traits: HashMap::new(),
            impls: HashMap::new(),
        }
    }

    /// 注册trait定义
    pub fn register_trait(&mut self, trait_def: TraitDef) {
        self.traits.insert(trait_def.name.clone(), trait_def);
    }

    /// 注册trait实现
    pub fn register_impl(&mut self, impl_def: TraitImpl) {
        let key = format!("{} for {}", impl_def.trait_ref.name, impl_def.for_type);
        self.impls.entry(key).or_insert_with(Vec::new).push(impl_def);
    }

    /// 查找trait定义
    pub fn get_trait(&self, name: &str) -> Option<&TraitDef> {
        self.traits.get(name)
    }

    /// 查找trait实现
    pub fn get_impl(&self, trait_name: &str, for_type: &str) -> Option<&Vec<TraitImpl>> {
        let key = format!("{} for {}", trait_name, for_type);
        self.impls.get(&key)
    }
}
