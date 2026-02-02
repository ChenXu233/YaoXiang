//! 类型变量定义
//!
//! 实现 Hindley-Milner 类型系统中的变量：
//! - TypeVar: 类型变量（用于类型推断）
//! - ConstVar: Const泛型变量（用于Const泛型参数）

use std::fmt;

/// 类型变量（用于类型推断）
///
/// 每个类型变量有一个唯一的索引，用于在类型环境中追踪
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(usize);

impl TypeVar {
    /// 创建新类型变量
    pub fn new(index: usize) -> Self {
        TypeVar(index)
    }

    /// 获取变量的索引
    pub fn index(&self) -> usize {
        self.0
    }
}

impl fmt::Display for TypeVar {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

/// Const泛型变量（用于Const泛型参数）
///
/// 每个Const变量有一个唯一的索引和类型，用于在类型环境中追踪Const参数
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConstVar(usize);

impl ConstVar {
    /// 创建新的Const变量
    pub fn new(index: usize) -> Self {
        ConstVar(index)
    }

    /// 获取变量的索引
    pub fn index(&self) -> usize {
        self.0
    }
}

impl fmt::Display for ConstVar {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "c{}", self.0)
    }
}
