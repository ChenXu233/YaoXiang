//! 类型环境模块
//!
//! 管理类型检查过程中的所有状态信息

use std::collections::{HashMap, HashSet};

use crate::frontend::core::type_system::{MonoType, PolyType, TypeConstraintSolver};

use super::overload;
use super::types::ImportInfo;

/// 类型错误收集器
pub type TypeErrorCollector = crate::util::diagnostic::ErrorCollector<super::Diagnostic>;

/// 类型环境
///
/// 存储类型检查过程中的所有状态信息：
/// - 变量绑定
/// - 类型定义
/// - 约束求解器
/// - 错误收集
/// - 导入/导出信息
/// - 方法绑定
/// - Trait 表
/// - Native 函数签名
#[derive(Debug, Default)]
pub struct TypeEnvironment {
    pub vars: HashMap<String, PolyType>,
    pub types: HashMap<String, PolyType>,
    pub solver: TypeConstraintSolver,
    pub errors: TypeErrorCollector,
    /// 导入追踪 - 模块导入信息
    /// 包含源模块ID用于访问控制
    pub imports: Vec<ImportInfo>,
    /// 当前模块的导出项
    pub exports: HashSet<String>,
    /// 方法绑定关系: "Type.method" -> FunctionType
    /// 用于存储显式绑定和 pub 自动绑定
    pub method_bindings: HashMap<String, MonoType>,
    /// 模块名称
    pub module_name: String,
    /// 重载候选存储: 函数名 -> 多个重载版本
    /// 用于支持函数重载解析
    pub overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>>,
    /// Trait 表：存储所有已解析的 Trait 定义和实现
    pub trait_table: super::super::type_level::trait_bounds::TraitTable,
    /// Native 函数签名表：存储已注册的 native 函数类型签名
    /// Key: 函数名（如 "std.io.println"），Value: 函数类型
    pub native_signatures: HashMap<String, MonoType>,
    /// 模块注册表 - 提供统一的模块查询接口
    pub module_registry: super::super::module::registry::ModuleRegistry,
}

impl TypeEnvironment {
    /// 创建新的类型环境
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建新的类型环境（带模块名）
    pub fn new_with_module(module_name: String) -> Self {
        Self {
            module_name,
            trait_table: super::super::type_level::trait_bounds::TraitTable::default(),
            module_registry: super::super::module::registry::ModuleRegistry::with_std(),
            ..Self::default()
        }
    }

    /// 添加变量绑定
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.vars.insert(name, poly);
    }

    /// 获取变量类型
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.vars.get(name)
    }

    /// 获取求解器
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        &mut self.solver
    }

    /// 添加类型定义
    pub fn add_type(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.types.insert(name, poly);
    }

    /// 获取类型定义
    pub fn get_type(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.types.get(name)
    }

    /// 添加方法绑定
    /// 例如: Point.distance = distance 存储为 "Point.distance" -> fn_type
    pub fn add_method_binding(
        &mut self,
        type_name: &str,
        method_name: &str,
        fn_type: MonoType,
    ) {
        let key = format!("{}.{}", type_name, method_name);
        self.method_bindings.insert(key.clone(), fn_type);
        // 方法绑定也导出
        self.exports.insert(key);
    }

    /// 获取方法绑定
    pub fn get_method_binding(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> Option<&MonoType> {
        let key = format!("{}.{}", type_name, method_name);
        self.method_bindings.get(&key)
    }

    /// 添加导出项
    pub fn add_export(
        &mut self,
        name: &str,
    ) {
        self.exports.insert(name.to_string());
    }

    /// 检查是否是导出项
    pub fn is_exported(
        &self,
        name: &str,
    ) -> bool {
        self.exports.contains(name)
    }

    /// 检查名称是否可见（可从当前模块访问）
    ///
    /// 一个名称在以下情况下可见：
    /// 1. 在当前模块中定义
    /// 2. 被当前模块导出
    /// 3. 从导入了该名称的模块导入
    pub fn is_visible(
        &self,
        name: &str,
    ) -> bool {
        // 当前模块定义的变量总是可见的
        if self.vars.contains_key(name) {
            return true;
        }
        // 当前模块定义的类型总是可见的
        if self.types.contains_key(name) {
            return true;
        }
        // 当前模块导出的内容可见
        if self.exports.contains(name) {
            return true;
        }
        false
    }

    // ============ Trait 相关方法 ============

    /// 添加 Trait 定义
    pub fn add_trait(
        &mut self,
        definition: super::super::type_level::trait_bounds::TraitDefinition,
    ) {
        self.trait_table.add_trait(definition);
    }

    /// 获取 Trait 定义
    pub fn get_trait(
        &self,
        name: &str,
    ) -> Option<&super::super::type_level::trait_bounds::TraitDefinition> {
        self.trait_table.get_trait(name)
    }

    /// 检查 Trait 是否已定义
    pub fn has_trait(
        &self,
        name: &str,
    ) -> bool {
        self.trait_table.has_trait(name)
    }

    /// 添加 Trait 实现
    pub fn add_trait_impl(
        &mut self,
        impl_: super::super::type_level::trait_bounds::TraitImplementation,
    ) {
        self.trait_table.add_impl(impl_);
    }

    /// 检查类型是否实现了 Trait
    pub fn has_trait_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> bool {
        self.trait_table.has_impl(trait_name, for_type)
    }

    /// 获取 Trait 实现
    pub fn get_trait_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> Option<&super::super::type_level::trait_bounds::TraitImplementation> {
        self.trait_table.get_impl(trait_name, for_type)
    }

    /// 注册 native 函数签名
    pub fn add_native_signature(
        &mut self,
        name: &str,
        sig: MonoType,
    ) {
        self.native_signatures.insert(name.to_string(), sig);
    }

    /// 获取 native 函数签名
    pub fn get_native_signature(
        &self,
        name: &str,
    ) -> Option<&MonoType> {
        self.native_signatures.get(name)
    }

    /// 检查是否是已注册的 native 函数
    pub fn is_native_function(
        &self,
        name: &str,
    ) -> bool {
        self.native_signatures.contains_key(name)
    }
}
