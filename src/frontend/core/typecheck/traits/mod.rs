//! 特质系统模块
//!
//! 实现 RFC-011 特质系统（结构化类型模型）：
//! - solver: 特质求解和错误定义
//! - std_traits: 标准库特质定义 (数据定义在 core::types::base)
//! - auto_derive: 自动派生

pub mod auto_derive;
pub mod solver;
pub mod std_traits;

// 重新导出
pub use solver::{TraitSolver, TraitConstraint};

pub use crate::util::diagnostic::Result;

// ============ 测试模块 ============

#[cfg(test)]
mod tests;
