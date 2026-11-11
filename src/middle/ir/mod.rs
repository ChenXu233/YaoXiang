//! IR（中间表示；D1 6.7.1c 前目录名为 `middle/core/`）
//!
//! 定义编译器中间表示的核心数据结构，包括指令、操作数与模块结构。
//! （字节码 IR / .42 容器 / opcode 词表自 D1 6.7.1b 起住在 `middle/bytecode/`。）
//! 这是整个middle层的基石，所有其他模块都依赖于此。

// ponytail: 目录改名后目录名与 IR 定义文件同名（ir/ir.rs）——module_inception 是过渡形态，
// 04 任务拆分 ir.rs（operand/instr/block/fn_）后消失，届时本豁免一并删除。
#[allow(clippy::module_inception)]
pub mod ir;
pub mod ir_gen;
pub mod normalize;
pub mod verify;

#[cfg(test)]
mod tests;

pub use ir::*;
pub use ir_gen::*;
