//! 错误处理模块
//!
//! 提供统一的错误处理和诊断机制

pub mod conversion;
pub mod diagnostic;
pub mod result;
pub mod span;

// 重新导出
pub use diagnostic::{Diagnostic, DiagnosticBuilder};
pub use result::{Result, ResultExt};
pub use conversion::ErrorConvert;
