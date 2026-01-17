//! Virtual Machine executor

use super::*;
use crate::middle::{ConstValue, ModuleIR};
use crate::runtime::memory::Heap;
use crate::runtime::scheduler::FlowScheduler;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc as StdArc;

/// VM configuration
#[derive(Debug, Clone)]
pub struct VMConfig {
    /// Initial stack size
    pub stack_size: usize,
    /// Enable JIT compilation
    pub enable_jit: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            stack_size: 64 * 1024,
            enable_jit: false,
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
    scheduler: FlowScheduler,
    /// Arc 追踪：用于运行时引用计数管理
    arc_refcounts: HashMap<usize, ArcRefCount>,
    /// State
    status: VMStatus,
    error: Option<VMError>,
}

impl Default for VM {
    fn default() -> Self {
        Self::new_with_config(VMConfig::default())
    }
}

#[allow(deprecated)]
impl VM {
    /// Create VM with config
    /// Create VM with config
    pub fn new_with_config(config: VMConfig) -> Self {
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
            scheduler: FlowScheduler::new(),
            arc_refcounts: HashMap::new(),
            status: VMStatus::Ready,
            error: None,
        }
    }

    /// Create VM with default config
    pub fn new() -> Self {
        Self::default()
    }

    /// Get VM status
    pub fn status(&self) -> VMStatus {
        self.status
    }

    /// Get VM error
    pub fn error(&self) -> Option<&VMError> {
        self.error.as_ref()
    }

    /// Execute a module
    ///
    /// # Arguments
    ///
    /// * `_module` - The module IR to execute
    pub fn execute_module(
        &mut self,
        _module: &ModuleIR,
    ) -> VMResult<()> {
        // TODO: Implement full execution engine
        self.status = VMStatus::Running;
        self.status = VMStatus::Finished;
        Ok(())
    }

    /// Execute bytecode instructions
    ///
    /// This is a basic execution engine for testing Drop instruction.
    pub fn execute_instructions(
        &mut self,
        instructions: &[Opcode],
    ) -> VMResult<()> {
        self.status = VMStatus::Running;
        self.code = instructions.to_vec();
        self.ip = 0;

        while self.ip < self.code.len() {
            let opcode = self.code[self.ip];
            self.ip += 1;

            match opcode {
                Opcode::Nop => {}
                Opcode::Push => {
                    // Push a placeholder value for testing
                    self.stack.push(Value::Void);
                }
                Opcode::Pop => {
                    if let Some(mut val) = self.stack.pop() {
                        val.drop();
                    }
                }
                Opcode::Drop => {
                    // Drop the top value without popping
                    if let Some(val) = self.stack.last_mut() {
                        val.drop();
                    }
                }
                Opcode::Alloc => {
                    // Allocate a heap object for testing
                    let obj = HeapObject {
                        type_id: 0,
                        data: vec![0; 16],
                    };
                    self.stack.push(Value::HeapObject(obj));
                }
                Opcode::Ret => {
                    // Return - pop and return value
                    self.status = VMStatus::Finished;
                    return Ok(());
                }
                Opcode::Call => {
                    // Function call placeholder
                    // In real implementation, this would push return address and jump
                }
                Opcode::ArcNew => {
                    // ArcNew: dst, src - 创建新的 Arc
                    // 从堆对象创建 Arc（简化实现）
                    if let Some(src) = self.stack.pop() {
                        // 获取堆对象的指针（这里简化处理）
                        let target = match &src {
                            Value::HeapObject(obj) => obj.data.as_ptr() as usize,
                            _ => 0,
                        };
                        // 创建 Arc 并压栈
                        let arc = ArcValue::new(target);
                        self.stack.push(Value::Arc(arc));
                    }
                }
                Opcode::ArcClone => {
                    // ArcClone: dst, src - 克隆 Arc（引用计数 +1）
                    if let Some(src) = self.stack.pop() {
                        // Arc 是 Clone 的，直接克隆
                        self.stack.push(src.clone());
                    }
                }
                Opcode::ArcDrop => {
                    // ArcDrop: src - 释放 Arc（引用计数 -1）
                    // Arc 的引用计数由 StdArc 自动管理
                    // 当 Arc 被 drop 时，StdArc 会自动减少计数
                    if let Some(mut val) = self.stack.pop() {
                        val.drop();
                    }
                }
                _ => {
                    // Other opcodes not yet implemented in basic engine
                }
            }
        }

        self.status = VMStatus::Finished;
        Ok(())
    }

    /// Push a value onto the stack
    pub fn push(
        &mut self,
        value: Value,
    ) {
        self.stack.push(value);
    }

    /// Pop a value from the stack
    pub fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    /// Get stack depth
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
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
    /// Heap-allocated object (with ownership tracking)
    HeapObject(HeapObject),
    /// Arc (原子引用计数)
    Arc(ArcValue),
    // TODO: Add more types
}

