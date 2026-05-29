//! Interpreter backend for YaoXiang bytecode execution
//!
//! This module implements the interpreter-based execution backend.
//! It reads bytecode instructions and executes them directly.

pub mod executor;
pub mod ffi;
pub mod frames;
pub mod registers;
pub mod runtime;

#[cfg(test)]
mod tests;

pub use executor::Interpreter;
pub use registers::RegisterFile;
pub use frames::Frame;
pub use runtime::{EvalStrategy, InterpreterRuntimeConfig};
