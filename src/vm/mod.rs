//! Virtual Machine and bytecode execution
//!
//! This module contains the bytecode interpreter.

pub use errors::{VMError, VMResult};
pub use executor::{Opcode as ExecutorOpcode, VMConfig, VMStatus, Value, VM};

pub mod inline_cache;
pub mod opcode; // 添加 opcode 模块

mod errors;
mod executor;
mod frames;
mod instructions;

#[cfg(test)]
mod tests;
