#![allow(clippy::result_large_err)]

//! 错误转换
//!
//! 提供不同错误类型之间的转换
//! 所有转换都通过注册表中的 E8001（内部编译器错误）路径

use super::error::Diagnostic;
use super::codes::{ErrorCodeDefinition, I18nRegistry};
use crate::util::i18n::current_lang;

/// 错误转换trait
pub trait ErrorConvert<T> {
    fn convert(self) -> Result<T, Diagnostic>;
}

impl<T> ErrorConvert<T> for Result<T, String> {
    fn convert(self) -> Result<T, Diagnostic> {
        self.map_err(|msg| {
            let i18n = I18nRegistry::new(current_lang());
            ErrorCodeDefinition::internal_error(&msg).build(i18n)
        })
    }
}

impl<T> ErrorConvert<T> for Result<T, &str> {
    fn convert(self) -> Result<T, Diagnostic> {
        self.map_err(|msg| {
            let i18n = I18nRegistry::new(current_lang());
            ErrorCodeDefinition::internal_error(msg).build(i18n)
        })
    }
}
