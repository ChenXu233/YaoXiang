#![allow(clippy::result_large_err)]

//! 统一 Result 类型
//!
//! 为编译器模块提供统一的错误处理
//! 所有错误转换都通过注册表中的错误码路径

use super::error::Diagnostic;
use super::codes::ErrorCodeDefinition;

/// 统一结果类型
pub type Result<T, E = Diagnostic> = std::result::Result<T, E>;

/// Result 扩展
pub trait ResultExt<T, E> {
    fn with_context<F>(
        self,
        f: F,
    ) -> Result<T, Diagnostic>
    where
        F: FnOnce() -> String,
        E: std::fmt::Display;
}

impl<T, E: std::fmt::Display> ResultExt<T, E> for Result<T, E> {
    fn with_context<F>(
        self,
        f: F,
    ) -> Result<T, Diagnostic>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|err| {
            ErrorCodeDefinition::internal_error(&format!("{}: {}", f(), err)).build()
        })
    }
}
