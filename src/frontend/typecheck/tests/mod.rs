//! 类型检查器测试模块
//!
//! 迁移自 old/typecheck/tests/

mod basic;
mod errors;
mod generics;
mod inference;
mod scope;

// 重新导出主要测试
pub use basic::*;
pub use inference::*;
pub use generics::*;
pub use errors::*;
pub use scope::*;
