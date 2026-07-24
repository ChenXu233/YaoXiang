//! Backend abstraction layer for YaoXiang execution
//!
//! This module provides a unified interface for different execution backends:
//! - Interpreter: Fast bytecode interpretation
//! - AOT: Ahead-of-time compilation (future)
//! - JIT: Just-in-time compilation (future)
//!
//! # Architecture
//!
//! ```text
//! BytecodeIR (from codegen)
//!         |
//!         v
//!    +----+----+
//!    |         |
//! Interpreter  AOT (future)
//!    |         |
//!    +----+----+
//!         |
//!         v
//!    RuntimeValue
//! ```

pub mod common;
pub mod interpreter;
pub mod runtime;

use crate::middle::bytecode::{BytecodeModule, BytecodeFunction};
use crate::backends::common::{RuntimeValue, Heap, Handle};

/// Stack frame information for error reporting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFrame {
    /// Function name
    pub function_name: String,
    /// Instruction pointer
    pub ip: usize,
}

impl std::fmt::Display for StackFrame {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "  at {} (ip: {})", self.function_name, self.ip)
    }
}

/// Result type for executor operations
pub type ExecutorResult<T> = Result<T, ExecutorError>;

/// Executor error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    /// Runtime error with message and optional stack trace
    Runtime(String, Option<Vec<StackFrame>>),
    /// Type error with optional stack trace
    Type(String, Option<Vec<StackFrame>>),
    /// Stack overflow
    StackOverflow(Option<Vec<StackFrame>>),
    /// Heap exhaustion
    HeapExhausted,
    /// Invalid opcode
    InvalidOpcode(u8),
    /// Invalid handle access
    InvalidHandle(Handle),
    /// Division by zero
    DivisionByZero(Option<Vec<StackFrame>>),
    /// Index out of bounds
    IndexOutOfBounds(Option<Vec<StackFrame>>),
    /// Field not found
    FieldNotFound(String, Option<Vec<StackFrame>>),
    /// Function not found
    FunctionNotFound(String, Option<Vec<StackFrame>>),
}

impl ExecutorError {
    /// Create a runtime error (backward compatible, no stack)
    pub fn runtime_only(msg: impl Into<String>) -> Self {
        ExecutorError::Runtime(msg.into(), None)
    }

    /// Create a type error (backward compatible, no stack)
    pub fn type_only(msg: impl Into<String>) -> Self {
        ExecutorError::Type(msg.into(), None)
    }

    /// Get the stack trace from this error
    pub fn stack_trace(&self) -> Option<&Vec<StackFrame>> {
        match self {
            ExecutorError::Runtime(_, stack) => stack.as_ref(),
            ExecutorError::Type(_, stack) => stack.as_ref(),
            ExecutorError::StackOverflow(stack) => stack.as_ref(),
            ExecutorError::DivisionByZero(stack) => stack.as_ref(),
            ExecutorError::IndexOutOfBounds(stack) => stack.as_ref(),
            ExecutorError::FieldNotFound(_, stack) => stack.as_ref(),
            ExecutorError::FunctionNotFound(_, stack) => stack.as_ref(),
            ExecutorError::HeapExhausted => None,
            ExecutorError::InvalidOpcode(_) => None,
            ExecutorError::InvalidHandle(_) => None,
        }
    }

    /// Create a new runtime error with stack trace
    pub fn runtime(
        msg: impl Into<String>,
        stack: Vec<StackFrame>,
    ) -> Self {
        ExecutorError::Runtime(msg.into(), Some(stack))
    }

    /// Create a new type error with stack trace
    pub fn type_error(
        msg: impl Into<String>,
        stack: Vec<StackFrame>,
    ) -> Self {
        ExecutorError::Type(msg.into(), Some(stack))
    }

    /// Create a new function not found error with stack trace
    pub fn function_not_found(
        name: impl Into<String>,
        stack: Vec<StackFrame>,
    ) -> Self {
        ExecutorError::FunctionNotFound(name.into(), Some(stack))
    }

