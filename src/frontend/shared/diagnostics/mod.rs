//! 诊断模块
//!
//! 提供诊断信息的格式化和处理

pub mod code;
pub mod formatter;
pub mod severity;
pub mod traits;

// 重新导出
pub use formatter::DiagnosticFormatter;
pub use severity::Severity;
pub use code::ErrorCode;
pub use traits::DiagnosticTrait;
