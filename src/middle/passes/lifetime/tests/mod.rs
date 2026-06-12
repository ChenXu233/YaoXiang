//! 生命周期分析测试模块
//!
//! 包含 borrow_checker 等模块的单元测试。

pub mod borrow_checker;
pub mod chain_calls;
pub mod consume_analysis;
pub mod cycle_check;
pub mod intra_task_cycle;
pub mod lifecycle;
pub mod move_semantics;
pub mod ownership_flow;
