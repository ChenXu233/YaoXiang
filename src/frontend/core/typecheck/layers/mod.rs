//! 有序证明层
//!
//! 按层序执行：equivalence → ownership → termination → predicate。
//! 下层失败上层不跑。每层返回 ProofResult。

pub mod dispatch;
pub mod equivalence;
pub mod ownership;
pub mod predicate;
pub mod termination;

#[cfg(test)]
mod tests;
