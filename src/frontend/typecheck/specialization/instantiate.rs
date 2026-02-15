#![allow(clippy::result_large_err)]

//! 实例化算法
//!
//! 实现泛型实例化，复用 core/type_system/substitute.rs 中的公共实现

use crate::util::diagnostic::Result;
use crate::frontend::core::type_system::{MonoType, Substituter};
use crate::frontend::core::type_system::substitute::contains_type_vars;

/// 实例化结果
#[derive(Debug, Clone)]
pub struct InstanceResult {
    pub instance: MonoType,
    pub generic: MonoType,
}

/// 实例化算法（复用公共替换器）
#[derive(Debug, Default)]
pub struct Instantiator(Substituter);

impl Instantiator {
    /// 创建新的实例化器
    pub fn new() -> Self {
        Self(Substituter::new())
    }

    /// 执行实例化
    pub fn instantiate(
        &self,
        generic: &MonoType,
        args: &[MonoType],
    ) -> Result<InstanceResult> {
        // 检查是否可以实例化
        if !self.can_instantiate(generic, args) {
            return Err(
                crate::util::diagnostic::ErrorCodeDefinition::cannot_instantiate_generic().build(),
            );
        }

        // 执行实例化：用具体类型替换泛型参数
        let instance = self.0.substitute_generic_params(generic, args);

        Ok(InstanceResult {
            instance,
            generic: generic.clone(),
        })
    }

    /// 检查是否可以实例化
    pub fn can_instantiate(
        &self,
        generic: &MonoType,
        args: &[MonoType],
    ) -> bool {
        // 检查泛型类型是否包含类型变量
        contains_type_vars(generic) && !args.is_empty()
    }
}
