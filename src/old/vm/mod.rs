//! Virtual Machine and bytecode execution
//!
//! This module contains the bytecode interpreter.

pub use errors::{VMError, VMResult};
pub use executor::{RegisterFile, VMConfig, VMStatus, VM};
pub use opcode::TypedOpcode;

// Re-export RuntimeValue for VM use
pub use crate::runtime::value::RuntimeValue;

pub mod inline_cache;
pub mod opcode;

mod errors;
mod executor;
mod frames;

#[cfg(test)]
mod tests;
