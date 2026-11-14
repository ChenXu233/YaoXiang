//! 代码生成器测试模块
//!
//! 包含 buffer、emitter、flow、mod、operand 等模块的单元测试。
//! （字节码格式定义 + 序列化的单测自 D1 6.7.1b' 起随实现住在 `middle/bytecode/tests/`。）

pub mod buffer;
pub mod emitter;
pub mod flow;
pub mod global_slot;
pub mod mod_;
pub mod operand;
pub mod witness_constructor;
