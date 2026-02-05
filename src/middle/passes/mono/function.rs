//! 函数单态化子模块
//!
//! 提供函数单态化相关的辅助函数和trait

use crate::frontend::core::parser::ast::Type as AstType;
use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::middle::passes::mono::instance::{FunctionId, GenericFunctionId, InstantiationRequest};
use crate::middle::passes::mono::platform_specializer::PlatformConstraint;
use std::collections::{HashMap, HashSet};

/// 函数单态化相关trait
pub trait FunctionMonomorphizer {
    /// 检查函数是否是泛型函数
    fn is_generic_function(
        &self,
        func: &FunctionIR,
    ) -> bool;

    /// 检查类型是否包含类型变量
    fn contains_type_var(
        &self,
        ty: &MonoType,
    ) -> bool;

    /// 提取函数的类型参数
    fn extract_type_params(
        &self,
        func: &FunctionIR,
    ) -> Vec<String>;

    /// 收集所有实例化请求
    fn collect_instantiation_requests(
        &mut self,
        module: &ModuleIR,
    );

    /// 从指令中收集函数调用类型
    fn collect_instruction_types(
        &self,
        instr: &Instruction,
        all_call_type_names: &mut HashSet<String>,
        all_generic_calls: &mut Vec<(String, Vec<MonoType>)>,
    );

    /// 将类型列表转换为唯一键字符串
    fn types_to_key(types: &[MonoType]) -> String;

    /// 从类型名字符串解析回MonoType列表
    fn parse_type_names(key: &str) -> Vec<MonoType>;

    /// 将类型名转换为MonoType
    fn type_name_to_mono_type(name: &str) -> Option<MonoType>;

    /// 将操作数转换为类型
    fn operand_to_type(
        &self,
        operand: &Operand,
    ) -> Option<MonoType>;

    /// 根据收集到的类型参数为泛型函数排队实例化请求
    fn queue_instantiations_for_types(
        &mut self,
        type_args: &[MonoType],
        generic_calls: &[(String, Vec<MonoType>)],
    );

    /// 添加实例化请求
    fn add_instantiation_request(
        &mut self,
        generic_id: GenericFunctionId,
        type_args: Vec<MonoType>,
    );

    /// 处理实例化队列
    fn process_instantiation_queue(&mut self);

    /// 检查是否应该特化
    fn should_specialize(
        &self,
        generic_id: &GenericFunctionId,
    ) -> bool;

    /// 获取函数的平台约束
    fn get_function_platform_constraint(
        &self,
        func_name: &str,
    ) -> Option<&PlatformConstraint>;

    /// 实例化单个函数
    fn instantiate_function(
        &mut self,
        request: &InstantiationRequest,
    ) -> Option<FunctionId>;

    /// 生成特化函数名称
    fn generate_specialized_name(
        base_name: &str,
        type_args: &[MonoType],
    ) -> String;

    /// 类型替换
    fn substitute_types(
        &self,
        generic_func: &FunctionIR,
        func_id: &FunctionId,
        type_args: &[MonoType],
    ) -> FunctionIR;

    /// 单个类型替换
    fn substitute_single_type(
        &self,
        ty: &MonoType,
        type_map: &HashMap<usize, MonoType>,
    ) -> MonoType;

    /// 替换基本块中的指令
    fn substitute_block(
        &self,
        block: &BasicBlock,
        type_map: &HashMap<usize, MonoType>,
    ) -> BasicBlock;

    /// 替换指令中的类型
    fn substitute_instruction(
        &self,
        instr: &Instruction,
        type_map: &HashMap<usize, MonoType>,
    ) -> Instruction;

    /// 替换AST类型
    fn substitute_type_ast(
        &self,
        ty: &AstType,
        type_map: &HashMap<usize, MonoType>,
    ) -> AstType;

    /// 构建输出模块
    fn build_output_module(
        &self,
        original_module: &ModuleIR,
    ) -> ModuleIR;
}

/// 函数单态化器的默认实现
#[allow(clippy::only_used_in_recursion)]
impl FunctionMonomorphizer for super::Monomorphizer {
    fn is_generic_function(
        &self,
        func: &FunctionIR,
    ) -> bool {
        for param_ty in &func.params {
            if matches!(param_ty, MonoType::TypeVar(_)) {
                return true;
            }
        }
        if matches!(func.return_type, MonoType::TypeVar(_)) {
            return true;
        }
        for local_ty in &func.locals {
            if self.contains_type_var(local_ty) {
                return true;
            }
        }
        false
    }

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

