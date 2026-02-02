#![allow(clippy::result_large_err)]

//! 兼容性检查
//!
//! 检查类型之间的兼容性

use crate::util::diagnostic::Result;
use crate::frontend::core::type_system::MonoType;

/// 兼容性检查器
pub struct CompatibilityChecker;

impl Default for CompatibilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl CompatibilityChecker {
    /// 创建新的兼容性检查器
    pub fn new() -> Self {
        Self
    }

    /// 检查类型兼容性
    pub fn check_compatibility(
        &self,
        expected: &MonoType,
        actual: &MonoType,
    ) -> Result<bool> {
        // 简化的兼容性检查：检查类型是否相等或兼容
        Ok(self.is_compatible(expected, actual))
    }

    /// 检查两个类型是否兼容
    #[allow(clippy::only_used_in_recursion)]
    fn is_compatible(
        &self,
        expected: &MonoType,
        actual: &MonoType,
    ) -> bool {
        // 基本类型直接比较
        if expected == actual {
            return true;
        }

        // 检查类型变体兼容性
        match (expected, actual) {
            // 数字类型兼容性
            (MonoType::Int(_), MonoType::Int(_)) => true,
            (MonoType::Int(_), MonoType::Float(_)) => true,
            (MonoType::Float(_), MonoType::Int(_)) => true,
            (MonoType::Float(_), MonoType::Float(_)) => true,

            // 函数类型兼容性（参数和返回类型都要兼容）
            (
                MonoType::Fn {
                    params: exp_params,
                    return_type: exp_ret,
                    ..
                },
                MonoType::Fn {
                    params: act_params,
                    return_type: act_ret,
                    ..
                },
            ) => {
                exp_params.len() == act_params.len()
                    && exp_params
                        .iter()
                        .zip(act_params.iter())
                        .all(|(e, a)| self.is_compatible(e, a))
                    && self.is_compatible(exp_ret, act_ret)
            }

            // 列表兼容性
            (MonoType::List(exp_inner), MonoType::List(act_inner)) => {
                self.is_compatible(exp_inner, act_inner)
            }

            // 元组兼容性
            (MonoType::Tuple(exp_types), MonoType::Tuple(act_types)) => {
                exp_types.len() == act_types.len()
                    && exp_types
                        .iter()
                        .zip(act_types.iter())
                        .all(|(e, a)| self.is_compatible(e, a))
            }

            _ => false,
        }
    }

    /// 检查函数参数兼容性
    pub fn check_function_compatibility(
        &self,
        params: &[MonoType],
        args: &[MonoType],
    ) -> Result<bool> {
        // 检查参数数量是否匹配
        if params.len() != args.len() {
            return Ok(false);
        }

        // 检查每个参数是否兼容
        for (param, arg) in params.iter().zip(args.iter()) {
            if !self.check_compatibility(param, arg)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
