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
pub mod constraint;
pub mod cross_module;
pub mod dce; // 死代码消除
pub mod function;
pub mod global;
pub mod instance;
pub mod instantiation_graph; // 实例化图
pub mod module_state;
pub mod platform_info; // 平台信息获取
pub mod platform_specializer; // 平台特化器
pub mod reachability; // 可达性分析
#[cfg(test)]
pub mod tests;
pub mod type_mono;

use self::instance::{
    ClosureId, ClosureInstance, ClosureSpecializationKey, FunctionId, GenericClosureId,
    GenericFunctionId, GenericTypeId, InstantiationRequest, SpecializationKey, TypeId,
    TypeInstance,
};

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{FunctionIR, ModuleIR};

use self::function::FunctionMonomorphizer;
use self::type_mono::TypeMonomorphizer;

use self::dce::{DceConfig, DcePass};
use self::platform_info::{PlatformDetector, PlatformInfo};
use self::platform_specializer::{PlatformConstraint, SpecializationDecider};

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

    /// ==================== DCE 相关 ====================

    /// DCE 配置
    dce_config: DceConfig,

    /// DCE 统计信息（可访问）
    dce_stats: Option<dce::DceStats>,

    /// ==================== 平台特化相关 ====================

    /// 当前目标平台信息
    platform_info: PlatformInfo,

    /// 平台特化决策器
    specialization_decider: SpecializationDecider,

    /// 函数平台约束映射（函数名 -> 平台约束）
    /// 用于在实例化时检查是否应该实例化特定平台的版本
    function_platform_constraints: HashMap<String, Option<PlatformConstraint>>,
}

impl Monomorphizer {
    /// 创建新的单态化器（使用检测到的平台）
    pub fn new() -> Self {
        Self::with_platform(PlatformDetector::detect_from_env())
    }

