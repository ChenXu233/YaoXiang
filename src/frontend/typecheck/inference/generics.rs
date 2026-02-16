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
    next_type_var: usize,
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
            next_type_var: 0,
        }
    }

    fn fresh_type_var(&mut self) -> MonoType {
        let var = crate::frontend::core::type_system::var::TypeVar::new(self.next_type_var);
        self.next_type_var += 1;
        MonoType::TypeVar(var)
    }

    /// 推断泛型函数类型
    pub fn infer_generic_function(
        &mut self,
        _name: &str,
        type_params: &[String],
    ) -> Result<MonoType> {
        // 为每个泛型参数分配独立类型变量，避免后续推断时发生索引碰撞。
        for _ in type_params {
            let _ = self.fresh_type_var();
        }
        Ok(self.fresh_type_var())
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
        args: &[MonoType],
    ) -> Result<MonoType> {
        if args.len() == 1 {
            Ok(args[0].clone())
        } else {
            Ok(self.fresh_type_var())
        }
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

#[cfg(test)]
mod tests {
    use super::GenericInferrer;
    use crate::frontend::core::type_system::MonoType;

    #[test]
    fn test_infer_generic_function_creates_fresh_vars() {
        let mut inferrer = GenericInferrer::new();

        let t1 = inferrer
            .infer_generic_function("f", &["T".to_string()])
            .unwrap();
        let t2 = inferrer
            .infer_generic_function("g", &["U".to_string()])
            .unwrap();

        match (t1, t2) {
            (MonoType::TypeVar(v1), MonoType::TypeVar(v2)) => {
                assert_ne!(v1, v2);
            }
            _ => panic!("Expected type variables"),
        }
    }
}
