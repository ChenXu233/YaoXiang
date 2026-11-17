//! 编译器各个阶段
//!
//! 包含中间层的各个编译阶段。

pub mod codegen;
pub mod mono;

// IR 生成器在 middle/lower（WBS 7.0.1–7.0.3 迁入）
pub use crate::middle::lower::{generate_ir, generate_ir_with_context, AstToIrGenerator};
