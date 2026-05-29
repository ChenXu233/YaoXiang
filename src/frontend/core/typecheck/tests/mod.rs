//! Typecheck 模块测试 — 基于语言规范 §3 & RFC-010/011
//!
//! 测试覆盖：
//! - §3.1-§3.17: 类型系统
//! - §6: 函数定义
//! - RFC-010: 统一类型语法
//! - RFC-011: 泛型系统设计
//!
//! 单文件模块测试：
//! - checker: TypeChecker 主检查器
//! - environment: TypeEnvironment 类型环境
//! - signature: 签名解析
//! - types: 类型定义
//! - overload: 重载解析
//! - type_eval: 类型求值
//! - dead_code: 死代码分析
//! - spawn_placement: spawn 放置检查
//!
//! 规范测试：
//! - rfc010: RFC-010 统一类型语法测试
//! - rfc011: RFC-011 泛型系统测试

mod checker;
mod dead_code;
mod environment;
mod overload;
mod rfc010;
mod rfc011;
mod signature;
mod spawn_placement;
mod type_eval;
mod types;
