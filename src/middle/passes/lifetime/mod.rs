//! 所有权分析与生命周期管理
//!
//! 实现 Move 语义检查、Drop 语义检查和 Clone 语义检查，确保内存正确释放而无需 GC。
//! 设计原则：
//! 1. 每个值有一个所有者
//! 2. 当所有者离开作用域时，值被释放
//! 3. 所有权可以转移（Move），但不能复制（除非使用 clone()）
//!
//! # 模块结构
//!
//! - `error.rs`: 所有权错误类型定义
//! - `chain_calls.rs`: 链式调用分析
//! - `cycle_check.rs`: 跨 spawn 循环检测
//! - `intra_task_cycle.rs`: 任务内循环追踪
//! - `lifecycle.rs`: 生命周期管理
//! - `ownership_flow.rs`: 所有权流分析
//! - `unsafe_check.rs`: unsafe 块绕过检查

// 子模块
pub mod chain_calls;
pub mod cycle_check;
pub mod error;
pub mod intra_task_cycle;
pub mod lifecycle;
pub mod ownership_flow;
pub mod unsafe_check;

#[cfg(test)]
mod tests;

pub use chain_calls::*;
pub use cycle_check::*;
pub use error::*;
pub use intra_task_cycle::*;
pub use lifecycle::*;
pub use ownership_flow::*;
pub use unsafe_check::*;
