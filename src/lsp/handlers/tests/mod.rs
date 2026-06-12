//! LSP 处理器集成测试
//!
//! 测试覆盖：
//! - 代码操作处理器
//! - 代码补全处理器
//! - 跳转定义处理器
//! - 诊断处理器
//! - 格式化处理器
//! - 悬停提示处理器
//! - 初始化处理器
//! - 幽灵提示处理器
//! - 查找引用处理器
//! - 重命名处理器
//! - 语义 tokens 处理器
//! - 文档同步处理器
//! - 工作区符号搜索处理器

mod code_action;
mod completion;
mod definition;
mod diagnostics;
mod formatting;
mod hover;
mod initialize;
mod inlay_hint;
mod references;
mod rename;
mod semantic_tokens;
mod text_document;
mod workspace_symbol;
