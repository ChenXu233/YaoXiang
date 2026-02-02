//! RFC-011 Const泛型支持
//!
//! 提供Const泛型的编译期求值和尺寸计算能力。
//!
//! # 示例
//! ```yaoxiang
//! type Array[T, N: Int] = {
//!     data: T[N],
//!     length: N,
//! }
//!
//! const SIZE: Int = factorial(5)  # 120
//!
//! type IntArray[10] = Array[Int, 10]
//! ```

pub mod eval;
pub mod generic_size;

// 重新导出主要类型
pub use eval::{ConstGenericEval, ConstExpr, ConstBinOp};
pub use generic_size::GenericSize;
use crate::frontend::core::type_system::ConstValue;

/// Const泛型错误
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ConstGenericError {
    #[error("Const evaluation failed: {0}")]
    EvalFailed(String),

    #[error("Const not supported for type: {0}")]
    NotSupported(String),

    #[error("Const dimension mismatch: {0}")]
    DimensionMismatch(String),
}

/// Const泛型求值结果
#[derive(Debug, Clone, PartialEq)]
pub struct ConstGenericResult {
    /// 求值结果
    pub value: ConstValue,

    /// 是否是编译期常量
    pub is_const: bool,
}

impl ConstGenericResult {
    /// 创建新的结果
    pub fn new(
        value: ConstValue,
        is_const: bool,
    ) -> Self {
        Self { value, is_const }
    }

    /// 检查是否是常量
    pub fn is_const(&self) -> bool {
        self.is_const
    }

    /// 获取整数值
    pub fn as_int(&self) -> Option<i128> {
        match &self.value {
            ConstValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// 获取布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            ConstValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}
