//! 单态化器
//!
//! 将泛型函数特化为具体类型的函数。
//! 核心策略：
//! 1. 按需特化：只对实际调用的类型组合生成代码
//! 2. 代码共享：相同类型组合共享一份代码
//! 3. 阈值控制：单函数特化数量上限为16

use crate::frontend::parser::ast::Type;
use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use std::collections::{HashMap, HashSet};

// 导出 instance 模块
pub mod instance;

use self::instance::{FunctionId, GenericFunctionId, InstantiationRequest, SpecializationKey};

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

    /// 单函数特化数量上限
    specialization_limit: usize,

    /// 当前特化计数器
    next_function_id: usize,
}

impl Monomorphizer {
    /// 创建新的单态化器
    pub fn new() -> Self {
        Monomorphizer {
            instantiated_functions: HashMap::new(),
            instantiation_queue: Vec::new(),
            specialization_cache: HashMap::new(),
            generic_functions: HashMap::new(),
            specialization_limit: 16,
            next_function_id: 0,
        }
    }

    /// 单态化模块中的所有泛型函数
    pub fn monomorphize_module(&mut self, module: &ModuleIR) -> ModuleIR {
        // 1. 收集所有泛型函数
        self.collect_generic_functions(module);

        // 2. 收集所有实例化请求
        self.collect_instantiation_requests(module);

        // 3. 处理实例化队列
        self.process_instantiation_queue();

        // 4. 生成最终模块
        self.build_output_module(module)
    }

    /// 收集所有泛型函数
    fn collect_generic_functions(&mut self, module: &ModuleIR) {
        for func in &module.functions {
            if self.is_generic_function(func) {
                let generic_id =
                    GenericFunctionId::new(func.name.clone(), self.extract_type_params(func));
                self.generic_functions.insert(generic_id, func.clone());
            }
        }
    }

    /// 检查函数是否是泛型函数
    fn is_generic_function(&self, func: &FunctionIR) -> bool {
        // 检查参数类型或返回类型是否包含类型变量
        for param_ty in &func.params {
            if matches!(param_ty, MonoType::TypeVar(_)) {
                return true;
            }
        }
        if matches!(func.return_type, MonoType::TypeVar(_)) {
            return true;
        }
        // 检查局部变量类型是否包含类型变量
        for local_ty in &func.locals {
            if self.contains_type_var(local_ty) {
                return true;
            }
        }
        false
    }

