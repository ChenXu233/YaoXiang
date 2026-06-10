//! 终止检查模块
//!
//! 实现 RFC-027 Section 7：编译器全自动证明循环终止和递归函数终止。
//!
//! **当前实现（Phase 1）**：
//! - 策略 3：有界递增/递减模式 (`i += const` with `i < bound`)
//! - 递归参数递减检查 (`factorial(n-1)`)
//! - `for` 循环自动通过（范围迭代天然终止）
//!
//! **后续扩展**：
//! - 策略 1：线性秩函数自动合成
//! - 策略 2：谓词违反计数
//! - 策略 4：乘法缩放度量模板

pub mod checker;
pub mod measures;

#[cfg(test)]
mod tests;

pub use checker::TerminationChecker;
pub use measures::{Direction, LinearMeasure};
