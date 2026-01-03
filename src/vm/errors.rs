//! VM errors

use thiserror::Error;

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
}



