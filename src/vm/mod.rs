//! Virtual Machine and bytecode execution
//!
//! This module contains the bytecode interpreter.

pub use errors::{VMError, VMResult};
pub use executor::{RegisterFile, VMConfig, VMStatus, Value, VM};
pub use opcode::TypedOpcode;

pub mod inline_cache;
pub mod opcode;

mod errors;
mod executor;
mod frames;
mod instructions;

#[cfg(test)]
mod tests;
