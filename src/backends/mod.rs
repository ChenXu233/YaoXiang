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
use crate::backends::common::RuntimeValue;

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
    /// Division by zero（#282：携带触发表达式，渲染层直接使用）
    DivisionByZero(String, Option<Vec<StackFrame>>),
    /// Index out of bounds（#280：携带 max/index 以映射 E6003）
    IndexOutOfBounds {
        /// 容器长度
        max: usize,
        /// 越界索引
        index: usize,
        /// 调用栈
        stack: Option<Vec<StackFrame>>,
    },
    /// 断言失败（#280：映射 E6005）
    AssertionFailed(String, Option<Vec<StackFrame>>),
    /// Field not found
    FieldNotFound(String, Option<Vec<StackFrame>>),
    /// Dict 键缺失（#299 §4：与索引越界区分——语义不同的契约失败，映射 E6008）
    KeyNotFound {
        key: String,
        stack: Option<Vec<StackFrame>>,
    },
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
            ExecutorError::DivisionByZero(_, stack) => stack.as_ref(),
            ExecutorError::IndexOutOfBounds { stack, .. } => stack.as_ref(),
            ExecutorError::AssertionFailed(_, stack) => stack.as_ref(),
            ExecutorError::FieldNotFound(_, stack) => stack.as_ref(),
            ExecutorError::KeyNotFound { stack, .. } => stack.as_ref(),
            ExecutorError::FunctionNotFound(_, stack) => stack.as_ref(),
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
    /// #299 §4: Dict 键缺失专用构造器（E6008）
    pub fn key_not_found(
        key: impl Into<String>,
        stack: Vec<StackFrame>,
    ) -> Self {
        ExecutorError::KeyNotFound {
            key: key.into(),
            stack: Some(stack),
        }
    }
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
    pub fn division_by_zero(
        expr: impl Into<String>,
        stack: Vec<StackFrame>,
    ) -> Self {
        ExecutorError::DivisionByZero(expr.into(), Some(stack))
    }

    /// Create an index out of bounds error
    pub fn index_out_of_bounds(
        max: usize,
        index: usize,
        stack: Option<Vec<StackFrame>>,
    ) -> Self {
        ExecutorError::IndexOutOfBounds { max, index, stack }
    }

    /// Create an assertion failed error
    pub fn assertion_failed(
        msg: impl Into<String>,
        stack: Option<Vec<StackFrame>>,
    ) -> Self {
        ExecutorError::AssertionFailed(msg.into(), stack)
    }

    /// Add stack trace to an error if it doesn't have one
    pub fn with_stack(
        self,
        stack: Vec<StackFrame>,
    ) -> Self {
        match self {
            // Already has stack trace
            ExecutorError::KeyNotFound { stack: Some(_), .. } => self,
            ExecutorError::Runtime(_, Some(_)) => self,
            ExecutorError::Type(_, Some(_)) => self,
            ExecutorError::StackOverflow(Some(_)) => self,
            ExecutorError::DivisionByZero(_, Some(_)) => self,
            ExecutorError::IndexOutOfBounds { stack: Some(_), .. } => self,
            ExecutorError::AssertionFailed(_, Some(_)) => self,
            ExecutorError::FieldNotFound(_, Some(_)) => self,
            ExecutorError::FunctionNotFound(_, Some(_)) => self,
            // Add stack trace
            ExecutorError::Runtime(msg, None) => ExecutorError::Runtime(msg, Some(stack)),
            ExecutorError::Type(msg, None) => ExecutorError::Type(msg, Some(stack)),
            ExecutorError::StackOverflow(None) => ExecutorError::StackOverflow(Some(stack)),
            ExecutorError::DivisionByZero(expr, None) => {
                ExecutorError::DivisionByZero(expr, Some(stack))
            }
            ExecutorError::IndexOutOfBounds {
                max,
                index,
                stack: None,
            } => ExecutorError::IndexOutOfBounds {
                max,
                index,
                stack: Some(stack),
            },
            ExecutorError::AssertionFailed(msg, None) => {
                ExecutorError::AssertionFailed(msg, Some(stack))
            }
            ExecutorError::FieldNotFound(name, None) => {
                ExecutorError::FieldNotFound(name, Some(stack))
            }
            ExecutorError::KeyNotFound { key, stack: None } => ExecutorError::KeyNotFound {
                key,
                stack: Some(stack),
            },
            ExecutorError::FunctionNotFound(name, None) => {
                ExecutorError::FunctionNotFound(name, Some(stack))
            }
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
            ExecutorError::DivisionByZero(expr, stack) => {
                // #282：直接用变体携带的表达式（原 fallback `<unknown>`）
                write!(f, "Division by zero: {}", expr)?;
                if let Some(frames) = stack {
                    for frame in frames {
                        writeln!(f, "{}", frame)?;
                    }
                }
                Ok(())
            }
            ExecutorError::IndexOutOfBounds { max, index, stack } => {
                write!(f, "Index out of bounds: {index} (length {max})")?;
                if let Some(frames) = stack {
                    for frame in frames {
                        writeln!(f, "{}", frame)?;
                    }
                }
                Ok(())
            }
            ExecutorError::AssertionFailed(msg, stack) => {
                write!(f, "Assertion failed: {}", msg)?;
                if let Some(frames) = stack {
                    for frame in frames {
                        writeln!(f, "{}", frame)?;
                    }
                }
                Ok(())
            }
            ExecutorError::KeyNotFound { key, stack } => {
                write!(f, "Key not found: {}", key)?;
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

/// Configuration for an executor
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum call stack depth
    pub max_stack_depth: usize,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_stack_depth: 1024,
        }
    }
}
