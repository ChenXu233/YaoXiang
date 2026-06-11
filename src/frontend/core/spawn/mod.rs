//! spawn 块分析模块
//!
//! 对齐 RFC-024（spawn 块并发模型）和 RFC-018（LLVM AOT 编译器）。
//!
//! - `placement`：spawn 出现位置合法性检查
//! - `analysis`：任务识别、依赖分析、资源冲突检测、spawn for 展开

pub mod analysis;
pub mod placement;

#[cfg(test)]
mod tests;
