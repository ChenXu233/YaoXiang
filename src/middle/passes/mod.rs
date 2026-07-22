//! 编译器各个阶段
//!
//! 包含中间层的各个编译阶段。

pub mod codegen;
pub mod mono;

// IR生成器实际在core模块中，直接re-export
pub use crate::middle::core::ir_gen::*;
