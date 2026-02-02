#![allow(clippy::result_large_err)]

//! RFC-011 泛型推断
//!
//! 实现泛型函数的类型推断

use crate::util::diagnostic::Result;
use crate::frontend::core::type_system::MonoType;

/// 泛型推断器
pub struct GenericInferrer;

impl Default for GenericInferrer {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericInferrer {
    /// 创建新的泛型推断器
    pub fn new() -> Self {
        Self
    }

    /// 推断泛型函数类型
    pub fn infer_generic_function(
        &mut self,
        _name: &str,
        _type_params: &[String],
    ) -> Result<MonoType> {
        Ok(MonoType::TypeVar(
            crate::frontend::core::type_system::var::TypeVar::new(0),
        ))
    }

    /// 推断泛型约束
    pub fn infer_generic_constraints(
        &mut self,
        _constraints: &[String],
    ) -> Result<()> {
        Ok(())
    }

    /// 推断泛型实例化
    pub fn infer_generic_instantiation(
        &mut self,
        _generic: &str,
        _args: &[MonoType],
    ) -> Result<MonoType> {
        Ok(MonoType::TypeVar(
            crate::frontend::core::type_system::var::TypeVar::new(0),
        ))
    }
}
