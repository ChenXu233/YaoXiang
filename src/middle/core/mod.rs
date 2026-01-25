//! 核心中间表示
//!
//! 定义编译器中间表示的核心数据结构，包括指令、操作数、字节码等。
//! 这是整个middle层的基石，所有其他模块都依赖于此。

pub mod bytecode;
pub mod ir;
pub mod ir_gen;

pub use ir::*;
pub use bytecode::*;
pub use ir_gen::*;
