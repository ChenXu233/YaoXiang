//! Span 处理
//!
//! 提供源码位置跟踪功能

use crate::util::span::Span;

/// 支持Span的错误 trait
///
/// 允许错误类型提供源代码位置信息
pub trait SpannedError {
    /// 获取错误对应的源代码位置
    fn span(&self) -> Span;
}