    /// Create a new field not found error with stack trace
    pub fn field_not_found(
        name: impl Into<String>,
        stack: Vec<StackFrame>,
    ) -> Self {
        ExecutorError::FieldNotFound(name.into(), Some(stack))
    }

    /// Create a stack overflow error with stack trace
    pub fn stack_overflow(stack: Vec<StackFrame>) -> Self {
        ExecutorError::StackOverflow(Some(stack))
    }

    /// Create a division by zero error with stack trace
    pub fn division_by_zero(stack: Vec<StackFrame>) -> Self {
        ExecutorError::DivisionByZero(Some(stack))
    }

    /// Create an index out of bounds error with stack trace
    pub fn index_out_of_bounds(stack: Vec<StackFrame>) -> Self {
        ExecutorError::IndexOutOfBounds(Some(stack))
    }

    /// Add stack trace to an error if it doesn't have one
    pub fn with_stack(
        self,
        stack: Vec<StackFrame>,
    ) -> Self {
        match self {
            // Already has stack trace
            ExecutorError::Runtime(_, Some(_)) => self,
            ExecutorError::Type(_, Some(_)) => self,
            ExecutorError::StackOverflow(Some(_)) => self,
            ExecutorError::DivisionByZero(Some(_)) => self,
            ExecutorError::IndexOutOfBounds(Some(_)) => self,
            ExecutorError::FieldNotFound(_, Some(_)) => self,
            ExecutorError::FunctionNotFound(_, Some(_)) => self,
            // Add stack trace
            ExecutorError::Runtime(msg, None) => ExecutorError::Runtime(msg, Some(stack)),
            ExecutorError::Type(msg, None) => ExecutorError::Type(msg, Some(stack)),
            ExecutorError::StackOverflow(None) => ExecutorError::StackOverflow(Some(stack)),
            ExecutorError::DivisionByZero(None) => ExecutorError::DivisionByZero(Some(stack)),
            ExecutorError::IndexOutOfBounds(None) => ExecutorError::IndexOutOfBounds(Some(stack)),
            ExecutorError::FieldNotFound(name, None) => {
                ExecutorError::FieldNotFound(name, Some(stack))
            }
            ExecutorError::FunctionNotFound(name, None) => {
                ExecutorError::FunctionNotFound(name, Some(stack))
            }
            // These don't support stack trace
            ExecutorError::HeapExhausted => self,
            ExecutorError::InvalidOpcode(op) => ExecutorError::InvalidOpcode(op),
            ExecutorError::InvalidHandle(h) => ExecutorError::InvalidHandle(h),
        }
    }
}

