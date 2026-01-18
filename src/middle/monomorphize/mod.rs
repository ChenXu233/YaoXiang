//! 单态化器
//!
//! 将泛型函数和泛型类型特化为具体类型的代码。
//! 核心策略：
//! 1. 按需特化：只对实际使用的类型组合生成代码
//! 2. 代码共享：相同类型组合共享一份代码
//! 3. 类型单态化：支持泛型结构和枚举的类型实例化

use crate::frontend::parser::ast::Type;
use crate::frontend::typecheck::{EnumType, MonoType, StructType};
use crate::middle::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use std::collections::{HashMap, HashSet};

// 导出 instance 模块
pub mod instance;

// 导出 tests 模块
#[cfg(test)]
pub mod tests;

use self::instance::{
    FunctionId, GenericFunctionId, GenericTypeId, InstantiationRequest, SpecializationKey, TypeId,
    TypeInstance,
};

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

    /// ==================== 类型单态化相关 ====================

    /// 已实例化的类型
    type_instances: HashMap<TypeId, TypeInstance>,

    /// 类型特化缓存：避免重复实例化类型
    type_specialization_cache: HashMap<SpecializationKey, TypeId>,

    /// 泛型类型集合（存储原始类型定义）
    generic_types: HashMap<GenericTypeId, MonoType>,

    /// 下一个类型ID计数器
    next_type_id: usize,
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
            // 类型单态化相关字段
            type_instances: HashMap::new(),
            type_specialization_cache: HashMap::new(),
            generic_types: HashMap::new(),
            next_type_id: 0,
        }
    }

    /// 单态化模块中的所有泛型函数和泛型类型
    pub fn monomorphize_module(
        &mut self,
        module: &ModuleIR,
    ) -> ModuleIR {
        // 1. 收集所有泛型函数
        self.collect_generic_functions(module);

        // 2. 收集所有泛型类型
        self.collect_generic_types(module);

        // 3. 收集所有实例化请求
        self.collect_instantiation_requests(module);

        // 4. 处理实例化队列
        self.process_instantiation_queue();

        // 5. 生成最终模块
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

    /// 检查函数是否是泛型函数
    fn is_generic_function(
        &self,
        func: &FunctionIR,
    ) -> bool {
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
    fn contains_type_var(
        &self,
        ty: &MonoType,
    ) -> bool {
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
    fn extract_type_params(
        &self,
        func: &FunctionIR,
    ) -> Vec<String> {
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
    fn collect_instantiation_requests(
        &mut self,
        module: &ModuleIR,
    ) {
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
    fn operand_to_type(
        &self,
        operand: &Operand,
    ) -> Option<MonoType> {
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
    fn should_specialize(
        &self,
        generic_id: &GenericFunctionId,
    ) -> bool {
        let count = self
            .specialization_cache
            .keys()
            .filter(|key| key.name == generic_id.name())
            .count();
        count < self.specialization_limit
    }

    /// 实例化单个函数
    fn instantiate_function(
        &mut self,
        request: &InstantiationRequest,
    ) -> Option<FunctionId> {
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
    fn generate_specialized_name(
        base_name: &str,
        type_args: &[MonoType],
    ) -> String {
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
    fn substitute_type_ast(
        &self,
        ty: &Type,
        _type_map: &HashMap<usize, MonoType>,
    ) -> Type {
        // TODO: 实现完整的类型替换
        // 当前简化版本直接返回原类型
        ty.clone()
    }

    /// 构建输出模块
    fn build_output_module(
        &self,
        original_module: &ModuleIR,
    ) -> ModuleIR {
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

    // ==================== 类型单态化方法 ====================

    /// 收集所有泛型类型定义
    fn collect_generic_types(
        &mut self,
        module: &ModuleIR,
    ) {
        for ty in &module.types {
            if self.contains_type_var_type(ty) {
                let type_params = self.extract_type_params_from_type(ty);
                let type_name = Self::get_type_name(ty);
                let generic_id = GenericTypeId::new(type_name, type_params);
                // 将 AST Type 转换为 MonoType
                let mono_type = self.type_to_mono_type(ty);
                self.generic_types.insert(generic_id, mono_type);
            }
        }
    }

    /// 将 AST Type 转换为 MonoType
    #[allow(clippy::only_used_in_recursion)]
    fn type_to_mono_type(
        &self,
        ty: &Type,
    ) -> MonoType {
        match ty {
            Type::Name(name) => MonoType::TypeRef(name.clone()),
            Type::Int(n) => MonoType::Int(*n),
            Type::Float(n) => MonoType::Float(*n),
            Type::Char => MonoType::Char,
            Type::String => MonoType::String,
            Type::Bytes => MonoType::Bytes,
            Type::Bool => MonoType::Bool,
            Type::Void => MonoType::Void,
            Type::Struct(fields) => MonoType::Struct(StructType {
                name: fields
                    .first()
                    .map(|(n, _)| n.clone())
                    .unwrap_or_else(|| "Struct".to_string()),
                fields: fields
                    .iter()
                    .map(|(n, ty)| (n.clone(), self.type_to_mono_type(ty)))
                    .collect(),
            }),
            Type::NamedStruct { name, fields } => MonoType::Struct(StructType {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|(n, ty)| (n.clone(), self.type_to_mono_type(ty)))
                    .collect(),
            }),
            Type::Union(variants) => MonoType::Union(
                variants
                    .iter()
                    .filter_map(|(_, ty)| ty.as_ref().map(|t| self.type_to_mono_type(t)))
                    .collect(),
            ),
            Type::Enum(variants) => MonoType::Enum(crate::frontend::typecheck::EnumType {
                name: variants
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Enum".to_string()),
                variants: variants.clone(),
            }),
            Type::Variant(_) => {
                // Variant is complex, represent as TypeRef for now
                MonoType::TypeRef("Variant".to_string())
            }
            Type::Tuple(types) => {
                MonoType::Tuple(types.iter().map(|t| self.type_to_mono_type(t)).collect())
            }
            Type::List(elem) => MonoType::List(Box::new(self.type_to_mono_type(elem))),
            Type::Dict(key, value) => MonoType::Dict(
                Box::new(self.type_to_mono_type(key)),
                Box::new(self.type_to_mono_type(value)),
            ),
            Type::Set(elem) => MonoType::Set(Box::new(self.type_to_mono_type(elem))),
            Type::Fn {
                params,
                return_type,
            } => MonoType::Fn {
                params: params.iter().map(|t| self.type_to_mono_type(t)).collect(),
                return_type: Box::new(self.type_to_mono_type(return_type)),
                is_async: false, // AST doesn't track async, assume sync
            },
            Type::Option(inner) => {
                // Option<T> as a special tagged type
                MonoType::Union(vec![self.type_to_mono_type(inner)])
            }
            Type::Result(_, _) => {
                // Result<T, E> as TypeRef for now
                MonoType::TypeRef("Result".to_string())
            }
            Type::Generic { name, args } => {
                let args_str = args
                    .iter()
                    .map(|t| self.type_to_mono_type(t).type_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                MonoType::TypeRef(format!("{}<{}>", name, args_str))
            }
            Type::Sum(types) => {
                MonoType::Union(types.iter().map(|t| self.type_to_mono_type(t)).collect())
            }
        }
    }

    /// 获取类型的名称
    fn get_type_name(ty: &Type) -> String {
        match ty {
            Type::Name(name) => name.clone(),
            Type::Int(n) => format!("int{}", n),
            Type::Float(n) => format!("float{}", n),
            Type::Char => "char".to_string(),
            Type::String => "string".to_string(),
            Type::Bytes => "bytes".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Void => "void".to_string(),
            Type::Struct(fields) => {
                if let Some((first_field, _)) = fields.first() {
                    first_field.clone()
                } else {
                    "Struct".to_string()
                }
            }
            Type::NamedStruct { name, .. } => name.clone(),
            Type::Union(variants) => {
                if let Some((first_variant, _)) = variants.first() {
                    first_variant.clone()
                } else {
                    "Union".to_string()
                }
            }
            Type::Enum(variants) => {
                if let Some(first_variant) = variants.first() {
                    first_variant.clone()
                } else {
                    "Enum".to_string()
                }
            }
            Type::Variant(variants) => {
                if let Some(first_variant) = variants.first() {
                    first_variant.name.clone()
                } else {
                    "Variant".to_string()
                }
            }
            Type::Tuple(types) => format!("tuple{}", types.len()),
            Type::List(_) => "List".to_string(),
            Type::Dict(_, _) => "Dict".to_string(),
            Type::Set(_) => "Set".to_string(),
            Type::Fn { .. } => "Fn".to_string(),
            Type::Option(_) => "Option".to_string(),
            Type::Result(_, _) => "Result".to_string(),
            Type::Generic { name, .. } => name.clone(),
            Type::Sum(_) => "Sum".to_string(),
        }
    }

    /// 检查类型是否包含类型变量（AST Type 版本）
    #[allow(clippy::only_used_in_recursion)]
    fn contains_type_var_type(
        &self,
        ty: &Type,
    ) -> bool {
        match ty {
            Type::Name(_) => false,
            Type::Int(_)
            | Type::Float(_)
            | Type::Char
            | Type::String
            | Type::Bytes
            | Type::Bool
            | Type::Void => false,
            Type::Struct(fields) | Type::NamedStruct { fields, .. } => fields
                .iter()
                .any(|(_, fty)| self.contains_type_var_type(fty)),
            Type::Union(variants) => variants
                .iter()
                .any(|(_, ty)| ty.as_ref().is_some_and(|t| self.contains_type_var_type(t))),
            Type::Enum(_) => false,
            Type::Variant(_) => false,
            Type::Tuple(types) => types.iter().any(|t| self.contains_type_var_type(t)),
            Type::List(elem) => self.contains_type_var_type(elem),
            Type::Dict(key, value) => {
                self.contains_type_var_type(key) || self.contains_type_var_type(value)
            }
            Type::Set(elem) => self.contains_type_var_type(elem),
            Type::Fn {
                params,
                return_type,
                ..
            } => {
                params.iter().any(|t| self.contains_type_var_type(t))
                    || self.contains_type_var_type(return_type)
            }
            Type::Option(inner) => self.contains_type_var_type(inner),
            Type::Result(ok, err) => {
                self.contains_type_var_type(ok) || self.contains_type_var_type(err)
            }
            Type::Generic { args, .. } => args.iter().any(|t| self.contains_type_var_type(t)),
            Type::Sum(types) => types.iter().any(|t| self.contains_type_var_type(t)),
        }
    }

    /// 从类型中提取类型参数（AST Type 版本）
    fn extract_type_params_from_type(
        &self,
        ty: &Type,
    ) -> Vec<String> {
        let mut type_params = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.collect_type_vars_from_type(ty, &mut type_params, &mut seen);
        type_params
    }

    /// 递归收集类型变量（AST Type 版本）
    #[allow(clippy::only_used_in_recursion)]
    fn collect_type_vars_from_type(
        &self,
        ty: &Type,
        type_params: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        match ty {
            Type::Name(name) => {
                // 假设大写字母开头的是类型参数
                if name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                    && seen.insert(name.clone())
                {
                    type_params.push(name.clone());
                }
            }
            Type::Struct(fields) | Type::NamedStruct { fields, .. } => {
                fields
                    .iter()
                    .for_each(|(_, fty)| self.collect_type_vars_from_type(fty, type_params, seen));
            }
            Type::Union(variants) => {
                variants.iter().for_each(|(_, ty)| {
                    if let Some(t) = ty {
                        self.collect_type_vars_from_type(t, type_params, seen);
                    }
                });
            }
            Type::Enum(_) => {}
            Type::Variant(_) => {}
            Type::Tuple(types) => types
                .iter()
                .for_each(|t| self.collect_type_vars_from_type(t, type_params, seen)),
            Type::List(elem) => self.collect_type_vars_from_type(elem, type_params, seen),
            Type::Dict(key, value) => {
                self.collect_type_vars_from_type(key, type_params, seen);
                self.collect_type_vars_from_type(value, type_params, seen);
            }
            Type::Set(elem) => self.collect_type_vars_from_type(elem, type_params, seen),
            Type::Fn {
                params,
                return_type,
                ..
            } => {
                params
                    .iter()
                    .for_each(|p| self.collect_type_vars_from_type(p, type_params, seen));
                self.collect_type_vars_from_type(return_type, type_params, seen);
            }
            Type::Option(inner) => self.collect_type_vars_from_type(inner, type_params, seen),
            Type::Result(ok, err) => {
                self.collect_type_vars_from_type(ok, type_params, seen);
                self.collect_type_vars_from_type(err, type_params, seen);
            }
            Type::Generic { args, .. } => args
                .iter()
                .for_each(|t| self.collect_type_vars_from_type(t, type_params, seen)),
            Type::Sum(types) => types
                .iter()
                .for_each(|t| self.collect_type_vars_from_type(t, type_params, seen)),
            Type::Int(_)
            | Type::Float(_)
            | Type::Char
            | Type::String
            | Type::Bytes
            | Type::Bool
            | Type::Void => {}
        }
    }

    /// 单态化泛型类型
    pub fn monomorphize_type(
        &mut self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
    ) -> Option<MonoType> {
        let cache_key = SpecializationKey::new(generic_id.name().to_string(), type_args.to_vec());
        if let Some(cached_id) = self.type_specialization_cache.get(&cache_key) {
            if let Some(instance) = self.type_instances.get(cached_id) {
                return instance.get_mono_type().cloned();
            }
        }

        let generic_type = self.generic_types.get(generic_id)?;
        let mono_type = self.instantiate_type(generic_id, type_args, generic_type)?;

        let type_id = self.generate_type_id(generic_id, type_args);
        let mut instance =
            TypeInstance::new(type_id.clone(), generic_id.clone(), type_args.to_vec());
        instance.set_mono_type(mono_type.clone());

        self.type_specialization_cache
            .insert(cache_key, type_id.clone());
        self.type_instances.insert(type_id, instance);

        Some(mono_type)
    }

    /// 实例化具体类型
    #[allow(clippy::only_used_in_recursion)]
    fn instantiate_type(
        &self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
        generic_type: &MonoType,
    ) -> Option<MonoType> {
        let type_params = generic_id.type_params().to_vec();

        match generic_type {
            MonoType::Struct(struct_type) => {
                let mono_fields: Vec<(String, MonoType)> = struct_type
                    .fields
                    .iter()
                    .map(|(name, ty)| {
                        (
                            name.clone(),
                            self.substitute_type_args(ty, type_args, &type_params),
                        )
                    })
                    .collect();
                Some(MonoType::Struct(StructType {
                    name: self.generate_type_name(generic_id, type_args),
                    fields: mono_fields,
                }))
            }
            MonoType::Enum(enum_type) => Some(MonoType::Enum(EnumType {
                name: self.generate_type_name(generic_id, type_args),
                variants: enum_type.variants.clone(),
            })),
            MonoType::List(elem) => Some(MonoType::List(Box::new(self.substitute_type_args(
                elem,
                type_args,
                &type_params,
            )))),
            MonoType::Dict(key, value) => Some(MonoType::Dict(
                Box::new(self.substitute_type_args(key, type_args, &type_params)),
                Box::new(self.substitute_type_args(value, type_args, &type_params)),
            )),
            MonoType::Set(elem) => Some(MonoType::Set(Box::new(self.substitute_type_args(
                elem,
                type_args,
                &type_params,
            )))),
            MonoType::Tuple(types) => Some(MonoType::Tuple(
                types
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, &type_params))
                    .collect(),
            )),
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => Some(MonoType::Fn {
                params: params
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, &type_params))
                    .collect(),
                return_type: Box::new(self.substitute_type_args(
                    return_type,
                    type_args,
                    &type_params,
                )),
                is_async: *is_async,
            }),
            MonoType::Arc(inner) => Some(MonoType::Arc(Box::new(self.substitute_type_args(
                inner,
                type_args,
                &type_params,
            )))),
            MonoType::Range { elem_type } => Some(MonoType::Range {
                elem_type: Box::new(self.substitute_type_args(elem_type, type_args, &type_params)),
            }),
            MonoType::Union(types) | MonoType::Intersection(types) => {
                let substituted: Vec<MonoType> = types
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, &type_params))
                    .collect();
                Some(if matches!(generic_type, MonoType::Union(_)) {
                    MonoType::Union(substituted)
                } else {
                    MonoType::Intersection(substituted)
                })
            }
            _ => Some(generic_type.clone()),
        }
    }

    /// 递归替换类型中的泛型参数
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_type_args(
        &self,
        ty: &MonoType,
        type_args: &[MonoType],
        type_params: &[String],
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(tv) => {
                let idx = tv.index();
                if idx < type_args.len() {
                    type_args[idx].clone()
                } else {
                    ty.clone()
                }
            }
            MonoType::Struct(struct_type) => MonoType::Struct(StructType {
                name: struct_type.name.clone(),
                fields: struct_type
                    .fields
                    .iter()
                    .map(|(name, field_ty)| {
                        (
                            name.clone(),
                            self.substitute_type_args(field_ty, type_args, type_params),
                        )
                    })
                    .collect(),
            }),
            MonoType::List(elem) => MonoType::List(Box::new(self.substitute_type_args(
                elem,
                type_args,
                type_params,
            ))),
            MonoType::Dict(key, value) => MonoType::Dict(
                Box::new(self.substitute_type_args(key, type_args, type_params)),
                Box::new(self.substitute_type_args(value, type_args, type_params)),
            ),
            MonoType::Set(elem) => MonoType::Set(Box::new(self.substitute_type_args(
                elem,
                type_args,
                type_params,
            ))),
            MonoType::Tuple(types) => MonoType::Tuple(
                types
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, type_params))
                    .collect(),
            ),
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, type_params))
                    .collect(),
                return_type: Box::new(self.substitute_type_args(
                    return_type,
                    type_args,
                    type_params,
                )),
                is_async: *is_async,
            },
            MonoType::Arc(inner) => MonoType::Arc(Box::new(self.substitute_type_args(
                inner,
                type_args,
                type_params,
            ))),
            MonoType::Range { elem_type } => MonoType::Range {
                elem_type: Box::new(self.substitute_type_args(elem_type, type_args, type_params)),
            },
            MonoType::Union(types) | MonoType::Intersection(types) => {
                let substituted: Vec<MonoType> = types
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, type_params))
                    .collect();
                if matches!(ty, MonoType::Union(_)) {
                    MonoType::Union(substituted)
                } else {
                    MonoType::Intersection(substituted)
                }
            }
            _ => ty.clone(),
        }
    }

    /// 生成类型ID
    fn generate_type_id(
        &self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
    ) -> TypeId {
        TypeId::new(
            self.generate_type_name(generic_id, type_args),
            type_args.to_vec(),
        )
    }

    /// 生成单态化类型名称
    fn generate_type_name(
        &self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
    ) -> String {
        if type_args.is_empty() {
            generic_id.name().to_string()
        } else {
            let args_str = type_args
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join("_");
            format!("{}_{}", generic_id.name(), args_str)
        }
    }

    /// 注册单态化后的类型
    pub fn register_monomorphized_type(
        &mut self,
        mono_type: MonoType,
    ) -> TypeId {
        let type_params = self.extract_type_params_from_mono_type(&mono_type);
        let type_id = TypeId::new(mono_type.type_name(), vec![]);
        let generic_id = GenericTypeId::new(mono_type.type_name(), type_params);
        let mut instance = TypeInstance::new(type_id.clone(), generic_id, vec![]);
        instance.set_mono_type(mono_type.clone());
        self.type_instances.insert(type_id.clone(), instance);
        type_id
    }

    /// 从 MonoType 提取类型参数
    fn extract_type_params_from_mono_type(
        &self,
        ty: &MonoType,
    ) -> Vec<String> {
        let mut type_params = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.collect_type_vars_from_mono_type(ty, &mut type_params, &mut seen);
        type_params
    }

    /// 递归收集 MonoType 中的类型变量
    #[allow(clippy::only_used_in_recursion)]
    fn collect_type_vars_from_mono_type(
        &self,
        ty: &MonoType,
        type_params: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        match ty {
            MonoType::TypeVar(tv) => {
                let name = format!("T{}", tv.index());
                if seen.insert(name.clone()) {
                    type_params.push(name);
                }
            }
            MonoType::Struct(struct_type) => {
                struct_type.fields.iter().for_each(|(_, field_ty)| {
                    self.collect_type_vars_from_mono_type(field_ty, type_params, seen);
                });
            }
            MonoType::Enum(_) => {}
            MonoType::Tuple(types) => {
                types
                    .iter()
                    .for_each(|t| self.collect_type_vars_from_mono_type(t, type_params, seen));
            }
            MonoType::List(elem) => {
                self.collect_type_vars_from_mono_type(elem, type_params, seen);
            }
            MonoType::Dict(key, value) => {
                self.collect_type_vars_from_mono_type(key, type_params, seen);
                self.collect_type_vars_from_mono_type(value, type_params, seen);
            }
            MonoType::Set(elem) => {
                self.collect_type_vars_from_mono_type(elem, type_params, seen);
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                params
                    .iter()
                    .for_each(|p| self.collect_type_vars_from_mono_type(p, type_params, seen));
                self.collect_type_vars_from_mono_type(return_type, type_params, seen);
            }
            MonoType::Range { elem_type } => {
                self.collect_type_vars_from_mono_type(elem_type, type_params, seen);
            }
            MonoType::TypeRef(_)
            | MonoType::Void
            | MonoType::Bool
            | MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes => {}
            MonoType::Union(types) | MonoType::Intersection(types) => {
                types
                    .iter()
                    .for_each(|t| self.collect_type_vars_from_mono_type(t, type_params, seen));
            }
            MonoType::Arc(inner) => {
                self.collect_type_vars_from_mono_type(inner, type_params, seen);
            }
        }
    }

    /// 获取已实例化的类型数量
    pub fn type_instance_count(&self) -> usize {
        self.type_instances.len()
    }

    /// 获取泛型类型数量
    pub fn generic_type_count(&self) -> usize {
        self.generic_types.len()
    }
}

impl Default for Monomorphizer {
    fn default() -> Self {
        Self::new()
    }
}
