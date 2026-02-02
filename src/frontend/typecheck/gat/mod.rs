//! GAT (Generic Associated Types) 支持模块
//!
//! 实现 RFC-011 GAT 支持

pub mod checker;
pub mod higher_rank;

// 重新导出
pub use checker::GATChecker;
pub use higher_rank::{HigherRankChecker, HigherRankError};

pub use crate::util::diagnostic::Result;