    /// 创建指定平台的单态化器
    pub fn with_platform(platform_info: PlatformInfo) -> Self {
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
            // DCE 相关字段
            dce_config: DceConfig::default(),
            dce_stats: None,
            // 平台特化相关字段
            platform_info: platform_info.clone(),
            specialization_decider: SpecializationDecider::new(&platform_info),
            function_platform_constraints: HashMap::new(),
        }
    }

    /// 获取当前平台信息
    pub fn platform_info(&self) -> &PlatformInfo {
        &self.platform_info
    }

    /// 设置目标平台
    pub fn set_target_platform(
        &mut self,
        platform_info: PlatformInfo,
    ) {
        self.platform_info = platform_info.clone();
        self.specialization_decider = SpecializationDecider::new(&platform_info);
    }

    /// 创建带配置的单态化器
    pub fn with_dce_config(config: DceConfig) -> Self {
        let platform_info = PlatformDetector::detect_from_env();
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
            // DCE 相关字段
            dce_config: config,
            dce_stats: None,
            // 平台特化相关字段
            platform_info: platform_info.clone(),
            specialization_decider: SpecializationDecider::new(&platform_info),
            function_platform_constraints: HashMap::new(),
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

        // 执行 DCE（死代码消除）
        self.run_dce(module)
    }

    /// 运行 DCE（死代码消除）
    ///
    /// 在单态化完成后，消除未使用的泛型实例
    fn run_dce(
        &mut self,
        module: &ModuleIR,
    ) -> ModuleIR {
        if !self.dce_config.enabled {
            return self.build_output_module(module);
        }

        // 确定入口点
        let entry_points = self.find_entry_points();

        // 创建 DCE Pass
        let mut dce_pass = DcePass::new(self.dce_config.clone());

        // 运行 DCE，获取保留的实例
        let kept_functions = self.instantiated_functions.clone();
        let kept_types: HashMap<TypeId, MonoType> = self
            .type_instances
            .iter()
            .filter_map(|(id, inst)| inst.get_mono_type().map(|ty| (id.clone(), ty.clone())))
            .collect();

        let result = dce_pass.run_on_module(
            module,
            &kept_functions,
            &kept_types,
            &entry_points,
            &self.generic_functions,
        );

        // 保存统计信息
        self.dce_stats = Some(result.stats);

        // 使用 DCE 过滤后的结果构建输出模块
        self.build_output_module_with_filtered_instances(
            module,
            result.kept_functions,
            result.kept_types,
        )
    }

    /// 查找入口点函数
    fn find_entry_points(&self) -> Vec<FunctionId> {
        let mut entries = Vec::new();

        // 查找 main 函数
        if let Some((id, _)) = self
            .instantiated_functions
            .iter()
            .find(|(id, _)| id.name() == "main")
        {
            entries.push(id.clone());
        }

        // 查找其他入口点（导出的函数）
        // TODO: 实现更完整的入口点检测

        entries
    }

    /// 使用过滤后的实例构建输出模块
    fn build_output_module_with_filtered_instances(
        &self,
        original_module: &ModuleIR,
        kept_functions: HashMap<FunctionId, FunctionIR>,
        _kept_types: HashMap<TypeId, MonoType>,
    ) -> ModuleIR {
        let mut output_funcs: Vec<FunctionIR> = original_module
            .functions
            .iter()
            .filter(|f| !self.is_generic_function(f))
            .cloned()
            .collect();

        // 添加保留下来的实例化函数
        for func in kept_functions.values() {
            output_funcs.push(func.clone());
        }

        ModuleIR {
            types: original_module.types.clone(),
            globals: original_module.globals.clone(),
            functions: output_funcs,
        }
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

    // ==================== DCE 相关 API ====================

    /// 获取 DCE 统计信息
    pub fn dce_stats(&self) -> Option<&dce::DceStats> {
        self.dce_stats.as_ref()
    }

    /// 设置 DCE 配置
    pub fn set_dce_config(
        &mut self,
        config: DceConfig,
    ) {
        self.dce_config = config;
    }

    /// 获取当前 DCE 配置
    pub fn dce_config(&self) -> &DceConfig {
        &self.dce_config
    }

    /// 启用 DCE
    pub fn enable_dce(&mut self) {
        self.dce_config.enabled = true;
    }

    /// 禁用 DCE
    pub fn disable_dce(&mut self) {
        self.dce_config.enabled = false;
    }

    // ==================== Send/Sync 约束驱动单态化 ====================

    /// 根据 Send/Sync 约束单态化泛型函数
    ///
    /// # Arguments
    ///
    /// * `generic_id` - 泛型函数ID
    /// * `type_args` - 类型参数
    /// * `send_constraints` - Send 约束（类型变量 -> 是否必须 Send）
    /// * `sync_constraints` - Sync 约束（类型变量 -> 是否必须 Sync）
    ///
    /// # Returns
    ///
    /// 如果约束可以满足，返回特化后的函数ID；否则返回 None
    pub fn monomorphize_with_send_sync_constraints(
        &mut self,
        generic_id: &GenericFunctionId,
        type_args: &[MonoType],
        send_constraints: &[(MonoType, bool)],
        sync_constraints: &[(MonoType, bool)],
    ) -> Option<FunctionId> {
        // 1. 检查约束是否可以满足
        if !self.can_satisfy_send_sync_constraints(type_args, send_constraints, sync_constraints) {
            return None;
        }

        // 2. 生成特化后的类型参数（根据约束调整）
        let specialized_types =
            self.apply_send_sync_constraints(type_args, send_constraints, sync_constraints);

        // 3. 执行单态化
        self.monomorphize_function(generic_id, &specialized_types)
    }

    /// 检查是否可以满足 Send/Sync 约束
    fn can_satisfy_send_sync_constraints(
        &self,
        _type_args: &[MonoType],
        send_constraints: &[(MonoType, bool)],
        sync_constraints: &[(MonoType, bool)],
    ) -> bool {
        // 检查 Send 约束
        for (ty, require_send) in send_constraints {
            if *require_send && !self.is_type_send(ty) {
                return false;
            }
        }

        // 检查 Sync 约束
        for (ty, require_sync) in sync_constraints {
            if *require_sync && !self.is_type_sync(ty) {
                return false;
            }
        }

        true
    }

    /// 检查类型是否满足 Send（复用 SendSyncChecker 的逻辑）
    fn is_type_send(
        &self,
        ty: &MonoType,
    ) -> bool {
        use crate::middle::passes::lifetime::send_sync::SendSyncChecker;
        let checker = SendSyncChecker::new();
        checker.is_send(ty)
    }

    /// 检查类型是否满足 Sync
    fn is_type_sync(
        &self,
        ty: &MonoType,
    ) -> bool {
        use crate::middle::passes::lifetime::send_sync::SendSyncChecker;
        let checker = SendSyncChecker::new();
        checker.is_sync(ty)
    }

    /// 根据 Send/Sync 约束调整类型参数
    ///
    /// 对于无法 Send 的类型，如果需要 Send 版本，会生成 Arc 包装
    fn apply_send_sync_constraints(
        &self,
        type_args: &[MonoType],
        _send_constraints: &[(MonoType, bool)],
        _sync_constraints: &[(MonoType, bool)],
    ) -> Vec<MonoType> {
        // 当前实现直接返回原始类型
        // 未来可以扩展：对于需要 Arc 包装的类型，生成 Arc<T> 类型
        type_args.to_vec()
    }

    /// 为泛型函数生成 Send 特化版本
    ///
    /// 当泛型函数用于 spawn 时，生成 Send 版本
    pub fn generate_send_specialization(
        &mut self,
        generic_id: &GenericFunctionId,
        type_args: &[MonoType],
    ) -> Option<FunctionId> {
        // 构造 Send 约束
        let send_constraints: Vec<(MonoType, bool)> =
            type_args.iter().map(|ty| (ty.clone(), true)).collect();

        self.monomorphize_with_send_sync_constraints(generic_id, type_args, &send_constraints, &[])
    }

    /// 为泛型函数生成 Sync 特化版本
    ///
    /// 当泛型函数需要跨线程共享引用时，生成 Sync 版本
    pub fn generate_sync_specialization(
        &mut self,
        generic_id: &GenericFunctionId,
        type_args: &[MonoType],
    ) -> Option<FunctionId> {
        // 构造 Sync 约束
        let sync_constraints: Vec<(MonoType, bool)> =
            type_args.iter().map(|ty| (ty.clone(), true)).collect();

        self.monomorphize_with_send_sync_constraints(generic_id, type_args, &[], &sync_constraints)
    }

    /// 为泛型函数生成 Send + Sync 特化版本
    pub fn generate_send_sync_specialization(
        &mut self,
        generic_id: &GenericFunctionId,
        type_args: &[MonoType],
    ) -> Option<FunctionId> {
        let send_constraints: Vec<(MonoType, bool)> =
            type_args.iter().map(|ty| (ty.clone(), true)).collect();
        let sync_constraints = send_constraints.clone();

        self.monomorphize_with_send_sync_constraints(
            generic_id,
            type_args,
            &send_constraints,
            &sync_constraints,
        )
    }
}

