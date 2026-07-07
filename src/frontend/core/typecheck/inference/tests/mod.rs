//! Inference 模块测试 — 基于语言规范 §3 & RFC-010/011
//!
//! 测试覆盖：
//! - §3.1-§3.17: 类型推断
//! - §6: 函数定义推断
//! - RFC-010: 统一类型语法
//!
//! 子模块测试：
//! - expressions: 表达式推断
//! - statements: 语句检查
//! - patterns: 模式匹配
//! - bounds: 类型边界
//! - subtyping: 子类型
//! - scope: 作用域
//! - assignment: 赋值检查

mod assignment;
mod bounds;
mod bounds_duck;
mod compatibility;
mod expressions;
mod patterns;
mod scope;
mod statements;
mod subtyping;
