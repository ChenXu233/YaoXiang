//! 证明管道基础设施
//!
//! 定义所有 layers/ 和 checker.rs 共享的类型。
//! proof/ 只定义类型，不包含检查逻辑。

pub mod assumptions;
pub mod budget;
pub mod context;
pub mod dep_graph;
pub mod smt;
pub mod verdict;
