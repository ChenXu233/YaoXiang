//! 全局单态化器
//!
//! 核心职责：
//! 1. 类型替换逻辑（供跨模块和模块内使用）
//! 2. 基本块/指令类型替换
//! 3. 特化名称生成

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, FunctionIR, Instruction};
use std::collections::HashMap;

/// 类型替换（用于函数）
pub fn substitute_types_in_function(
    generic_func: &FunctionIR,
    type_args: Vec<MonoType>,
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
        .map(|ty| substitute_single_type(ty, &type_param_map))
        .collect();

    let new_return_type = substitute_single_type(&generic_func.return_type, &type_param_map);

    let new_locals: Vec<MonoType> = generic_func
        .locals
        .iter()
        .map(|ty| substitute_single_type(ty, &type_param_map))
        .collect();

    let new_blocks: Vec<BasicBlock> = generic_func
        .blocks
        .iter()
        .map(|block| substitute_block(block, &type_param_map))
        .collect();

    FunctionIR {
        name: generate_specialized_name(&generic_func.name, &type_args),
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
pub fn substitute_single_type(
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
        MonoType::List(elem) => MonoType::List(Box::new(substitute_single_type(elem, type_map))),
        MonoType::Dict(key, value) => MonoType::Dict(
            Box::new(substitute_single_type(key, type_map)),
            Box::new(substitute_single_type(value, type_map)),
        ),
        MonoType::Set(elem) => MonoType::Set(Box::new(substitute_single_type(elem, type_map))),
        MonoType::Tuple(types) => MonoType::Tuple(
            types
                .iter()
                .map(|t| substitute_single_type(t, type_map))
                .collect(),
        ),
        MonoType::Fn {
            params,
            return_type,
            is_async,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|t| substitute_single_type(t, type_map))
                .collect(),
            return_type: Box::new(substitute_single_type(return_type, type_map)),
            is_async: *is_async,
        },
        MonoType::Arc(inner) => MonoType::Arc(Box::new(substitute_single_type(inner, type_map))),
        MonoType::Range { elem_type } => MonoType::Range {
            elem_type: Box::new(substitute_single_type(elem_type, type_map)),
        },
        MonoType::Union(types) | MonoType::Intersection(types) => {
            let substituted: Vec<MonoType> = types
                .iter()
                .map(|ty| substitute_single_type(ty, type_map))
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

/// 替换基本块中的指令
pub fn substitute_block(
    block: &BasicBlock,
    type_map: &HashMap<usize, MonoType>,
) -> BasicBlock {
    let new_instructions: Vec<Instruction> = block
        .instructions
        .iter()
        .map(|instr| substitute_instruction(instr, type_map))
        .collect();

    BasicBlock {
        label: block.label,
        instructions: new_instructions,
        successors: block.successors.clone(),
    }
}

/// 替换指令中的类型
pub fn substitute_instruction(
    instr: &Instruction,
    _type_map: &HashMap<usize, MonoType>,
) -> Instruction {
    instr.clone()
}

/// 生成特化函数名称
pub fn generate_specialized_name(
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
