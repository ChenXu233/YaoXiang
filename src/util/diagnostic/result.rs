//! 统一 Result 类型
//!
//! 为编译器模块提供统一的错误处理

use super::error::Diagnostic;

/// 统一结果类型
pub type Result<T, E = Diagnostic> = std::result::Result<T, E>;
