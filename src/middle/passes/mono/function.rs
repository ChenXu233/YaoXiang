//! 函数单态化子模块
//!
//! 提供函数单态化相关的辅助函数和trait

use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{BasicBlock, FunctionBody, FunctionIR, LocalSlot, ModuleIR};
use crate::middle::passes::mono::instance::{FunctionId, InstantiationRequest};
use std::collections::HashMap;

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
        func.generic_params.is_some()
    }

    fn contains_type_var(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            MonoType::TypeVar(_) => true,
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
        func.generic_params.clone().unwrap_or_default()
    }

    fn instantiate_function(
        &mut self,
        request: &InstantiationRequest,
    ) -> Option<FunctionId> {
        let key = request.specialization_key();

        if self.processed.contains(&key) {
            return None;
        }

        let generic_id = request.generic_id();
        let generic_func = self.generic_functions.get(generic_id.name())?;

        let type_args = request.type_args.clone();
        let specialized_name = Self::generate_specialized_name(generic_id.name(), &type_args);
        let func_id = FunctionId::new(specialized_name.clone(), type_args);

        let specialized_func = self.substitute_types(generic_func, &func_id, &request.type_args);

        self.processed.insert(key);
        self.specialized_functions
            .insert(specialized_name, specialized_func);

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
        let new_locals: Vec<LocalSlot> = match &generic_func.body {
            FunctionBody::Code { locals, .. } => locals
                .iter()
                .map(|slot| LocalSlot {
                    name: slot.name.clone(),
                    ty: self.substitute_single_type(&slot.ty, &type_param_map),
                    scope_depth: slot.scope_depth,
                })
                .collect(),
            _ => Vec::new(),
        };
        // 指令里已没有需要改写的 ast::Type 载荷（6.6.11 删了 Cast/TypeTest 两个
        // 死变体），块级改写退化为原样克隆——不再有逐指令映射这一步。
        let new_blocks: Vec<BasicBlock> = match &generic_func.body {
            FunctionBody::Code { blocks, .. } => blocks.clone(),
            _ => Vec::new(),
        };
        let new_entry = match &generic_func.body {
            FunctionBody::Code { entry, .. } => *entry,
            _ => 0,
        };

        FunctionIR {
            name: func_id.name().to_string(),
            // 特化函数是泛型的新名字实例，不复用原 DefId——mono 路径保持按名分发
            def: None,
            params: new_params,
            return_type: new_return_type,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: new_blocks,
                entry: new_entry,
                locals: new_locals,
            },
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
            MonoType::Fn {
                params,
                return_type,
            } => MonoType::Fn {
                params: params
                    .iter()
                    .map(|t| self.substitute_single_type(t, type_map))
                    .collect(),
                return_type: Box::new(self.substitute_single_type(return_type, type_map)),
            },
            MonoType::Generic { name, args } => MonoType::Generic {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| self.substitute_single_type(a, type_map))
                    .collect(),
            },
            _ => ty.clone(),
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
        for func in self.specialized_functions.values() {
            output_funcs.push(func.clone());
        }
        ModuleIR {
            globals: original_module.globals.clone(),
            functions: output_funcs,
            init: original_module.init.clone(),
            init_file_ids: original_module.init_file_ids.clone(),
            init_locals: original_module.init_locals.clone(),
            ffi_libs: original_module.ffi_libs.clone(),
            ffi_bindings: original_module.ffi_bindings.clone(),
            entry_function: original_module.entry_function.clone(),
            source_files: original_module.source_files.clone(),
            function_files: original_module.function_files.clone(),
        }
    }
}