    fn extract_type_params(
        &self,
        func: &FunctionIR,
    ) -> Vec<String> {
        let mut type_params = Vec::new();
        let mut seen = HashSet::new();

        for param_ty in &func.params {
            if let MonoType::TypeVar(tv) = param_ty {
                let name = format!("T{}", tv.index());
                if seen.insert(name.clone()) {
                    type_params.push(name);
                }
            }
        }

        if let MonoType::TypeVar(tv) = &func.return_type {
            let name = format!("T{}", tv.index());
            if seen.insert(name.clone()) {
                type_params.push(name);
            }
        }

        type_params
    }

    fn collect_instantiation_requests(
        &mut self,
        module: &ModuleIR,
    ) {
        let mut all_call_type_names: HashSet<String> = HashSet::new();
        let mut all_generic_calls: Vec<(String, Vec<MonoType>)> = Vec::new();

        for func in &module.functions {
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

        for type_names in &all_call_type_names {
            let type_args = Self::parse_type_names(type_names);
            self.queue_instantiations_for_types(&type_args, &all_generic_calls);
        }
    }

    fn collect_instruction_types(
        &self,
        instr: &Instruction,
        all_call_type_names: &mut HashSet<String>,
        all_generic_calls: &mut Vec<(String, Vec<MonoType>)>,
    ) {
        match instr {
            Instruction::Call { func, args, .. } => {
                let arg_types: Vec<MonoType> = args
                    .iter()
                    .filter_map(|a| self.operand_to_type(a))
                    .collect();

                if !arg_types.is_empty() {
                    let type_key = Self::types_to_key(&arg_types);
                    all_call_type_names.insert(type_key);

                    if let Operand::Global(func_idx) = func {
                        let func_name = format!("func_{}", func_idx);
                        all_generic_calls.push((func_name, arg_types));
                    } else if let Operand::Const(ConstValue::String(name)) = func {
                        all_generic_calls.push((name.clone(), arg_types));
                    }
                }
            }

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

            Instruction::Ret(Some(operand)) => {
                if let Some(ty) = self.operand_to_type(operand) {
                    let type_key = Self::types_to_key(&[ty]);
                    all_call_type_names.insert(type_key);
                }
            }
            Instruction::Ret(None) => {}

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

            Instruction::Load { dst, .. } => {
                if let Some(ty) = self.operand_to_type(dst) {
                    let type_key = Self::types_to_key(&[ty]);
                    all_call_type_names.insert(type_key);
                }
            }

            Instruction::Alloc { dst, .. } => {
                if let Some(ty) = self.operand_to_type(dst) {
                    let type_key = Self::types_to_key(&[ty]);
                    all_call_type_names.insert(type_key);
                }
            }

            _ => {}
        }
    }

    fn types_to_key(types: &[MonoType]) -> String {
        types
            .iter()
            .map(|t| t.type_name())
            .collect::<Vec<_>>()
            .join(",")
    }

    fn parse_type_names(key: &str) -> Vec<MonoType> {
        if key.is_empty() {
            return Vec::new();
        }
        key.split(',')
            .filter_map(Self::type_name_to_mono_type)
            .collect()
    }

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

    fn operand_to_type(
        &self,
        operand: &Operand,
    ) -> Option<MonoType> {
        match operand {
            Operand::Local(_id) => Some(MonoType::Int(64)),
            Operand::Temp(_id) => Some(MonoType::Int(64)),
            Operand::Arg(_id) => Some(MonoType::Int(64)),
            Operand::Global(_id) => Some(MonoType::Int(64)),
            Operand::Const(ConstValue::Int(_)) => Some(MonoType::Int(64)),
            Operand::Const(ConstValue::Float(_)) => Some(MonoType::Float(64)),
            Operand::Const(ConstValue::Bool(_)) => Some(MonoType::Bool),
            Operand::Const(ConstValue::String(_)) => Some(MonoType::String),
            Operand::Const(ConstValue::Char(_)) => Some(MonoType::Char),
            Operand::Const(ConstValue::Void) => Some(MonoType::Void),
            _ => None,
        }
    }

    fn queue_instantiations_for_types(
        &mut self,
        type_args: &[MonoType],
        _generic_calls: &[(String, Vec<MonoType>)],
    ) {
        for generic_id in self.generic_functions.keys() {
            let type_param_count = generic_id.type_params().len();
            if type_param_count > 0 && type_args.len() >= type_param_count {
                let matching_args: Vec<MonoType> = type_args[..type_param_count].to_vec();
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

    fn add_instantiation_request(
        &mut self,
        generic_id: GenericFunctionId,
        type_args: Vec<MonoType>,
    ) {
        let request =
            InstantiationRequest::new(generic_id, type_args, crate::util::span::Span::default());
        self.instantiation_queue.push(request);
    }

    fn process_instantiation_queue(&mut self) {
        while let Some(request) = self.instantiation_queue.pop() {
            if !self.should_specialize(&request.generic_id) {
                continue;
            }
            self.instantiate_function(&request);
        }
    }

    fn should_specialize(
        &self,
        generic_id: &GenericFunctionId,
    ) -> bool {
        // 检查函数是否有平台约束
        let constraint = self.get_function_platform_constraint(generic_id.name());

        match constraint {
            Some(platform_constraint) => {
                // 有平台约束，使用决策器判断
                let decision = self.specialization_decider.decide(platform_constraint);
                decision.should_specialize()
            }
            None => {
                // 没有平台约束，始终实例化（通用版本）
                true
            }
        }
    }

    /// 获取函数的平台约束
    fn get_function_platform_constraint(
        &self,
        func_name: &str,
    ) -> Option<&PlatformConstraint> {
        self.function_platform_constraints
            .get(func_name)
            .and_then(|c| c.as_ref())
    }

    fn instantiate_function(
        &mut self,
        request: &InstantiationRequest,
    ) -> Option<FunctionId> {
        let key = request.specialization_key();

        if let Some(id) = self.specialization_cache.get(&key) {
            return Some(id.clone());
        }

        let generic_id = request.generic_id();
        let generic_func = self.generic_functions.get(generic_id)?;

        let type_args = request.type_args.clone();
        let specialized_name = Self::generate_specialized_name(generic_id.name(), &type_args);
        let func_id = FunctionId::new(specialized_name.clone(), type_args);

        if !self.should_specialize(generic_id) {
            return None;
        }

        let specialized_func = self.substitute_types(generic_func, &func_id, &request.type_args);

        self.specialization_cache.insert(key, func_id.clone());
        self.instantiated_functions
            .insert(func_id.clone(), specialized_func);

        Some(func_id)
    }

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

    fn substitute_types(
        &self,
        generic_func: &FunctionIR,
        func_id: &FunctionId,
        type_args: &[MonoType],
    ) -> FunctionIR {
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

        let new_params: Vec<MonoType> = generic_func
            .params
            .iter()
            .map(|ty| self.substitute_single_type(ty, &type_param_map))
            .collect();
        let new_return_type =
            self.substitute_single_type(&generic_func.return_type, &type_param_map);
        let new_locals: Vec<MonoType> = generic_func
            .locals
            .iter()
            .map(|ty| self.substitute_single_type(ty, &type_param_map))
            .collect();
        let new_blocks: Vec<BasicBlock> = generic_func
            .blocks
            .iter()
            .map(|block| self.substitute_block(block, &type_param_map))
            .collect();

        FunctionIR {
            name: func_id.name().to_string(),
            params: new_params,
            return_type: new_return_type,
            is_async: generic_func.is_async,
            locals: new_locals,
            blocks: new_blocks,
            entry: generic_func.entry,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn substitute_single_type(
        &self,
        ty: &MonoType,
        type_map: &HashMap<usize, MonoType>,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(tv) => type_map
                .get(&tv.index())
                .cloned()
                .unwrap_or_else(|| ty.clone()),
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

    fn substitute_type_ast(
        &self,
        ty: &AstType,
        type_map: &HashMap<usize, MonoType>,
    ) -> AstType {
        match ty {
            // 基本类型直接返回
            AstType::Name(_)
            | AstType::Int(_)
            | AstType::Float(_)
            | AstType::Char
            | AstType::String
            | AstType::Bytes
            | AstType::Bool
            | AstType::Void
            | AstType::Enum(_) => ty.clone(),

            // 结构体：递归替换字段类型
            AstType::Struct(fields) => AstType::Struct(
                fields
                    .iter()
                    .map(|f| crate::frontend::core::parser::ast::StructField {
                        name: f.name.clone(),
                        is_mut: f.is_mut,
                        ty: self.substitute_type_ast(&f.ty, type_map),
                    })
                    .collect(),
            ),

            // 命名结构体
            AstType::NamedStruct { name, fields } => AstType::NamedStruct {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|f| crate::frontend::core::parser::ast::StructField {
                        name: f.name.clone(),
                        is_mut: f.is_mut,
                        ty: self.substitute_type_ast(&f.ty, type_map),
                    })
                    .collect(),
            },

            // 联合类型
            AstType::Union(members) => AstType::Union(
                members
                    .iter()
                    .map(|(name, ty)| {
                        (
                            name.clone(),
                            ty.as_ref().map(|t| self.substitute_type_ast(t, type_map)),
                        )
                    })
                    .collect(),
            ),

            // 变体类型
            AstType::Variant(variants) => AstType::Variant(
                variants
                    .iter()
                    .map(|v| crate::frontend::core::parser::ast::VariantDef {
                        name: v.name.clone(),
                        params: v
                            .params
                            .iter()
                            .map(|(n, t)| (n.clone(), self.substitute_type_ast(t, type_map)))
                            .collect(),
                        span: v.span,
                    })
                    .collect(),
            ),

            // 元组：递归替换元素类型
            AstType::Tuple(types) => AstType::Tuple(
                types
                    .iter()
                    .map(|t| self.substitute_type_ast(t, type_map))
                    .collect(),
            ),

            // 列表：替换元素类型
            AstType::List(elem) => {
                AstType::List(Box::new(self.substitute_type_ast(elem, type_map)))
            }

            // 字典：替换键值类型
            AstType::Dict(key, value) => AstType::Dict(
                Box::new(self.substitute_type_ast(key, type_map)),
                Box::new(self.substitute_type_ast(value, type_map)),
            ),

            // 集合：替换元素类型
            AstType::Set(elem) => AstType::Set(Box::new(self.substitute_type_ast(elem, type_map))),

            // 函数类型：替换参数和返回类型
            AstType::Fn {
                params,
                return_type,
            } => AstType::Fn {
                params: params
                    .iter()
                    .map(|t| self.substitute_type_ast(t, type_map))
                    .collect(),
                return_type: Box::new(self.substitute_type_ast(return_type, type_map)),
            },

            // Option：替换内部类型
            AstType::Option(inner) => {
                AstType::Option(Box::new(self.substitute_type_ast(inner, type_map)))
            }

            // Result：替换 Ok 和 Err 类型
            AstType::Result(ok, err) => AstType::Result(
                Box::new(self.substitute_type_ast(ok, type_map)),
                Box::new(self.substitute_type_ast(err, type_map)),
            ),

            // 泛型类型：替换类型参数
            AstType::Generic { name, args } => AstType::Generic {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|t| self.substitute_type_ast(t, type_map))
                    .collect(),
            },

            // 关联类型：递归替换
            AstType::AssocType {
                host_type,
                assoc_name,
                assoc_args,
            } => AstType::AssocType {
                host_type: Box::new(self.substitute_type_ast(host_type, type_map)),
                assoc_name: assoc_name.clone(),
                assoc_args: assoc_args
                    .iter()
                    .map(|t| self.substitute_type_ast(t, type_map))
                    .collect(),
            },

            // Sum 类型
            AstType::Sum(types) => AstType::Sum(
                types
                    .iter()
                    .map(|t| self.substitute_type_ast(t, type_map))
                    .collect(),
            ),

            // 字面量类型：替换基础类型
            AstType::Literal { name, base_type } => AstType::Literal {
                name: name.clone(),
                base_type: Box::new(self.substitute_type_ast(base_type, type_map)),
            },
        }
    }

    fn build_output_module(
        &self,
        original_module: &ModuleIR,
    ) -> ModuleIR {
        let mut output_funcs: Vec<FunctionIR> = original_module
            .functions
            .iter()
            .filter(|f| !self.is_generic_function(f))
            .cloned()
            .collect();
        for func in self.instantiated_functions.values() {
            output_funcs.push(func.clone());
        }
        ModuleIR {
            types: original_module.types.clone(),
            globals: original_module.globals.clone(),
            functions: output_funcs,
        }
    }
}
