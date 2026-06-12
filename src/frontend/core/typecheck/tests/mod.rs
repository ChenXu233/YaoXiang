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
//!
//! 规范测试：
//! - rfc010: RFC-010 统一类型语法测试
//! - rfc011: RFC-011 泛型系统测试

mod checker;
mod environment;
mod rfc010;
mod rfc011;
mod rfc027_phase1_integration;
mod rfc027_phase2_smt;
mod signature;
mod types;
