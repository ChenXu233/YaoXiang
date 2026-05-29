#![allow(clippy::result_large_err)]

//! RFC-011 类型边界检查
//!
//! 检查泛型类型边界和约束
//! 支持鸭子类型：检查类型是否满足接口要求的所有方法（包括方法绑定）

use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use crate::frontend::core::types::base::MonoType;
use crate::frontend::core::typecheck::traits::solver::TraitSolver;
use crate::frontend::core::typecheck::traits::auto_derive::can_auto_derive_for_monotype;
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::util::span::Span;

/// 约束检查错误
#[derive(Debug, Clone)]
pub struct ConstraintCheckError {
    pub type_name: String,
    pub constraint_name: String,
    pub reason: String,
    pub span: Span,
}

/// 边界检查器
pub struct BoundsChecker {
    trait_solver: TraitSolver,
}

impl Default for BoundsChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundsChecker {
    /// 创建新的边界检查器
    pub fn new() -> Self {
        Self {
            trait_solver: TraitSolver::new(),
        }
    }

    /// 检查特质边界
    pub fn check_trait_bounds(
        &mut self,
        ty: &MonoType,
        bounds: &[String],
    ) -> Result<()> {
        // 检查每个边界
        for bound in bounds {
            if !self.trait_solver.check_trait(ty, bound) {
                // 尝试自动派生（仅对结构体类型）
                if let MonoType::Struct(s) = ty {
                    if let Some(trait_table) = self.trait_solver.trait_table() {
                        if can_auto_derive_for_monotype(trait_table, bound, s) {
                            continue; // 自动派生通过
                        }
                    }
                }
                return Err(ErrorCodeDefinition::trait_bound_not_satisfied(
                    &format!("{}", ty),
                    bound,
                )
                .build());
            }
        }
        Ok(())
    }

    /// 检查 Const 边界
    pub fn check_const_bounds(
        &self,
        _ty: &MonoType,
        _bounds: &[MonoType],
    ) -> Result<()> {
        // 简化的实现：const边界检查
        // TODO: 实现完整的const边界检查
        Ok(())
    }

    /// 检查生命周期边界
    pub fn check_lifetime_bounds(
        &self,
        _ty: &MonoType,
        _bounds: &[String],
    ) -> Result<()> {
        // 简化的实现：生命周期边界检查
        // TODO: 实现完整的生命周期边界检查
        Ok(())
    }

    /// 检查泛型参数边界
    pub fn check_generic_bounds(
        &mut self,
        ty: &MonoType,
        trait_bounds: &[String],
        const_bounds: &[MonoType],
    ) -> Result<()> {
        // 检查特质边界
        self.check_trait_bounds(ty, trait_bounds)?;

        // 检查const边界
        self.check_const_bounds(ty, const_bounds)?;

        Ok(())
    }

