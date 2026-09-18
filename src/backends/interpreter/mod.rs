//! Interpreter backend for YaoXiang bytecode execution
//!
//! This module implements the interpreter-based execution backend.
//! It reads bytecode instructions and executes them directly.

pub mod executor;
pub mod ffi;
pub mod frames;
pub mod image;

#[cfg(test)]
mod tests;

pub use executor::Interpreter;
pub use frames::Frame;
