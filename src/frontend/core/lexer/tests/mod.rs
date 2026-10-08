//! Lexer tests module
//!
//! Organized test modules for better maintainability:
//! - literals: 字面量测试（整数、浮点、字符串、字符）
//! - rfc004_lexer: RFC-004 绑定语法测试
//! - rfc010_lexer: RFC-010 泛型语法测试
//! - rfc029g_lexer: RFC-029g 删除 `pub` 关键字
//! - fstring: f-string 测试
//! - lexer_mod: 词法器整体行为测试
//! - symbols: 符号索引测试

mod fstring;
mod lexer_mod;
mod literals;
mod rfc004_lexer;
mod rfc010_lexer;
mod rfc029g_lexer;
mod symbols;