/// Heap-allocated object with type info
#[derive(Debug, Clone)]
pub struct HeapObject {
    /// Type identifier
    pub type_id: usize,
    /// Object data (byte representation for simplicity)
    pub data: Vec<u8>,
}

/// Arc 引用计数追踪
#[derive(Debug)]
struct ArcRefCount {
    /// 原子引用计数
    count: AtomicUsize,
    /// 指向堆对象的指针
    target: usize,
}

/// Arc 值（运行时引用计数指针）
#[derive(Debug, Clone)]
pub struct ArcValue {
    /// 内部引用标准库的 Arc（用于实际的引用计数管理）
    inner: StdArc<ArcInner>,
}

/// Arc 内部数据
#[derive(Debug)]
struct ArcInner {
    /// 指向堆对象的指针（用于内存释放）
    target: usize,
}

impl ArcValue {
    /// 创建新的 Arc，指向给定的堆对象
    pub fn new(target: usize) -> Self {
        Self {
            inner: StdArc::new(ArcInner { target }),
        }
    }

    /// 获取内部目标指针
    pub fn target(&self) -> usize {
        self.inner.target
    }
}

impl Value {
    /// Drop this value, releasing any owned resources
    #[allow(clippy::should_implement_trait)]
    pub fn drop(&mut self) {
        match self {
            Value::String(s) => {
                // Release string memory
                s.clear();
            }
            Value::Bytes(b) => {
                // Release byte array
                b.clear();
            }
            Value::List(list) => {
                // Recursively drop all elements
                for item in list.iter_mut() {
                    item.drop();
                }
                list.clear();
            }
            Value::Dict(map) => {
                // Recursively drop all values (keys don't need explicit cleanup for simple types)
                // Take ownership of values to drop them properly
                let values: Vec<_> = map.values().cloned().collect();
                for mut v in values {
                    v.drop();
                }
                // Clear the map (keys are dropped automatically when removed)
                map.clear();
            }
            Value::HeapObject(obj) => {
                // Release heap object
                obj.data.clear();
            }
            Value::Arc(_arc) => {
                // Arc 的引用计数由 StdArc 自动管理
                // 当所有 Arc 副本被 drop 时，StdArc 会自动释放内存
                // 这里不需要额外处理
            }
            // Primitive types don't need explicit cleanup
            Value::Void | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::Char(_) => {}
        }
    }

    /// Check if this value needs to be dropped (is owned)
    pub fn needs_drop(&self) -> bool {
        matches!(
            self,
            Value::String(_)
                | Value::Bytes(_)
                | Value::List(_)
                | Value::Dict(_)
                | Value::HeapObject(_)
                | Value::Arc(_)
        )
    }
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
    /// Drop value (ownership-based cleanup)
    Drop,
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
    /// Create Arc (原子引用计数)
    ArcNew,
    /// Clone Arc (引用计数 +1)
    ArcClone,
    /// Drop Arc (引用计数 -1，归零时释放)
    ArcDrop,
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