#[cfg(test)]
impl Monomorphizer {
    /// 测试辅助：插入泛型函数
    pub fn test_insert_generic_function(
        &mut self,
        id: GenericFunctionId,
        ir: FunctionIR,
    ) {
        self.generic_functions.insert(id, ir);
    }

    /// 测试辅助：插入泛型类型
    pub fn test_insert_generic_type(
        &mut self,
        id: GenericTypeId,
        ty: MonoType,
    ) {
        self.generic_types.insert(id, ty);
    }

    /// 测试辅助：获取泛型函数映射（用于断言）
    pub fn test_get_generic_functions(&self) -> &HashMap<GenericFunctionId, FunctionIR> {
        &self.generic_functions
    }

    /// 测试辅助：获取泛型类型映射（用于断言）
    pub fn test_get_generic_types(&self) -> &HashMap<GenericTypeId, MonoType> {
        &self.generic_types
    }

    /// 测试辅助：清空队列（用于测试）
    pub fn test_clear_queue(&mut self) {
        self.instantiation_queue.clear();
    }

    /// 测试辅助：获取特化缓存大小
    pub fn test_specialization_cache_len(&self) -> usize {
        self.specialization_cache.len()
    }

    /// 测试辅助：访问type_instances
    pub fn test_type_instances(&self) -> &HashMap<TypeId, TypeInstance> {
        &self.type_instances
    }
}

impl Default for Monomorphizer {
    fn default() -> Self {
        Self::new()
    }
}
