//! Common backend components for YaoXiang execution
//!
//! This module provides shared components used across all backends:
//! - Opcode definitions
//! - Runtime value types
//! - Heap storage
//! - Memory allocators

pub mod allocator;
pub mod heap;
pub mod opcode;
pub mod value;

// Re-exports for convenience
pub use opcode::Opcode;
pub use value::RuntimeValue;
pub use heap::{Handle, Heap, HeapValue};
pub use allocator::{Allocator, BumpAllocator, MemoryLayout, AllocError};