impl std::fmt::Display for ExecutorError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ExecutorError::Runtime(msg, stack) => {
                write!(f, "Runtime error: {}", msg)?;
                if let Some(frames) = stack {
                    for frame in frames {
                        writeln!(f, "{}", frame)?;
                    }
                }
                Ok(())
            }
            ExecutorError::Type(msg, stack) => {
                write!(f, "Type error: {}", msg)?;
                if let Some(frames) = stack {
                    for frame in frames {
                        writeln!(f, "{}", frame)?;
                    }
                }
                Ok(())
            }
            ExecutorError::StackOverflow(stack) => {
                write!(f, "Stack overflow")?;
                if let Some(frames) = stack {
                    for frame in frames {
                        writeln!(f, "{}", frame)?;
                    }
                }
                Ok(())
            }
            ExecutorError::HeapExhausted => write!(f, "Heap exhausted"),
            ExecutorError::InvalidOpcode(op) => write!(f, "Invalid opcode: {:#x}", op),
            ExecutorError::InvalidHandle(h) => write!(f, "Invalid handle: {}", h),
            ExecutorError::DivisionByZero(stack) => {
                write!(f, "Division by zero")?;
                if let Some(frames) = stack {
                    for frame in frames {
                        writeln!(f, "{}", frame)?;
                    }
                }
                Ok(())
            }
            ExecutorError::IndexOutOfBounds(stack) => {
                write!(f, "Index out of bounds")?;
                if let Some(frames) = stack {
                    for frame in frames {
                        writeln!(f, "{}", frame)?;
                    }
                }
                Ok(())
            }
            ExecutorError::FieldNotFound(name, stack) => {
                write!(f, "Field not found: {}", name)?;
                if let Some(frames) = stack {
                    for frame in frames {
                        writeln!(f, "{}", frame)?;
                    }
                }
                Ok(())
            }
            ExecutorError::FunctionNotFound(name, stack) => {
                write!(f, "Function not found: {}", name)?;
                if let Some(frames) = stack {
                    for frame in frames {
                        writeln!(f, "{}", frame)?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ExecutorError {}

/// Execution state for a running program
#[derive(Debug, Clone, Default)]
pub struct ExecutionState {
    /// Current function name
    pub current_function: Option<String>,
    /// Instruction pointer
    pub ip: usize,
    /// Call stack depth
    pub call_depth: usize,
    /// Whether execution is complete
    pub is_complete: bool,
}

/// Executor trait - all backends must implement this
///
/// This trait defines the core execution interface. Implementations
/// can be interpreters, AOT compilers, or JIT compilers.
pub trait Executor {
    /// Execute a module
    fn execute_module(
        &mut self,
        module: &BytecodeModule,
    ) -> ExecutorResult<()>;

    /// Execute a single function with arguments
    fn execute_function(
        &mut self,
        func: &BytecodeFunction,
        args: &[RuntimeValue],
    ) -> ExecutorResult<RuntimeValue>;

    /// Reset the executor state
    fn reset(&mut self);

    /// Get current execution state
    fn state(&self) -> &ExecutionState;

    /// Get the heap for inspection
    fn heap(&self) -> &Heap;
}

/// Debuggable executor - adds debugging capabilities
pub trait DebuggableExecutor: Executor {
    /// Set a breakpoint at the given instruction offset
    fn set_breakpoint(
        &mut self,
        offset: usize,
    );

    /// Remove a breakpoint
    fn remove_breakpoint(
        &mut self,
        offset: usize,
    );

    /// Check if there's a breakpoint at the current position
    fn has_breakpoint(&self) -> bool;

    /// Step one instruction
    fn step(&mut self) -> ExecutorResult<()>;

    /// Step over the next instruction (don't follow calls)
    fn step_over(&mut self) -> ExecutorResult<()>;

    /// Step out of the current function
    fn step_out(&mut self) -> ExecutorResult<()>;

    /// Run until completion or breakpoint
    fn run(&mut self) -> ExecutorResult<()>;

    /// Get the current instruction index
    fn current_ip(&self) -> usize;

    /// Get the current function name
    fn current_function(&self) -> Option<&str>;

    /// Get all breakpoints
    fn breakpoints(&self) -> Vec<usize>;
}

/// Build mode for the backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildMode {
    /// Debug mode with assertions and debugging info
    #[default]
    Debug,
    /// Release mode with optimizations
    Release,
    /// Profile mode for performance analysis
    Profile,
}

/// Configuration for an executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum call stack depth
    pub max_stack_depth: usize,
    /// Initial heap capacity
    pub initial_heap_size: usize,
    /// Maximum heap size
    pub max_heap_size: usize,
    /// Build mode
    pub build_mode: BuildMode,
    /// Enable runtime checks (bounds, null, etc.)
    pub enable_checks: bool,
    /// Enable debugging features
    pub enable_debug: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_stack_depth: 1024,
            initial_heap_size: 64 * 1024,
            max_heap_size: 64 * 1024 * 1024,
            build_mode: BuildMode::Debug,
            enable_checks: true,
            enable_debug: true,
        }
    }
}
