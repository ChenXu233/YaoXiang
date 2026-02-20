#![allow(clippy::result_large_err)]

//! 赋值检查
//!
//! 检查赋值语句的类型正确性
//!
//! 支持接口（约束）类型的直接赋值：
//! - `d: Drawable = Circle(1)` — 编译期可确定具体类型 → 零开销调用
//! - `d: Drawable = get_shape()` — 编译期无法确定 → vtable 调用

use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use crate::frontend::core::type_system::MonoType;
use crate::util::span::Span;
use super::subtyping::SubtypeChecker;
use super::bounds::BoundsChecker;

/// 接口赋值的类型推断信息
///
/// 区分编译期可确定的具体类型和需要 vtable 的动态类型
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintAssignmentInfo {
    /// 编译期可确定具体类型 → 零开销直接调用
    Concrete {
        /// 具体类型
        concrete_type: MonoType,
        /// 约束类型
        constraint_type: MonoType,
    },
    /// 编译期无法确定具体类型 → vtable 调用
    Dynamic {
        /// 约束类型
        constraint_type: MonoType,
    },
}

/// 赋值检查器
pub struct AssignmentChecker {
    subtype_checker: SubtypeChecker,
    /// 最近一次接口赋值的推断信息
    last_constraint_info: Option<ConstraintAssignmentInfo>,
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
            last_constraint_info: None,
        }
    }

    /// 获取最近一次接口赋值的推断信息
    pub fn last_constraint_info(&self) -> Option<&ConstraintAssignmentInfo> {
        self.last_constraint_info.as_ref()
    }

    /// 检查赋值类型兼容性
    ///
    /// 规则：
    /// 1. 如果目标是约束类型，检查右值是否满足约束（结构化子类型）
    /// 2. 否则使用子类型检查
    pub fn check_assignment(
        &mut self,
        lhs: &MonoType,
        rhs: &MonoType,
        span: Span,
    ) -> Result<()> {
        // 清除上次的推断信息
        self.last_constraint_info = None;

        // 如果目标是约束类型，执行约束满足检查
        if lhs.is_constraint() {
            let mut bounds_checker = BoundsChecker::new();
            match bounds_checker.check_constraint(rhs, lhs) {
                Ok(()) => {
                    // 约束满足：判断具体类型是否可在编译期确定
                    // 如果右值是具体的 Struct 类型（非约束），编译期可确定
                    let is_concrete = matches!(rhs, MonoType::Struct(s) if !rhs.is_constraint() && !s.name.is_empty());

                    if is_concrete {
                        self.last_constraint_info = Some(ConstraintAssignmentInfo::Concrete {
                            concrete_type: rhs.clone(),
                            constraint_type: lhs.clone(),
                        });
                    } else {
                        self.last_constraint_info = Some(ConstraintAssignmentInfo::Dynamic {
                            constraint_type: lhs.clone(),
                        });
                    }
                    return Ok(());
                }
                Err(e) => {
                    return Err(ErrorCodeDefinition::type_mismatch(
                        &lhs.type_name(),
                        &format!("{} ({})", rhs.type_name(), e.reason),
                    )
                    .at(span)
                    .build());
                }
            }
        }

        // 使用子类型检查
        if !self.can_assign(lhs, rhs) {
            return Err(
                ErrorCodeDefinition::type_mismatch(&lhs.type_name(), &rhs.type_name())
                    .at(span)
                    .build(),
            );
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
