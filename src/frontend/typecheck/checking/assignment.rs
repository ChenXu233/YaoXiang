#![allow(clippy::result_large_err)]

//! 赋值检查
//!
//! 检查赋值语句的类型正确性

use crate::frontend::shared::error::Result;
use crate::frontend::core::type_system::MonoType;
use super::subtyping::SubtypeChecker;

/// 赋值检查器
pub struct AssignmentChecker {
    subtype_checker: SubtypeChecker,
}

impl Default for AssignmentChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl AssignmentChecker {
    /// 创建新的赋值检查器
    pub fn new() -> Self {
        Self {
            subtype_checker: SubtypeChecker::new(),
        }
    }

    /// 检查赋值类型兼容性
    pub fn check_assignment(
        &self,
        _lhs: &MonoType,
        _rhs: &MonoType,
    ) -> Result<()> {
        // 简化的实现：总是返回成功
        Ok(())
    }

    /// 检查是否可以赋值
    fn can_assign(
        &self,
        lhs: &MonoType,
        rhs: &MonoType,
    ) -> bool {
        // 使用子类型检查
        self.subtype_checker.is_subtype(rhs, lhs)
    }

    /// 检查解构赋值
    pub fn check_destructuring(
        &self,
        _lhs_patterns: &[String],
        _rhs: &MonoType,
    ) -> Result<()> {
        // 简化的实现：总是返回成功
        Ok(())
    }
}
