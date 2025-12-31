//! Virtual Machine and bytecode execution
//!
//! This module contains the bytecode interpreter.

pub use errors::{VMError, VMResult};
pub use executor::{Opcode, VMConfig, VMStatus, Value, VM};

mod errors;
mod executor;
mod frames;
mod instructions;
