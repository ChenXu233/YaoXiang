#![allow(clippy::result_large_err)]

//! RFC-011 泛型推断
//!
//! 实现泛型函数的类型推断

use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use crate::frontend::core::type_system::MonoType;
use crate::frontend::typecheck::checking::bounds::BoundsChecker;
use crate::util::span::Span;

/// 泛型推断器
pub struct GenericInferrer {
    bounds_checker: BoundsChecker,
}

impl Default for GenericInferrer {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericInferrer {
    /// 创建新的泛型推断器
    pub fn new() -> Self {
        Self {
            bounds_checker: BoundsChecker::new(),
        }
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

    /// 检查泛型约束
    ///
    /// 在泛型函数实例化时，检查实际类型是否满足约束
    /// 约束格式：[T: ConstraintName](item: T)
    pub fn check_type_constraint(
        &mut self,
        actual_type: &MonoType,
        constraint_type: &MonoType,
        span: Span,
    ) -> Result<()> {
        self.bounds_checker
            .check_constraint(actual_type, constraint_type)
            .map_err(|e| {
                ErrorCodeDefinition::trait_bound_not_satisfied(&e.type_name, &e.constraint_name)
                    .at(span)
                    .build()
            })
    }
}
