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
pub mod dev;
pub mod interpreter;
pub mod runtime;

use crate::middle::bytecode::{BytecodeModule, BytecodeFunction};
use crate::backends::common::{RuntimeValue, Heap, Handle};

/// Result type for executor operations
pub type ExecutorResult<T> = Result<T, ExecutorError>;

/// Executor error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorError {
    /// Runtime error with message
    Runtime(String),
    /// Type error
    Type(String),
    /// Stack overflow
    StackOverflow,
    /// Heap exhaustion
    HeapExhausted,
    /// Invalid opcode
    InvalidOpcode(u8),
    /// Invalid handle access
    InvalidHandle(Handle),
    /// Division by zero
    DivisionByZero,
    /// Index out of bounds
    IndexOutOfBounds,
    /// Field not found
    FieldNotFound(String),
    /// Function not found
    FunctionNotFound(String),
}

impl std::fmt::Display for ExecutorError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ExecutorError::Runtime(msg) => write!(f, "Runtime error: {}", msg),
            ExecutorError::Type(msg) => write!(f, "Type error: {}", msg),
            ExecutorError::StackOverflow => write!(f, "Stack overflow"),
            ExecutorError::HeapExhausted => write!(f, "Heap exhausted"),
            ExecutorError::InvalidOpcode(op) => write!(f, "Invalid opcode: {:#x}", op),
            ExecutorError::InvalidHandle(h) => write!(f, "Invalid handle: {}", h),
            ExecutorError::DivisionByZero => write!(f, "Division by zero"),
            ExecutorError::IndexOutOfBounds => write!(f, "Index out of bounds"),
            ExecutorError::FieldNotFound(name) => write!(f, "Field not found: {}", name),
            ExecutorError::FunctionNotFound(name) => write!(f, "Function not found: {}", name),
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

/// Frame information for debugging
#[derive(Debug, Clone)]
pub struct FrameInfo {
    /// Function name
    pub function: String,
    /// Instruction pointer
    pub ip: usize,
    /// Local variables
    pub locals: Vec<(String, RuntimeValue)>,
}

/// Debug state for DebuggableExecutor
#[derive(Debug, Clone, Default)]
pub struct DebugState {
    /// Current execution state
    pub execution: ExecutionState,
    /// Breakpoint locations
    pub breakpoints: Vec<usize>,
    /// Current breakpoint hit (if any)
    pub breakpoint_hit: Option<usize>,
    /// Step mode (step, step-over, step-out)
    pub step_mode: StepMode,
    /// Step target for step-over
    pub step_target_ip: Option<usize>,
    /// Step target function for step-out
    pub step_target_depth: Option<usize>,
}

/// Step mode for debugging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    /// Continue execution normally
    Continue,
    /// Step one instruction
    Step,
    /// Step over calls
    StepOver,
    /// Step out of current function
    StepOut,
}

impl Default for StepMode {
    fn default() -> Self {
        Self::Continue
    }
}

/// Build mode for the backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    /// Debug mode with assertions and debugging info
    Debug,
    /// Release mode with optimizations
    Release,
    /// Profile mode for performance analysis
    Profile,
}

impl Default for BuildMode {
    fn default() -> Self {
        Self::Debug
    }
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
