//! Virtual Machine executor

use super::*;
use crate::middle::{ConstValue, ModuleIR};
use crate::runtime::gc::{GC, GCConfig};
use crate::runtime::memory::Heap;
use crate::runtime::scheduler::Scheduler;
use std::collections::HashMap;

/// VM configuration
#[derive(Debug, Clone)]
pub struct VMConfig {
    /// Initial stack size
    pub stack_size: usize,
    /// Enable JIT compilation
    pub enable_jit: bool,
    /// GC configuration
    pub gc_config: GCConfig,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            stack_size: 64 * 1024,
            enable_jit: false,
            gc_config: GCConfig::default(),
        }
    }
}

/// Virtual Machine
#[derive(Debug)]
pub struct VM {
    /// Configuration
    config: VMConfig,
    /// Stack
    stack: Vec<Value>,
    sp: usize,
    fp: usize,
    /// Constants
    constants: Vec<ConstValue>,
    /// Globals
    globals: HashMap<String, Value>,
    /// Bytecode
    code: Vec<Opcode>,
    ip: usize,
    /// Runtime
    heap: Heap,
    gc: GC,
    scheduler: Scheduler,
    /// State
    status: VMStatus,
    error: Option<VMError>,
}

impl Default for VM {
    fn default() -> Self {
        Self::new_with_config(VMConfig::default())
    }
}

impl VM {
    /// Create VM with config
    pub fn new_with_config(config: VMConfig) -> Self {
        let gc_config = config.gc_config.clone();
        let stack_size = config.stack_size;
        Self {
            config,
            stack: Vec::with_capacity(stack_size),
            sp: 0,
            fp: 0,
            constants: vec![],
            globals: HashMap::new(),
            code: vec![],
            ip: 0,
            heap: Heap::new(),
            gc: GC::new(gc_config),
            scheduler: Scheduler::new(),
            status: VMStatus::Ready,
            error: None,
        }
    }

    /// Create VM with default config
    pub fn new() -> Self {
        Self::default()
    }

    /// Execute a module
    ///
    /// # Arguments
    ///
    /// * `_module` - The module IR to execute
    pub fn execute_module(&mut self, _module: &ModuleIR) -> VMResult<()> {
        // TODO: Implement execution
        self.status = VMStatus::Running;
        self.status = VMStatus::Finished;
        Ok(())
    }
}

/// Runtime value
#[derive(Debug, Clone)]
pub enum Value {
    /// No value / unit type
    Void,
    /// Boolean value
    Bool(bool),
    /// Integer value (128-bit)
    Int(i128),
    /// Floating point value (64-bit)
    Float(f64),
    /// Character
    Char(char),
    /// String
    String(String),
    /// Byte array
    Bytes(Vec<u8>),
    /// List of values
    List(Vec<Value>),
    /// Dictionary mapping values to values
    Dict(HashMap<Value, Value>),
    // TODO: Add more types
}

/// VM opcode instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    /// No operation
    Nop = 0x00,
    /// Push value onto stack
    Push,
    /// Pop value from stack
    Pop,
    /// Duplicate top of stack
    Dup,
    /// Swap top two values
    Swap,
    /// Load from local variable
    Load,
    /// Store to local variable
    Store,
    /// Addition
    Add,
    /// Subtraction
    Sub,
    /// Multiplication
    Mul,
    /// Division
    Div,
    /// Modulo
    Mod,
    /// Negation
    Neg,
    /// Comparison
    Cmp,
    /// Unconditional jump
    Jmp,
    /// Jump if true
    JmpIf,
    /// Jump if false
    JmpIfNot,
    /// Function call
    Call,
    /// Async function call
    CallAsync,
    /// Return from function
    Ret,
    /// Allocate memory
    Alloc,
    /// Free memory
    Free,
    /// Load field from object
    LoadField,
    /// Store field to object
    StoreField,
    /// Type cast
    Cast,
    /// Spawn async task
    Spawn,
    /// Await async task
    Await,
    /// Yield execution
    Yield,
    // ... more opcodes
}

/// VM execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMStatus {
    /// Ready to execute
    Ready,
    /// Currently executing
    Running,
    /// Execution finished
    Finished,
    /// Error occurred
    Error,
}

impl TryFrom<u8> for Opcode {
    type Error = VMError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Opcode::Nop),
            _ => Err(VMError::InvalidOpcode(value)),
        }
    }
}