    /// 检查类型是否满足约束（结构化匹配 - 鸭子类型）
    ///
    /// 规则：类型的字段必须包含约束要求的所有方法（函数字段）
    /// 方法签名需要兼容（参数和返回值类型匹配）
    ///
    /// 支持两种方法来源：
    /// 1. 结构体字段中的函数字段
    /// 2. 通过 Type.method 绑定的方法（从 TypeEnvironment 查询）
    pub fn check_constraint(
        &mut self,
        ty: &MonoType,
        constraint: &MonoType,
        env: Option<&TypeEnvironment>,
    ) -> Result<(), ConstraintCheckError> {
        let constraint_fields = constraint.constraint_fields();

        // 如果约束没有字段，任何类型都满足（空约束）
        if constraint_fields.is_empty() {
            return Ok(());
        }

        // 获取待检查类型的函数字段和方法绑定
        let type_name = match ty {
            MonoType::Struct(s) => Some(s.name.clone()),
            MonoType::TypeRef(name) => Some(name.clone()),
            _ => None,
        };

        // 收集类型的函数字段
        let type_fn_fields: Vec<(String, &MonoType)> = match ty {
            MonoType::Struct(s) => s
                .fields
                .iter()
                .filter(|(_, ty)| matches!(ty, MonoType::Fn { .. }))
                .map(|(name, ty)| (name.clone(), ty))
                .collect(),
            _ => Vec::new(),
        };

        // 收集方法绑定（从 TypeEnvironment 查询）
        let method_bindings: Vec<(String, MonoType)> =
            if let (Some(env), Some(ref name)) = (env, &type_name) {
                env.method_bindings
                    .iter()
                    .filter(|(key, _)| key.starts_with(&format!("{}.", name)))
                    .map(|(key, fn_type)| {
                        // 提取方法名：从 "Type.method" 中提取 "method"
                        let method_name = key.split('.').next_back().unwrap_or(key).to_string();
                        (method_name, fn_type.clone())
                    })
                    .collect()
            } else {
                Vec::new()
            };

        // 检查每个约束字段是否存在且签名兼容
        let mut missing_fields = Vec::new();
        let mut mismatched_fields = Vec::new();

        for (field_name, constraint_fn) in constraint_fields {
            // 先在结构体字段中查找
            let type_fn = type_fn_fields.iter().find(|(name, _)| name == &field_name);

            // 如果字段中没有，再在方法绑定中查找
            let method_fn = if type_fn.is_none() {
                method_bindings.iter().find(|(name, _)| name == &field_name)
            } else {
                None
            };

            match (type_fn, method_fn) {
                (Some((_, found_fn)), _) => {
                    // 在结构体字段中找到，检查函数签名兼容性
                    if !Self::fn_signatures_compatible(found_fn, constraint_fn) {
                        mismatched_fields.push((
                            field_name,
                            constraint_fn.type_name(),
                            found_fn.type_name(),
                        ));
                    }
                }
                (_, Some((_, found_fn))) => {
                    // 在方法绑定中找到，检查函数签名兼容性
                    if !Self::fn_signatures_compatible(found_fn, constraint_fn) {
                        mismatched_fields.push((
                            field_name,
                            constraint_fn.type_name(),
                            found_fn.type_name(),
                        ));
                    }
                }
                (None, None) => {
                    // 两处都没有找到
                    missing_fields.push(field_name);
                }
            }
        }

        if !missing_fields.is_empty() || !mismatched_fields.is_empty() {
            let constraint_name = constraint.type_name();
            let type_name = ty.type_name();

            let reason = if !missing_fields.is_empty() && !mismatched_fields.is_empty() {
                format!(
                    "missing methods: {:?}, methods with incompatible signatures: {:?}",
                    missing_fields,
                    mismatched_fields
                        .iter()
                        .map(|(n, _, _)| n)
                        .collect::<Vec<_>>()
                )
            } else if !missing_fields.is_empty() {
                format!("missing methods: {:?}", missing_fields)
            } else {
                format!(
                    "methods with incompatible signatures: {:?}",
                    mismatched_fields
                        .iter()
                        .map(|(n, e, f)| format!("{} (expected {}, found {})", n, e, f))
                        .collect::<Vec<_>>()
                )
            };

            return Err(ConstraintCheckError {
                type_name,
                constraint_name,
                reason,
                span: Span::default(),
            });
        }

        Ok(())
    }

    /// 检查两个函数签名是否兼容
    ///
    /// 约束的签名和类型的签名兼容：
    /// - 参数数量相同（或类型的参数包含约束的第一个参数作为 self）
    /// - 返回类型相同
    fn fn_signatures_compatible(
        found_fn: &MonoType,
        constraint_fn: &MonoType,
    ) -> bool {
        match (found_fn, constraint_fn) {
            (
                MonoType::Fn {
                    params: found_params,
                    return_type: found_return,
                    ..
                },
                MonoType::Fn {
                    params: constraint_params,
                    return_type: constraint_return,
                    ..
                },
            ) => {
                // 检查返回类型
                if found_return != constraint_return {
                    return false;
                }

                // 检查参数数量
                // 约束签名通常不包含 self，类型签名可能包含 self 作为第一个参数
                if found_params.len() == constraint_params.len() {
                    // 参数数量相同，直接比较
                    found_params == constraint_params
                } else if found_params.len() == constraint_params.len() + 1 {
                    // 类型签名多一个参数（可能是 self），跳过第一个参数比较
                    &found_params[1..] == constraint_params
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
