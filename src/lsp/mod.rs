//! YaoXiang 语言服务器（LSP）
//!
//! 通过 stdin/stdout JSON-RPC 通信，为编辑器提供：
//! - 语法/类型诊断
//! - 代码补全
//! - 跳转定义
//! - 查找引用
//! - 悬停提示
//!
//! # 使用方式
//!
//! ```bash
//! yaoxiang lsp
//! ```

pub mod capabilities;
pub mod handlers;
pub mod locate;
pub mod protocol;
pub mod server;
pub mod session;
pub mod world;

pub use server::run_lsp_server;
