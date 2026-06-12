//! 类型约束定义
//!
//! 实现类型系统中的约束：
//! - TypeConstraint: 类型约束

use crate::frontend::core::types::mono::MonoType;
use crate::util::span::Span;

/// 类型约束
///
/// 在类型推断过程中收集的约束条件
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    /// 约束的左边
    pub left: MonoType,
    /// 约束的右边
    pub right: MonoType,
    /// 约束的来源位置
    pub span: Span,
}

impl TypeConstraint {
    /// 创建新的类型约束
    pub fn new(
        left: MonoType,
        right: MonoType,
        span: Span,
    ) -> Self {
        TypeConstraint { left, right, span }
    }
}