    /// 检查类型是否包含类型变量
    #[allow(clippy::only_used_in_recursion)]
    fn contains_type_var(&self, ty: &MonoType) -> bool {
        match ty {
            MonoType::TypeVar(_) => true,
            MonoType::List(elem) => self.contains_type_var(elem),
            MonoType::Dict(key, value) => {
                self.contains_type_var(key) || self.contains_type_var(value)
            }
            MonoType::Set(elem) => self.contains_type_var(elem),
            MonoType::Tuple(types) => types.iter().any(|t| self.contains_type_var(t)),
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                params.iter().any(|t| self.contains_type_var(t))
                    || self.contains_type_var(return_type)
            }
            _ => false,
        }
    }

    /// 提取函数的类型参数
    fn extract_type_params(&self, func: &FunctionIR) -> Vec<String> {
        let mut type_params = Vec::new();
        let mut seen = HashSet::new();

        // 从参数类型收集类型变量
        for param_ty in &func.params {
            if let MonoType::TypeVar(tv) = param_ty {
                let name = format!("T{}", tv.index());
                if seen.insert(name.clone()) {
                    type_params.push(name);
                }
            }
        }

        // 从返回类型收集类型变量
        if let MonoType::TypeVar(tv) = &func.return_type {
            let name = format!("T{}", tv.index());
            if seen.insert(name.clone()) {
                type_params.push(name);
            }
        }

        type_params
    }

    /// 收集所有实例化请求
    ///
    /// 遍历 IR 中所有函数调用，收集实际使用的类型参数，
    /// 为每个泛型函数生成实例化请求。
    fn collect_instantiation_requests(&mut self, module: &ModuleIR) {
        // 收集所有函数调用中的类型使用（使用 type_name 作为键来避免 Hash 约束）
        let mut all_call_type_names: HashSet<String> = HashSet::new();
        let mut all_generic_calls: Vec<(String, Vec<MonoType>)> = Vec::new();

        // 遍历所有函数收集调用类型
        for func in &module.functions {
            // 从函数体中收集
            for block in &func.blocks {
                for instr in &block.instructions {
                    self.collect_instruction_types(
                        instr,
                        &mut all_call_type_names,
                        &mut all_generic_calls,
                    );
                }
            }
        }

        // 为每组收集到的类型添加实例化请求
        for type_names in &all_call_type_names {
            // 解析类型名字符串回 MonoType
            let type_args = Self::parse_type_names(type_names);
            self.queue_instantiations_for_types(&type_args, &all_generic_calls);
        }
    }

    /// 从指令中收集函数调用类型
    fn collect_instruction_types(
        &self,
        instr: &Instruction,
        all_call_type_names: &mut HashSet<String>,
        all_generic_calls: &mut Vec<(String, Vec<MonoType>)>,
    ) {
        match instr {
            // 函数调用：收集参数类型
            Instruction::Call { func, args, .. } => {
                // 收集参数类型
                let arg_types: Vec<MonoType> = args
                    .iter()
                    .filter_map(|a| self.operand_to_type(a))
                    .collect();

                if !arg_types.is_empty() {
                    let type_key = Self::types_to_key(&arg_types);
                    all_call_type_names.insert(type_key);

                    // 尝试识别被调用的泛型函数
                    if let Operand::Global(func_idx) = func {
                        let func_name = format!("func_{}", func_idx);
                        all_generic_calls.push((func_name, arg_types));
                    } else if let Operand::Const(ConstValue::String(name)) = func {
                        all_generic_calls.push((name.clone(), arg_types));
                    }
                }
            }

            // 尾调用
            Instruction::TailCall { func: _, args } => {
                let arg_types: Vec<MonoType> = args
                    .iter()
                    .filter_map(|a| self.operand_to_type(a))
                    .collect();
                if !arg_types.is_empty() {
                    let type_key = Self::types_to_key(&arg_types);
                    all_call_type_names.insert(type_key);
                }
            }

            // 返回值可能包含类型信息
            Instruction::Ret(Some(operand)) => {
                if let Some(ty) = self.operand_to_type(operand) {
                    let type_key = Self::types_to_key(&[ty]);
                    all_call_type_names.insert(type_key);
                }
            }
            Instruction::Ret(None) => {}

            // 移动指令：传播类型信息
            Instruction::Move { dst, src } => {
                if let (Some(dst_ty), Some(src_ty)) =
                    (self.operand_to_type(dst), self.operand_to_type(src))
                {
                    if dst_ty != src_ty {
                        let type_key = Self::types_to_key(&[dst_ty]);
                        all_call_type_names.insert(type_key);
                    }
                }
            }

            // 加载指令：收集加载的类型
            Instruction::Load { dst, .. } => {
                if let Some(ty) = self.operand_to_type(dst) {
                    let type_key = Self::types_to_key(&[ty]);
                    all_call_type_names.insert(type_key);
                }
            }

            // 分配指令：收集分配的类型
            Instruction::Alloc { dst, .. } => {
                if let Some(ty) = self.operand_to_type(dst) {
                    let type_key = Self::types_to_key(&[ty]);
                    all_call_type_names.insert(type_key);
                }
            }

            _ => {}
        }
    }

    /// 将类型列表转换为唯一键字符串（避免 Hash 约束问题）
    fn types_to_key(types: &[MonoType]) -> String {
        types
            .iter()
            .map(|t| t.type_name())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// 从类型名字符串解析回 MonoType 列表
    fn parse_type_names(key: &str) -> Vec<MonoType> {
        if key.is_empty() {
            return Vec::new();
        }
        key.split(',')
            .filter_map(Self::type_name_to_mono_type)
            .collect()
    }

    /// 将类型名转换为 MonoType
    fn type_name_to_mono_type(name: &str) -> Option<MonoType> {
        match name {
            "Void" => Some(MonoType::Void),
            "Bool" => Some(MonoType::Bool),
            "Int64" => Some(MonoType::Int(64)),
            "Int32" => Some(MonoType::Int(32)),
            "Int16" => Some(MonoType::Int(16)),
            "Int8" => Some(MonoType::Int(8)),
            "Float64" => Some(MonoType::Float(64)),
            "Float32" => Some(MonoType::Float(32)),
            "Char" => Some(MonoType::Char),
            "String" => Some(MonoType::String),
            "Bytes" => Some(MonoType::Bytes),
            _ => None,
        }
    }

    /// 将操作数转换为类型
    fn operand_to_type(&self, operand: &Operand) -> Option<MonoType> {
        match operand {
            Operand::Local(_id) => {
                // 尝试从局部变量类型映射获取
                // 当前简化版本返回一个默认类型
                Some(MonoType::Int(64))
            }
            Operand::Temp(_id) => Some(MonoType::Int(64)),
            Operand::Arg(_id) => Some(MonoType::Int(64)),
            Operand::Global(_id) => {
                // 全局变量类型
                Some(MonoType::Int(64))
            }
            Operand::Const(ConstValue::Int(_)) => Some(MonoType::Int(64)),
            Operand::Const(ConstValue::Float(_)) => Some(MonoType::Float(64)),
            Operand::Const(ConstValue::Bool(_)) => Some(MonoType::Bool),
            Operand::Const(ConstValue::String(_)) => Some(MonoType::String),
            Operand::Const(ConstValue::Char(_)) => Some(MonoType::Char),
            Operand::Const(ConstValue::Void) => Some(MonoType::Void),
            _ => None,
        }
    }

    /// 根据收集到的类型参数为泛型函数排队实例化请求
    fn queue_instantiations_for_types(
        &mut self,
        type_args: &[MonoType],
        _generic_calls: &[(String, Vec<MonoType>)],
    ) {
        // 遍历所有泛型函数，尝试匹配类型参数
        for generic_id in self.generic_functions.keys() {
            let type_param_count = generic_id.type_params().len();

            // 如果类型参数数量匹配，添加实例化请求
            if type_param_count > 0 && type_args.len() >= type_param_count {
                // 截取匹配的参数数量
                let matching_args: Vec<MonoType> = type_args[..type_param_count].to_vec();

                // 检查是否应该特化
                if self.should_specialize(generic_id) {
                    let request = InstantiationRequest::new(
                        generic_id.clone(),
                        matching_args,
                        crate::util::span::Span::default(),
                    );
                    self.instantiation_queue.push(request);
                }
            }
        }
    }

    /// 添加实例化请求
    pub fn add_instantiation_request(
        &mut self,
        generic_id: GenericFunctionId,
        type_args: Vec<MonoType>,
    ) {
        let request =
            InstantiationRequest::new(generic_id, type_args, crate::util::span::Span::default());
        self.instantiation_queue.push(request);
    }

    /// 处理实例化队列
    fn process_instantiation_queue(&mut self) {
        while let Some(request) = self.instantiation_queue.pop() {
            // 检查是否应该特化
            if !self.should_specialize(&request.generic_id) {
                continue;
            }

            // 生成特化函数
            self.instantiate_function(&request);
        }
    }

    /// 检查是否应该特化
    fn should_specialize(&self, generic_id: &GenericFunctionId) -> bool {
        let count = self
            .specialization_cache
            .keys()
            .filter(|key| key.name == generic_id.name())
            .count();
        count < self.specialization_limit
    }

    /// 实例化单个函数
    fn instantiate_function(&mut self, request: &InstantiationRequest) -> Option<FunctionId> {
        let key = request.specialization_key();

        // 检查缓存
        if let Some(id) = self.specialization_cache.get(&key) {
            return Some(id.clone());
        }

        // 获取泛型函数
        let generic_id = request.generic_id();
        let generic_func = self.generic_functions.get(generic_id)?;

        // 生成特化函数ID
        let type_args = request.type_args.clone();
        let specialized_name = Self::generate_specialized_name(generic_id.name(), &type_args);
        let func_id = FunctionId::new(specialized_name.clone(), type_args);

        // 检查是否已达到上限
        if !self.should_specialize(generic_id) {
            return None;
        }

        // 执行类型替换生成特化函数
        let specialized_func = self.substitute_types(generic_func, &func_id, &request.type_args);

        // 缓存和存储
        self.specialization_cache.insert(key, func_id.clone());
        self.instantiated_functions
            .insert(func_id.clone(), specialized_func);

        Some(func_id)
    }

    /// 生成特化函数名称
    fn generate_specialized_name(base_name: &str, type_args: &[MonoType]) -> String {
        if type_args.is_empty() {
            base_name.to_string()
        } else {
            let args_str = type_args
                .iter()
                .map(|t| {
                    t.type_name()
                        .replace("/", "_")
                        .replace("<", "_")
                        .replace(">", "_")
                })
                .collect::<Vec<_>>()
                .join("_");
            format!("{}_{}", base_name, args_str)
        }
    }

    /// 类型替换
    fn substitute_types(
        &self,
        generic_func: &FunctionIR,
        func_id: &FunctionId,
        type_args: &[MonoType],
    ) -> FunctionIR {
        // 构建类型参数映射
        let type_param_map: HashMap<usize, MonoType> = generic_func
            .params
            .iter()
            .enumerate()
            .filter_map(|(idx, ty)| {
                if let MonoType::TypeVar(tv) = ty {
                    if idx < type_args.len() {
                        return Some((tv.index(), type_args[idx].clone()));
                    }
                }
                None
            })
            .collect();

        // 替换参数类型
        let new_params: Vec<MonoType> = generic_func
            .params
            .iter()
            .map(|ty| self.substitute_single_type(ty, &type_param_map))
            .collect();

        // 替换返回类型
        let new_return_type =
            self.substitute_single_type(&generic_func.return_type, &type_param_map);

        // 替换局部变量类型
        let new_locals: Vec<MonoType> = generic_func
            .locals
            .iter()
            .map(|ty| self.substitute_single_type(ty, &type_param_map))
            .collect();

        // 复制基本块并替换指令中的类型
        let new_blocks: Vec<BasicBlock> = generic_func
            .blocks
            .iter()
            .map(|block| self.substitute_block(block, &type_param_map))
            .collect();

        FunctionIR {
            name: func_id.specialized_name(),
            params: new_params,
            return_type: new_return_type,
            is_async: generic_func.is_async,
            locals: new_locals,
            blocks: new_blocks,
            entry: generic_func.entry,
        }
    }

    /// 单个类型替换
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_single_type(
        &self,
        ty: &MonoType,
        type_map: &HashMap<usize, MonoType>,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(tv) => {
                if let Some(replacement) = type_map.get(&tv.index()) {
                    replacement.clone()
                } else {
                    ty.clone()
                }
            }
            MonoType::List(elem) => {
                MonoType::List(Box::new(self.substitute_single_type(elem, type_map)))
            }
            MonoType::Dict(key, value) => MonoType::Dict(
                Box::new(self.substitute_single_type(key, type_map)),
                Box::new(self.substitute_single_type(value, type_map)),
            ),
            MonoType::Set(elem) => {
                MonoType::Set(Box::new(self.substitute_single_type(elem, type_map)))
            }
            MonoType::Tuple(types) => MonoType::Tuple(
                types
                    .iter()
                    .map(|t| self.substitute_single_type(t, type_map))
                    .collect(),
            ),
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params
                    .iter()
                    .map(|t| self.substitute_single_type(t, type_map))
                    .collect(),
                return_type: Box::new(self.substitute_single_type(return_type, type_map)),
                is_async: *is_async,
            },
            _ => ty.clone(),
        }
    }

    /// 替换基本块中的指令
    fn substitute_block(
        &self,
        block: &BasicBlock,
        type_map: &HashMap<usize, MonoType>,
    ) -> BasicBlock {
        let new_instructions: Vec<Instruction> = block
            .instructions
            .iter()
            .map(|instr| self.substitute_instruction(instr, type_map))
            .collect();

        BasicBlock {
            label: block.label,
            instructions: new_instructions,
            successors: block.successors.clone(),
        }
    }

    /// 替换指令中的类型
    fn substitute_instruction(
        &self,
        instr: &Instruction,
        type_map: &HashMap<usize, MonoType>,
    ) -> Instruction {
        match instr {
            Instruction::Cast {
                dst,
                src,
                target_type,
            } => {
                let new_target = self.substitute_type_ast(target_type, type_map);
                Instruction::Cast {
                    dst: dst.clone(),
                    src: src.clone(),
                    target_type: new_target,
                }
            }
            Instruction::TypeTest(operand, test_type) => {
                let new_test_type = self.substitute_type_ast(test_type, type_map);
                Instruction::TypeTest(operand.clone(), new_test_type)
            }
            _ => instr.clone(),
        }
    }

    /// 替换 AST 类型
    fn substitute_type_ast(&self, ty: &Type, _type_map: &HashMap<usize, MonoType>) -> Type {
        // TODO: 实现完整的类型替换
        // 当前简化版本直接返回原类型
        ty.clone()
    }

    /// 构建输出模块
    fn build_output_module(&self, original_module: &ModuleIR) -> ModuleIR {
        // 复制非泛型函数
        let mut output_funcs: Vec<FunctionIR> = original_module
            .functions
            .iter()
            .filter(|f| !self.is_generic_function(f))
            .cloned()
            .collect();

        // 添加特化函数
        for func in self.instantiated_functions.values() {
            output_funcs.push(func.clone());
        }

        ModuleIR {
            types: original_module.types.clone(),
            constants: original_module.constants.clone(),
            globals: original_module.globals.clone(),
            functions: output_funcs,
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
}

impl Default for Monomorphizer {
    fn default() -> Self {
        Self::new()
    }
}
