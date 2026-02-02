//! Lexer tests module
//!
//! Organized test modules for better maintainability:
//! - basic: 基础测试（标识符、空白符、换行等）
//! - literals: 字面量测试（整数、浮点、字符串、字符）
//! - operators: 运算符测试
//! - delimiters: 分隔符测试
//! - keywords: 关键字测试
//! - comments: 注释测试
//! - errors: 错误处理测试
//! - rfc004_lexer: RFC-004 绑定语法测试
//! - rfc010_lexer: RFC-010 泛型语法测试
//! - debug_lexer: 调试测试

mod basic;
mod literals;
mod operators;
mod delimiters;
mod keywords;
mod comments;
mod errors;
mod rfc004_lexer;
mod rfc010_lexer;
mod debug_lexer;

// Re-export all tests for backward compatibility
pub use basic::*;
pub use literals::*;
pub use operators::*;
pub use delimiters::*;
pub use keywords::*;
pub use comments::*;
pub use errors::*;
pub use rfc004_lexer::*;
pub use rfc010_lexer::*;
pub use debug_lexer::*;
