//! Traits 模块测试 — 基于语言规范 §3.5 & RFC-011 §2-3
//!
//! 测试覆盖：
//! - §3.5: 接口类型
//! - §3.9: 类型约束
//! - §3.10: 关联类型
//! - RFC-011 §2: 类型约束系统
//! - RFC-011 §3: 关联类型
//!
//! 子模块测试：
//! - solver: Trait 求解器
//! - impl_check: 实现检查
//! - inheritance: 继承
//! - coherence: 一致性
//! - auto_derive: 自动派生
//! - object_safety: 对象安全
//! - resolution: 解析
//! - std_traits: 标准库 trait
//! - gat: 泛型关联类型
//! - specialization: 特化

mod auto_derive;
mod borrow_token;
mod coherence;
mod gat;
mod impl_check;
mod inheritance;
mod object_safety;
mod resolution;
mod solver;
mod specialization;
mod std_traits;
