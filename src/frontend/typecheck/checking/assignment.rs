#![allow(clippy::result_large_err)]

//! 赋值检查
//!
//! 检查赋值语句的类型正确性

use crate::util::diagnostic::{ErrorCodeDefinition, I18nRegistry, Result};
use crate::frontend::core::type_system::MonoType;
use crate::util::span::Span;
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
    ///
    /// 规则：
    /// 1. 如果目标是约束类型，拒绝（约束只能在泛型上下文中使用）
    /// 2. 否则使用子类型检查
    pub fn check_assignment(
        &self,
        lhs: &MonoType,
        rhs: &MonoType,
        span: Span,
    ) -> Result<()> {
        // 如果目标是约束类型，拒绝
        if lhs.is_constraint() {
            return Err(ErrorCodeDefinition::constraint_not_in_generic(
                &lhs.type_name(),
            ).at(span).build(I18nRegistry::en()));
        }

        // 使用子类型检查
        if !self.can_assign(lhs, rhs) {
            return Err(ErrorCodeDefinition::type_mismatch(
                &lhs.type_name(),
                &rhs.type_name(),
            ).at(span).build(I18nRegistry::en()));
        }

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
        _span: Span,
    ) -> Result<()> {
        // 简化的实现：总是返回成功
        Ok(())
    }
}
