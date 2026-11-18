//! Common backend components for YaoXiang execution
//!
//! This module provides shared components used across all backends:
//! - Runtime value types
//! - Heap storage
//! - Memory allocators

pub mod heap;
pub mod value;

// Re-exports for convenience
pub use heap::{Handle, Heap, HeapValue};
pub use value::RuntimeValue;

#[cfg(test)]
mod tests;
