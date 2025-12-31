//! Virtual Machine and bytecode execution
//!
//! This module contains the bytecode interpreter.

pub use executor::{VM, VMConfig, Value, Opcode, VMStatus};
pub use errors::{VMError, VMResult};

mod executor;
mod instructions;
mod frames;
mod errors;
