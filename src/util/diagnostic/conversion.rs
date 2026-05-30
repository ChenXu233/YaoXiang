//! 错误转换
//!
//! 提供不同错误类型之间的转换
//! 所有转换都通过注册表中的 E8001（内部编译器错误）路径

use super::error::Diagnostic;
use super::codes::ErrorCodeDefinition;

/// 错误转换trait
pub trait ErrorConvert<T> {
    fn convert(self) -> Result<T, Diagnostic>;
}

impl<T> ErrorConvert<T> for Result<T, String> {
    fn convert(self) -> Result<T, Diagnostic> {
        self.map_err(|msg| ErrorCodeDefinition::internal_error(&msg).build())
    }
}

impl<T> ErrorConvert<T> for Result<T, &str> {
    fn convert(self) -> Result<T, Diagnostic> {
        self.map_err(|msg| ErrorCodeDefinition::internal_error(msg).build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_convert_string() {
        let result: Result<i32, String> = Err("test error".to_string());
        let converted: Result<i32, crate::util::diagnostic::Diagnostic> = result.convert();
        assert!(converted.is_err());
        let diag = converted.unwrap_err();
        assert_eq!(diag.code, "E8001");
        assert!(diag.message.contains("test error"));
    }

    #[test]
    fn test_error_convert_str() {
        let result: Result<i32, &str> = Err("test error");
        let converted: Result<i32, crate::util::diagnostic::Diagnostic> = result.convert();
        assert!(converted.is_err());
        let diag = converted.unwrap_err();
        assert_eq!(diag.code, "E8001");
    }

    #[test]
    fn test_error_convert_ok() {
        let result: Result<i32, String> = Ok(42);
        let converted: Result<i32, crate::util::diagnostic::Diagnostic> = result.convert();
        assert!(converted.is_ok());
        assert_eq!(converted.unwrap(), 42);
    }
}
