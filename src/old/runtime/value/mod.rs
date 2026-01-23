//! Core runtime types for YaoXiang
//!
//! This module provides the fundamental value types used throughout
//! the YaoXiang runtime system.

pub mod heap;
pub mod runtime_value;

pub use runtime_value::*;
pub use heap::*;

#[cfg(test)]
mod tests;
