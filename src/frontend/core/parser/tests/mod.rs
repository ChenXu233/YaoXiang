//!
//! Organized test modules for better maintainability:
//! - basic: 基础语法解析测试
//! - state: ParserState 单元测试
//! - type_parser: 类型解析测试
//! - fn_def: 函数定义测试
//! - ref_test: ref 关键字测试（RFC-004）
//! - syntax_validation: 语法验证测试
//! - unified_syntax_debug: RFC 统一语法调试测试
//! - old_syntax_rejection: 旧语法兼容性检查
//! - boundary: 边界情况测试
//! - unsafe_ptr: unsafe 和裸指针测试（RFC-009 Phase 7）

mod basic;
mod boundary;
mod fn_def;
mod old_syntax_rejection;
mod ref_test;
mod state;
mod syntax_validation;
mod unsafe_ptr;
