//! 错误转换
//!
//! 提供不同错误类型之间的转换

use crate::frontend::shared::error::Diagnostic;

/// 错误转换trait
pub trait ErrorConvert<T> {
    fn convert(self) -> Result<T, Diagnostic>;
}

impl<T> ErrorConvert<T> for Result<T, String> {
    fn convert(self) -> Result<T, Diagnostic> {
        self.map_err(Diagnostic::error)
    }
}

impl<T> ErrorConvert<T> for Result<T, &str> {
    fn convert(self) -> Result<T, Diagnostic> {
        self.map_err(|msg| Diagnostic::error(msg.to_string()))
    }
}
