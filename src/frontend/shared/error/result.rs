//! 统一 Result 类型
//!
//! 为前端模块提供统一的错误处理

use crate::frontend::shared::error::Diagnostic;

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
        self.map_err(|err| Diagnostic::error(format!("{}: {}", f(), err)))
    }
}
