//! LSP 请求/通知处理器
//!
//! 每个子模块对应一类 LSP 方法。

pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod formatting;
pub mod hover;
pub mod initialize;
pub mod references;
pub mod semantic_tokens;
pub mod text_document;
