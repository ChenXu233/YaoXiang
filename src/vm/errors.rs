//! VM errors

use thiserror::Error;
use std::time::Duration;
use crate::vm::opcode::TypedOpcode;
// Re-export types from interrupt module for convenience
pub use crate::runtime::interrupt::{AccessType, BreakpointId};

/// VM result
pub type VMResult<T> = Result<T, VMError>;

/// VM errors
#[derive(Debug, Error)]
pub enum VMError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(u8),

    #[error("Stack underflow")]
    StackUnderflow,

    #[error("Stack overflow")]
    StackOverflow,

    #[error("Invalid operand")]
    InvalidOperand,

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Index out of bounds")]
    IndexOutOfBounds,

    #[error("Key not found")]
    KeyNotFound,

    #[error("Uninitialized variable")]
    UninitializedVariable,

    #[error("Call stack overflow")]
    CallStackOverflow,

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Out of memory")]
    OutOfMemory,

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Unimplemented opcode: {0}")]
    UnimplementedOpcode(TypedOpcode),

    // === Interrupt-related errors ===
    // These are returned when an interrupt is caught by the scheduler
    #[error("Execution timeout after {0:?}")]
    Timeout(Duration),

    #[error("Breakpoint hit: {0}")]
    Breakpoint(BreakpointId),

    #[error("Memory access violation at address {addr:#018x} ({access})")]
    MemoryViolation {
        /// The memory address that was accessed
        addr: usize,
        /// The type of access that was attempted
        access: AccessType,
    },
}
