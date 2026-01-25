//! 闭包单态化子模块
//!
//! 提供闭包单态化相关的辅助函数和trait

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::FunctionIR;
use crate::middle::passes::mono::instance::{
    ClosureId, ClosureInstance, ClosureSpecializationKey, GenericClosureId,
};
use super::function::FunctionMonomorphizer;
use std::collections::HashMap;

/// 闭包单态化相关trait
pub trait ClosureMonomorphizer {
    /// 单态化泛型闭包
    fn monomorphize_closure(
        &mut self,
        generic_id: &GenericClosureId,
        type_args: &[MonoType],
        capture_types: &[MonoType],
    ) -> Option<ClosureId>;

    /// 实例化闭包
    fn instantiate_closure(
        &self,
        generic_closure: &ClosureInstance,
        type_args: &[MonoType],
        capture_types: &[MonoType],
    ) -> Option<ClosureInstance>;

    /// 为闭包体执行类型替换
    fn substitute_types_for_closure(
        &self,
        generic_body: &FunctionIR,
        type_map: &HashMap<usize, MonoType>,
    ) -> FunctionIR;

    /// 检查闭包是否已单态化
    fn is_closure_monomorphized(
        &self,
        generic_id: &GenericClosureId,
        type_args: &[MonoType],
        capture_types: &[MonoType],
    ) -> bool;

    /// 获取已实例化的闭包
    fn get_instantiated_closure(
        &self,
        closure_id: &ClosureId,
    ) -> Option<&ClosureInstance>;

    /// 获取已单态化的闭包数量
    fn instantiated_closure_count(&self) -> usize;

    /// 注册泛型闭包
    fn register_generic_closure(
        &mut self,
        generic_id: GenericClosureId,
        closure: ClosureInstance,
    );
}

/// 闭包单态化器的默认实现
impl ClosureMonomorphizer for super::Monomorphizer {
    fn monomorphize_closure(
        &mut self,
        generic_id: &GenericClosureId,
        type_args: &[MonoType],
        capture_types: &[MonoType],
    ) -> Option<ClosureId> {
        let cache_key = ClosureSpecializationKey::new(
            generic_id.name().to_string(),
            type_args.to_vec(),
            capture_types.to_vec(),
        );

        if let Some(id) = self.closure_specialization_cache.get(&cache_key) {
            return Some(id.clone());
        }

        let generic_closure = self.generic_closures.get(generic_id)?;

        let closure_id = ClosureId::new(
            format!("closure_{}", self.next_closure_id),
            type_args.to_vec(),
            capture_types.to_vec(),
        );
        self.next_closure_id += 1;

        let mono_closure = self.instantiate_closure(generic_closure, type_args, capture_types)?;

        self.closure_specialization_cache
            .insert(cache_key.clone(), closure_id.clone());
        self.instantiated_closures
            .insert(closure_id.clone(), mono_closure);

        Some(closure_id)
    }

    fn instantiate_closure(
        &self,
        generic_closure: &ClosureInstance,
        type_args: &[MonoType],
        capture_types: &[MonoType],
    ) -> Option<ClosureInstance> {
        let type_params = generic_closure.generic_id.type_params();
        let type_param_map: HashMap<usize, MonoType> = type_params
            .iter()
            .enumerate()
            .filter_map(|(idx, _)| {
                if idx < type_args.len() {
                    Some((idx, type_args[idx].clone()))
                } else {
                    None
                }
            })
            .collect();

        let mono_body =
            self.substitute_types_for_closure(&generic_closure.body_ir, &type_param_map);

        let mono_captures: Vec<_> = generic_closure
            .capture_vars
            .iter()
            .map(|cap| {
                crate::middle::passes::mono::instance::CaptureVariable::new(
                    cap.name.clone(),
                    self.substitute_single_type(&cap.mono_type, &type_param_map),
                    cap.value.clone(),
                )
            })
            .collect();

        let closure_id = ClosureId::new(
            format!("closure_{}", self.next_closure_id - 1),
            type_args.to_vec(),
            capture_types.to_vec(),
        );

        Some(ClosureInstance::new(
            closure_id,
            generic_closure.generic_id.clone(),
            type_args.to_vec(),
            mono_captures,
            mono_body,
        ))
    }

    fn substitute_types_for_closure(
        &self,
        generic_body: &FunctionIR,
        type_map: &HashMap<usize, MonoType>,
    ) -> FunctionIR {
        let new_params: Vec<MonoType> = generic_body
            .params
            .iter()
            .map(|ty| self.substitute_single_type(ty, type_map))
            .collect();
        let new_return_type = self.substitute_single_type(&generic_body.return_type, type_map);
        let new_locals: Vec<MonoType> = generic_body
            .locals
            .iter()
            .map(|ty| self.substitute_single_type(ty, type_map))
            .collect();
        let new_blocks: Vec<_> = generic_body
            .blocks
            .iter()
            .map(|block| self.substitute_block(block, type_map))
            .collect();

        FunctionIR {
            name: format!("{}_mono", generic_body.name),
            params: new_params,
            return_type: new_return_type,
            is_async: generic_body.is_async,
            locals: new_locals,
            blocks: new_blocks,
            entry: generic_body.entry,
        }
    }

    fn is_closure_monomorphized(
        &self,
        generic_id: &GenericClosureId,
        type_args: &[MonoType],
        capture_types: &[MonoType],
    ) -> bool {
        let cache_key = ClosureSpecializationKey::new(
            generic_id.name().to_string(),
            type_args.to_vec(),
            capture_types.to_vec(),
        );
        self.closure_specialization_cache.contains_key(&cache_key)
    }

    fn get_instantiated_closure(
        &self,
        closure_id: &ClosureId,
    ) -> Option<&ClosureInstance> {
        self.instantiated_closures.get(closure_id)
    }

    fn instantiated_closure_count(&self) -> usize {
        self.instantiated_closures.len()
    }

    fn register_generic_closure(
        &mut self,
        generic_id: GenericClosureId,
        closure: ClosureInstance,
    ) {
        self.generic_closures.insert(generic_id, closure);
    }
}
