//! 单态化器
//!
//! 将泛型函数和泛型类型特化为具体类型的代码。
//! 核心策略：
//! 1. 按需特化：只对实际使用的类型组合生成代码
//! 2. 代码共享：相同类型组合共享一份代码
//! 3. 类型单态化：支持泛型结构和枚举的类型实例化

use std::collections::HashMap;

// 导出子模块
pub mod closure;
pub mod function;
pub mod global;
pub mod instance;
pub mod module_state;
pub mod type_mono;

#[cfg(test)]
pub mod tests;

use self::instance::{
    ClosureId, ClosureInstance, ClosureSpecializationKey, FunctionId, GenericClosureId,
    GenericFunctionId, GenericTypeId, InstantiationRequest, SpecializationKey, TypeId,
    TypeInstance,
};

use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{FunctionIR, ModuleIR};

use self::function::FunctionMonomorphizer;
use self::type_mono::TypeMonomorphizer;

/// 单态化器
#[derive(Debug)]
pub struct Monomorphizer {
    /// 已实例化的函数
    instantiated_functions: HashMap<FunctionId, FunctionIR>,

    /// 待实例化的泛型函数队列
    instantiation_queue: Vec<InstantiationRequest>,

    /// 特化缓存：避免重复实例化
    specialization_cache: HashMap<SpecializationKey, FunctionId>,

    /// 泛型函数集合
    generic_functions: HashMap<GenericFunctionId, FunctionIR>,

    /// 当前特化计数器
    next_function_id: usize,

    /// ==================== 类型单态化相关 ====================

    /// 已实例化的类型
    type_instances: HashMap<TypeId, TypeInstance>,

    /// 类型特化缓存：避免重复实例化类型
    type_specialization_cache: HashMap<SpecializationKey, TypeId>,

    /// 泛型类型集合（存储原始类型定义）
    generic_types: HashMap<GenericTypeId, MonoType>,

    /// 下一个类型ID计数器
    next_type_id: usize,

    /// ==================== 闭包单态化相关 ====================

    /// 已实例化的闭包
    instantiated_closures: HashMap<ClosureId, ClosureInstance>,

    /// 闭包特化缓存：避免重复实例化闭包
    closure_specialization_cache: HashMap<ClosureSpecializationKey, ClosureId>,

    /// 泛型闭包集合
    generic_closures: HashMap<GenericClosureId, ClosureInstance>,

    /// 下一个闭包ID计数器
    next_closure_id: usize,
}

impl Monomorphizer {
    /// 创建新的单态化器
    pub fn new() -> Self {
        Monomorphizer {
            instantiated_functions: HashMap::new(),
            instantiation_queue: Vec::new(),
            specialization_cache: HashMap::new(),
            generic_functions: HashMap::new(),
            next_function_id: 0,
            // 类型单态化相关字段
            type_instances: HashMap::new(),
            type_specialization_cache: HashMap::new(),
            generic_types: HashMap::new(),
            next_type_id: 0,
            // 闭包单态化相关字段
            instantiated_closures: HashMap::new(),
            closure_specialization_cache: HashMap::new(),
            generic_closures: HashMap::new(),
            next_closure_id: 0,
        }
    }

    /// 单态化模块中的所有泛型函数和泛型类型
    pub fn monomorphize_module(
        &mut self,
        module: &ModuleIR,
    ) -> ModuleIR {
        self.collect_generic_functions(module);
        self.collect_generic_types(module);
        self.collect_instantiation_requests(module);
        self.process_instantiation_queue();
        self.build_output_module(module)
    }

    /// 收集所有泛型函数
    fn collect_generic_functions(
        &mut self,
        module: &ModuleIR,
    ) {
        for func in &module.functions {
            if self.is_generic_function(func) {
                let generic_id =
                    GenericFunctionId::new(func.name.clone(), self.extract_type_params(func));
                self.generic_functions.insert(generic_id, func.clone());
            }
        }
    }

    /// 获取已实例化的函数数量
    pub fn instantiated_count(&self) -> usize {
        self.instantiated_functions.len()
    }

    /// 获取泛型函数数量
    pub fn generic_count(&self) -> usize {
        self.generic_functions.len()
    }

    // ==================== 函数单态化公开 API ====================

    /// 单态化泛型函数
    pub fn monomorphize_function(
        &mut self,
        generic_id: &GenericFunctionId,
        type_args: &[MonoType],
    ) -> Option<FunctionId> {
        let cache_key = SpecializationKey::new(generic_id.name().to_string(), type_args.to_vec());

        if let Some(id) = self.specialization_cache.get(&cache_key) {
            return Some(id.clone());
        }

        if !self.generic_functions.contains_key(generic_id) {
            return None;
        }

        if !self.should_specialize(generic_id) {
            return None;
        }

        let request = InstantiationRequest::new(
            generic_id.clone(),
            type_args.to_vec(),
            crate::util::span::Span::default(),
        );

        self.instantiate_function(&request)
    }

    /// 检查泛型函数是否已单态化
    pub fn is_function_monomorphized(
        &self,
        generic_id: &GenericFunctionId,
        type_args: &[MonoType],
    ) -> bool {
        let cache_key = SpecializationKey::new(generic_id.name().to_string(), type_args.to_vec());
        self.specialization_cache.contains_key(&cache_key)
    }

    /// 获取函数实例（如果已实例化）
    pub fn get_instantiated_function(
        &self,
        func_id: &FunctionId,
    ) -> Option<&FunctionIR> {
        self.instantiated_functions.get(func_id)
    }

    /// 获取已单态化的函数数量
    pub fn instantiated_function_count(&self) -> usize {
        self.instantiated_functions.len()
    }
}

impl Default for Monomorphizer {
    fn default() -> Self {
        Self::new()
    }
}
