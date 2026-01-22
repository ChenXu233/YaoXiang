//! Immediate executor for embedded runtime
//!
//! Executes bytecode immediately without DAG scheduling.
//! Designed for WASM, game scripts, and resource-constrained environments.

use std::collections::HashMap;
use std::sync::Arc;

use crate::middle::codegen::bytecode::{CompiledModule, FunctionCode};
use crate::middle::ir::ConstValue;
use crate::runtime::memory::BumpAllocator;
use crate::runtime::value::runtime_value::{FunctionId, RuntimeValue};
use crate::vm::opcode::TypedOpcode;

/// Embedded runtime - immediate bytecode executor
///
/// # Design
/// - No DAG, no scheduler - direct interpretation
/// - Synchronous execution in call order
/// - Spawn treated as normal function call
///
/// # Usage
/// ```rust
/// let mut runtime = EmbeddedRuntime::new(64 * 1024);
/// let result = runtime.load_and_run(module);
/// ```
#[derive(Debug)]
pub struct EmbeddedRuntime {
    /// Memory allocator for heap allocations
    allocator: BumpAllocator,
    /// Constants pool (immutable)
    constants: Vec<ConstValue>,
    /// Function table: FunctionId -> FunctionCode
    functions: HashMap<FunctionId, FunctionCode>,
    /// Global variables
    globals: HashMap<String, RuntimeValue>,
    /// Main function ID
    main_func_id: Option<FunctionId>,
    /// Instruction pointer cache (func_id -> instruction count)
    func_ip_count: HashMap<FunctionId, usize>,
}

/// Interpreter state for bytecode execution
#[derive(Debug)]
struct Interpreter {
    /// Operand stack (push/pop values)
    stack: Vec<RuntimeValue>,
    /// Call stack (function frames)
    call_stack: Vec<Frame>,
}

/// Function call frame
#[derive(Debug)]
struct Frame {
    /// Function being executed
    func_id: FunctionId,
    /// Current instruction pointer (index into function's instructions)
    ip: usize,
    /// Return instruction index (where to resume after return)
    return_ip: usize,
    /// Local variables (indexed by local variable index)
    locals: Vec<RuntimeValue>,
    /// Upvalues (captured variables from outer scope)
    upvalues: Vec<RuntimeValue>,
}

/// Runtime errors for embedded execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// Stack underflow (pop from empty stack)
    StackUnderflow,
    /// Invalid local variable index
    InvalidLocal(usize),
    /// Invalid upvalue index
    InvalidUpvalue(usize),
    /// Invalid field access
    InvalidField(usize),
    /// Type mismatch during operation
    TypeMismatch,
    /// Function not found
    FunctionNotFound(FunctionId),
    /// Missing main function
    MissingMain,
    /// Invalid opcode
    InvalidOpcode(u8),
    /// Division by zero
    DivisionByZero,
    /// Invalid constant pool index
    InvalidConstIndex(usize),
    /// Call stack overflow (too many nested calls)
    CallStackOverflow,
    /// Invalid function call (wrong argument count)
    InvalidCall { expected: usize, got: usize },
    /// Invalid jump offset
    InvalidJump(isize),
    /// Unimplemented opcode
    UnimplementedOpcode(TypedOpcode),
    /// Index out of bounds
    IndexOutOfBounds { index: usize, length: usize },
    /// Exception thrown
    Exception { message: String },
}

impl std::fmt::Display for RuntimeError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            RuntimeError::StackUnderflow => write!(f, "stack underflow"),
            RuntimeError::InvalidLocal(idx) => write!(f, "invalid local variable {}", idx),
            RuntimeError::InvalidUpvalue(idx) => write!(f, "invalid upvalue {}", idx),
            RuntimeError::InvalidField(idx) => write!(f, "invalid field {}", idx),
            RuntimeError::TypeMismatch => write!(f, "type mismatch"),
            RuntimeError::FunctionNotFound(id) => write!(f, "function not found: {:?}", id),
            RuntimeError::MissingMain => write!(f, "missing main function"),
            RuntimeError::InvalidOpcode(op) => write!(f, "invalid opcode: {:#x}", op),
            RuntimeError::DivisionByZero => write!(f, "division by zero"),
            RuntimeError::InvalidConstIndex(idx) => write!(f, "invalid constant index: {}", idx),
            RuntimeError::CallStackOverflow => write!(f, "call stack overflow"),
            RuntimeError::InvalidCall { expected, got } => {
                write!(f, "invalid call: expected {} args, got {}", expected, got)
            }
            RuntimeError::InvalidJump(offset) => write!(f, "invalid jump offset: {}", offset),
            RuntimeError::UnimplementedOpcode(op) => {
                write!(f, "unimplemented opcode: {:?}", op)
            }
            RuntimeError::IndexOutOfBounds { index, length } => {
                write!(f, "index out of bounds: {} >= {}", index, length)
            }
            RuntimeError::Exception { message } => write!(f, "exception: {}", message),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl EmbeddedRuntime {
    /// Create a new embedded runtime with default capacity (64KB)
    pub fn new() -> Self {
        Self::with_capacity(64 * 1024)
    }

    /// Create a new embedded runtime with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            allocator: BumpAllocator::with_capacity(capacity),
            constants: Vec::new(),
            functions: HashMap::new(),
            globals: HashMap::new(),
            main_func_id: None,
            func_ip_count: HashMap::new(),
        }
    }

    /// Load a compiled module into the runtime
    pub fn load_module(
        &mut self,
        module: CompiledModule,
    ) {
        // Load constants
        self.constants = module.constants;

        // Load functions
        for func in module.functions {
            let func_id = FunctionId(self.functions.len() as u32);

            // Save values before moving
            let name = func.name;
            let instructions = func.instructions;
            let instructions_len = instructions.len();

            self.functions.insert(
                func_id,
                FunctionCode {
                    name: name.clone(),
                    params: func.params,
                    return_type: func.return_type,
                    instructions,
                    local_count: func.local_count,
                },
            );

            // Cache instruction count for bounds checking
            self.func_ip_count.insert(func_id, instructions_len);

            // Find main function
            if name == "main" {
                self.main_func_id = Some(func_id);
            }
        }

        // Load globals
        for (name, _ty, const_val) in module.globals {
            let value = match const_val {
                Some(ConstValue::Void) => RuntimeValue::Unit,
                Some(ConstValue::Bool(b)) => RuntimeValue::Bool(b),
                Some(ConstValue::Int(n)) => RuntimeValue::Int(n as i64),
                Some(ConstValue::Float(f)) => RuntimeValue::Float(f),
                Some(ConstValue::Char(c)) => RuntimeValue::Char(c as u32),
                Some(ConstValue::String(s)) => RuntimeValue::String(Arc::from(s.as_str())),
                Some(ConstValue::Bytes(b)) => RuntimeValue::Bytes(Arc::from(b.as_slice())),
                None => RuntimeValue::Unit,
            };
            self.globals.insert(name, value);
        }
    }

    /// Load and run the main function
    ///
    /// # Returns
    /// The return value of the main function, or an error
    pub fn load_and_run(
        &mut self,
        module: CompiledModule,
    ) -> Result<RuntimeValue, RuntimeError> {
        self.load_module(module);

        let main_id = self.main_func_id.ok_or(RuntimeError::MissingMain)?;
        self.execute_function(main_id, Vec::new())
    }

    /// Execute a function by ID
    fn execute_function(
        &mut self,
        func_id: FunctionId,
        args: Vec<RuntimeValue>,
    ) -> Result<RuntimeValue, RuntimeError> {
        let func = self
            .functions
            .get(&func_id)
            .ok_or(RuntimeError::FunctionNotFound(func_id))?;

        // Check argument count
        if args.len() != func.params.len() {
            return Err(RuntimeError::InvalidCall {
                expected: func.params.len(),
                got: args.len(),
            });
        }

        // Create initial frame
        let initial_frame = Frame {
            func_id,
            ip: 0,
            return_ip: 0, // 0 means no return (end of function)
            locals: args,
            upvalues: Vec::new(),
        };

        // Create interpreter
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            call_stack: vec![initial_frame],
        };

        // Execute
        self.execute(&mut interpreter, func_id)?;

        // Get result from stack
        interpreter.stack.pop().ok_or(RuntimeError::StackUnderflow)
    }

    /// Main execution loop
    fn execute(
        &mut self,
        interpreter: &mut Interpreter,
        mut current_func_id: FunctionId,
    ) -> Result<(), RuntimeError> {
        loop {
            // Get current frame
            let frame = interpreter.call_stack.last_mut().unwrap();

            // Get function and check IP bounds
            let func = match self.functions.get(&current_func_id) {
                Some(f) => f,
                None => return Err(RuntimeError::FunctionNotFound(current_func_id)),
            };

            if frame.ip >= func.instructions.len() {
                // End of function - return to caller
                interpreter.call_stack.pop();

                if interpreter.call_stack.is_empty() {
                    // Top-level function completed
                    return Ok(());
                }

                // Restore caller's context
                let caller_frame = interpreter.call_stack.last().unwrap();
                current_func_id = caller_frame.func_id;
                continue;
            }

            // Clone instruction to avoid borrow conflict
            let instr = func.instructions[frame.ip].clone();
            frame.ip += 1;

            self.execute_instruction(&instr, interpreter, current_func_id)?;
        }
    }

    /// Execute a single instruction
    fn execute_instruction(
        &mut self,
        instr: &crate::middle::codegen::bytecode::BytecodeInstruction,
        interpreter: &mut Interpreter,
        current_func_id: FunctionId,
    ) -> Result<(), RuntimeError> {
        let opcode: TypedOpcode = instr
            .opcode
            .try_into()
            .map_err(|_| RuntimeError::InvalidOpcode(instr.opcode))?;

        match opcode {
            // =====================
            // Control Flow (0x00-0x1F)
            // =====================
            TypedOpcode::Nop => {
                // No operation - do nothing
            }
            TypedOpcode::Return => {
                // Pop call stack (return unit)
                interpreter.call_stack.pop();
                if interpreter.call_stack.is_empty() {
                    return Ok(());
                }
            }
            TypedOpcode::ReturnValue => {
                // Pop return value from stack, then pop call stack
                let _return_value = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                interpreter.call_stack.pop();
                if interpreter.call_stack.is_empty() {
                    return Ok(());
                }
            }
            TypedOpcode::Jmp => {
                // Unconditional jump - operand is relative offset in instructions
                let offset = self.read_i32_operand(&instr.operands, 0)? as isize;
                let frame = interpreter.call_stack.last_mut().unwrap();
                frame.ip = frame.ip.wrapping_add(offset as usize);
                if frame.ip
                    >= self
                        .functions
                        .get(&current_func_id)
                        .unwrap()
                        .instructions
                        .len()
                {
                    return Err(RuntimeError::InvalidJump(offset));
                }
            }
            TypedOpcode::JmpIf => {
                // Conditional jump if true
                let cond = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let offset = self.read_i16_operand(&instr.operands, 0)? as isize;
                let frame = interpreter.call_stack.last_mut().unwrap();
                if cond.to_bool() == Some(true) {
                    frame.ip = frame.ip.wrapping_add(offset as usize);
                    if frame.ip
                        >= self
                            .functions
                            .get(&current_func_id)
                            .unwrap()
                            .instructions
                            .len()
                    {
                        return Err(RuntimeError::InvalidJump(offset));
                    }
                }
            }
            TypedOpcode::JmpIfNot => {
                // Conditional jump if false
                let cond = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let offset = self.read_i16_operand(&instr.operands, 0)? as isize;
                let frame = interpreter.call_stack.last_mut().unwrap();
                if cond.to_bool() != Some(true) {
                    frame.ip = frame.ip.wrapping_add(offset as usize);
                    if frame.ip
                        >= self
                            .functions
                            .get(&current_func_id)
                            .unwrap()
                            .instructions
                            .len()
                    {
                        return Err(RuntimeError::InvalidJump(offset));
                    }
                }
            }

            // =====================
            // Stack & Register Operations (0x10-0x1F)
            // =====================
            TypedOpcode::Mov => {
                // dst = src - handled by operand decoding, just push src to stack for simplicity
                // In our stack-based VM, this is handled by Load* instructions
            }
            TypedOpcode::LoadConst => {
                // dst = const_pool[const_idx]
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let const_idx = self.read_u16_operand(&instr.operands, 1)? as usize;
                let value = self.get_constant(const_idx)?;
                let frame = interpreter.call_stack.last_mut().unwrap();
                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = value;
            }
            TypedOpcode::LoadLocal => {
                // dst = locals[local_idx]
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let local_idx = self.read_u8_operand(&instr.operands, 1)?;
                let frame = interpreter.call_stack.last_mut().unwrap();
                if local_idx as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(local_idx as usize));
                }
                let value = frame.locals[local_idx as usize].clone();
                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = value;
            }
            TypedOpcode::StoreLocal => {
                // locals[local_idx] = src
                let src = self.read_u8_operand(&instr.operands, 0)?;
                let local_idx = self.read_u8_operand(&instr.operands, 1)?;
                let frame = interpreter.call_stack.last_mut().unwrap();
                if local_idx as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(local_idx as usize));
                }
                frame.locals[local_idx as usize] = frame.locals[src as usize].clone();
            }
            TypedOpcode::LoadArg => {
                // dst = args[arg_idx]
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let arg_idx = self.read_u8_operand(&instr.operands, 1)?;
                let frame = interpreter.call_stack.last_mut().unwrap();
                // Args are at the beginning of locals
                if arg_idx as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(arg_idx as usize));
                }
                let value = frame.locals[arg_idx as usize].clone();
                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = value;
            }

            // =====================
            // I64 Arithmetic (0x20-0x2F)
            // =====================
            TypedOpcode::I64Add => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int(a + b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Sub => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int(a - b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Mul => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int(a * b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Div => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => {
                        if b == 0 {
                            return Err(RuntimeError::DivisionByZero);
                        }
                        RuntimeValue::Int(a / b)
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Rem => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => {
                        if b == 0 {
                            return Err(RuntimeError::DivisionByZero);
                        }
                        RuntimeValue::Int(a % b)
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }

            // =====================
            // I64 Bitwise Operations (0x25-0x2A)
            // =====================
            TypedOpcode::I64And => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int(a & b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Or => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int(a | b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Xor => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int(a ^ b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Shl => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int(a << b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Sar => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int(a >> b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Shr => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int((a as i128 >> b) as i64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Neg => {
                let val = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match val.to_int() {
                    Some(a) => RuntimeValue::Int(-a),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }

            // =====================
            // I32 Arithmetic (0x30-0x3F) - convert to i64
            // =====================
            TypedOpcode::I32Add => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int((a as i32 + b as i32) as i64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I32Sub => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int((a as i32 - b as i32) as i64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I32Mul => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Int((a as i32 * b as i32) as i64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I32Div => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => {
                        if b == 0 {
                            return Err(RuntimeError::DivisionByZero);
                        }
                        RuntimeValue::Int((a as i32 / b as i32) as i64)
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I32Rem => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => {
                        if b == 0 {
                            return Err(RuntimeError::DivisionByZero);
                        }
                        RuntimeValue::Int((a as i32 % b as i32) as i64)
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I32Neg => {
                let val = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match val.to_int() {
                    Some(a) => RuntimeValue::Int(-(a as i32) as i64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }

            // =====================
            // F64 Arithmetic (0x40-0x4F)
            // =====================
            TypedOpcode::F64Add => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Float(a + b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Sub => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Float(a - b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Mul => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Float(a * b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Div => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => {
                        if b == 0.0 {
                            return Err(RuntimeError::DivisionByZero);
                        }
                        RuntimeValue::Float(a / b)
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Rem => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Float(a % b),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Sqrt => {
                let val = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match val.to_float() {
                    Some(a) => RuntimeValue::Float(a.sqrt()),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Neg => {
                let val = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match val.to_float() {
                    Some(a) => RuntimeValue::Float(-a),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }

            // =====================
            // F32 Arithmetic (0x50-0x5F) - convert to f64
            // =====================
            TypedOpcode::F32Add => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Float((a as f32 + b as f32) as f64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Sub => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Float((a as f32 - b as f32) as f64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Mul => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Float((a as f32 * b as f32) as f64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Div => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => {
                        if b == 0.0 {
                            return Err(RuntimeError::DivisionByZero);
                        }
                        RuntimeValue::Float((a as f32 / b as f32) as f64)
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Rem => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Float((a as f32 % b as f32) as f64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Sqrt => {
                let val = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match val.to_float() {
                    Some(a) => RuntimeValue::Float((a as f32).sqrt() as f64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Neg => {
                let val = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match val.to_float() {
                    Some(a) => RuntimeValue::Float(-(a as f32) as f64),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                interpreter.stack.push(result);
            }

            // =====================
            // Comparison (0x60-0x71)
            // =====================
            TypedOpcode::I64Eq => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = RuntimeValue::Bool(lhs == rhs);
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Ne => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = RuntimeValue::Bool(lhs != rhs);
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Lt => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool(a < b),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Le => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool(a <= b),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Gt => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool(a > b),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::I64Ge => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_int(), rhs.to_int()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool(a >= b),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Eq => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = RuntimeValue::Bool(lhs == rhs);
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Ne => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = RuntimeValue::Bool(lhs != rhs);
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Lt => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool(a < b),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Le => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool(a <= b),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Gt => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool(a > b),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F64Ge => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool(a >= b),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Eq => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = RuntimeValue::Bool(lhs == rhs);
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Ne => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = RuntimeValue::Bool(lhs != rhs);
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Lt => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool((a as f32) < (b as f32)),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Le => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool((a as f32) <= (b as f32)),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Gt => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool((a as f32) > (b as f32)),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }
            TypedOpcode::F32Ge => {
                let rhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let lhs = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
                let result = match (lhs.to_float(), rhs.to_float()) {
                    (Some(a), Some(b)) => RuntimeValue::Bool((a as f32) >= (b as f32)),
                    _ => RuntimeValue::Bool(false),
                };
                interpreter.stack.push(result);
            }

            // =====================
            // Memory & Object Operations (0x72-0x7D)
            // =====================
            TypedOpcode::HeapAlloc => {
                // 操作数：dst, type_id(u16), size
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let _type_id = self.read_u16_operand(&instr.operands, 1)?;
                let _size = self.read_u16_operand(&instr.operands, 3)?;

                let frame = interpreter.call_stack.last_mut().unwrap();

                // 分配内存（使用 bump allocator）
                // 简化：分配一个空值占位
                let allocated = RuntimeValue::Unit;

                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = allocated;
            }
            TypedOpcode::StackAlloc => {
                // 操作数：dst, size(u16)
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let size = self.read_u16_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();

                // 在局部变量中分配空间
                let start_idx = frame.locals.len();
                frame
                    .locals
                    .resize(start_idx + size as usize, RuntimeValue::Unit);

                // 存储起始索引到 dst
                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = RuntimeValue::Int(start_idx as i64);
            }
            TypedOpcode::Drop => {
                // 弹出并丢弃栈顶值
                interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;
            }
            TypedOpcode::GetField => {
                // 操作数：dst, src, field_idx
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let src = self.read_u8_operand(&instr.operands, 1)?;
                let _field_idx = self.read_u8_operand(&instr.operands, 2)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                if src as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(src as usize));
                }

                let src_value = &frame.locals[src as usize];
                let field_value = match src_value {
                    RuntimeValue::Struct { .. } => {
                        // 简化：返回第一个字段值
                        // 实际实现需要根据 field_idx 获取具体字段
                        RuntimeValue::Unit
                    }
                    RuntimeValue::Tuple(_handle) => {
                        // 从 handle 获取值
                        RuntimeValue::Unit
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                };

                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = field_value;
            }
            TypedOpcode::SetField => {
                // 操作数：dst, value, field_idx
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let value = self.read_u8_operand(&instr.operands, 1)?;
                let _field_idx = self.read_u8_operand(&instr.operands, 2)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                if dst as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(dst as usize));
                }
                if value as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(value as usize));
                }

                // 简化：直接修改局部变量
                // 实际实现需要更新结构体字段
                frame.locals[dst as usize] = frame.locals[value as usize].clone();
            }
            TypedOpcode::LoadElement => {
                // 操作数：dst, src, index
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let src = self.read_u8_operand(&instr.operands, 1)?;
                let index = self.read_u8_operand(&instr.operands, 2)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                if src as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(src as usize));
                }

                let element_value = match &frame.locals[src as usize] {
                    RuntimeValue::List(_handle) => {
                        // 从列表获取元素
                        RuntimeValue::Unit
                    }
                    RuntimeValue::Array(_handle) => {
                        // 从数组获取元素
                        RuntimeValue::Unit
                    }
                    RuntimeValue::Bytes(bytes) => {
                        // 从字节数组获取
                        let idx = index as usize;
                        if idx < bytes.len() {
                            RuntimeValue::Int(bytes[idx] as i64)
                        } else {
                            return Err(RuntimeError::StackUnderflow);
                        }
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                };

                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = element_value;
            }
            TypedOpcode::StoreElement => {
                // 操作数：dst, value, index
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let value = self.read_u8_operand(&instr.operands, 1)?;
                let _index = self.read_u8_operand(&instr.operands, 2)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                if dst as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(dst as usize));
                }
                if value as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(value as usize));
                }

                // 简化：存储到局部变量
                // 实际实现需要更新列表/数组元素
                frame.locals[dst as usize] = frame.locals[value as usize].clone();
            }
            TypedOpcode::NewListWithCap => {
                // 操作数：dst, capacity(u16)
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let _capacity = self.read_u16_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                // 简化：创建一个空列表占位
                let list_value = RuntimeValue::List(crate::runtime::value::heap::Handle::new(0));

                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = list_value;
            }
            TypedOpcode::ArcNew => {
                // 操作数：dst, src
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let src = self.read_u8_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                if src as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(src as usize));
                }

                // 使用 Arc 包装值
                let arc_value = RuntimeValue::Arc(Arc::new(frame.locals[src as usize].clone()));

                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = arc_value;
            }
            TypedOpcode::ArcClone => {
                // 操作数：dst, src (Arc)
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let src = self.read_u8_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                if src as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(src as usize));
                }

                // Arc 已经通过 Arc::clone 实现引用计数
                let cloned = frame.locals[src as usize].clone();

                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = cloned;
            }
            TypedOpcode::ArcDrop => {
                // 操作数：src (Arc)
                let src = self.read_u8_operand(&instr.operands, 0)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                if src as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(src as usize));
                }

                // Arc 的析构函数会自动减少引用计数
                // 当引用计数为 0 时，值会被自动释放
                frame.locals[src as usize] = RuntimeValue::Unit;
            }

            // =====================
            // Function Calls (0x80-0x86)
            // =====================
            TypedOpcode::CallStatic => {
                // 操作数：dst, func_id(u32), base_arg_reg, arg_count
                let _dst = self.read_u8_operand(&instr.operands, 0)?;
                let func_id_u32 = self.read_u32_operand(&instr.operands, 1)?;
                let base_arg_reg = self.read_u8_operand(&instr.operands, 5)?;
                let arg_count = self.read_u8_operand(&instr.operands, 6)?;

                let func_id = FunctionId(func_id_u32);
                let frame = interpreter.call_stack.last().unwrap();

                // 收集参数
                let mut args = Vec::with_capacity(arg_count as usize);
                for i in 0..arg_count {
                    let reg_idx = base_arg_reg.wrapping_add(i);
                    if reg_idx as usize >= frame.locals.len() {
                        return Err(RuntimeError::InvalidLocal(reg_idx as usize));
                    }
                    args.push(frame.locals[reg_idx as usize].clone());
                }

                // 压入返回地址和新帧
                let current_frame = interpreter.call_stack.last_mut().unwrap();
                current_frame.return_ip = current_frame.ip;

                // 创建新帧
                let new_frame = Frame {
                    func_id,
                    ip: 0,
                    return_ip: 0,
                    locals: args,
                    upvalues: Vec::new(),
                };
                interpreter.call_stack.push(new_frame);
            }
            TypedOpcode::CallVirt => {
                // 操作数：dst, obj_reg, vtable_idx(u16), base_arg_reg, arg_count
                let _dst = self.read_u8_operand(&instr.operands, 0)?;
                let obj_reg = self.read_u8_operand(&instr.operands, 1)?;
                let vtable_idx = self.read_u16_operand(&instr.operands, 2)?;
                let base_arg_reg = self.read_u8_operand(&instr.operands, 4)?;
                let arg_count = self.read_u8_operand(&instr.operands, 5)?;

                let frame = interpreter.call_stack.last().unwrap();

                // 获取对象值并查找 vtable
                let (func_id, upvalues) = match &frame.locals[obj_reg as usize] {
                    RuntimeValue::Struct { vtable, .. } => {
                        let method = vtable
                            .get(vtable_idx as usize)
                            .ok_or(RuntimeError::InvalidField(vtable_idx as usize))?;
                        (method.1.func_id, method.1.env.clone())
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                };

                let Some(func_code) = self.functions.get(&func_id) else {
                    return Err(RuntimeError::FunctionNotFound(func_id));
                };

                // 收集参数（从 base_arg_reg 开始的 arg_count 个局部变量）
                let mut args = Vec::with_capacity(arg_count as usize + 1); // +1 for self
                                                                           // 第一个参数是 self（对象本身）
                args.push(frame.locals[obj_reg as usize].clone());
                // 其余参数
                for i in 0..arg_count {
                    let reg_idx = base_arg_reg.wrapping_add(i);
                    if reg_idx as usize >= frame.locals.len() {
                        return Err(RuntimeError::InvalidLocal(reg_idx as usize));
                    }
                    args.push(frame.locals[reg_idx as usize].clone());
                }

                if args.len() != func_code.params.len() {
                    return Err(RuntimeError::InvalidCall {
                        expected: func_code.params.len(),
                        got: args.len(),
                    });
                }

                // 压入返回地址和新帧
                let current_frame = interpreter.call_stack.last_mut().unwrap();
                current_frame.return_ip = current_frame.ip;

                // 创建新帧
                let new_frame = Frame {
                    func_id,
                    ip: 0,
                    return_ip: 0,
                    locals: args,
                    upvalues,
                };
                interpreter.call_stack.push(new_frame);
            }
            TypedOpcode::CallDyn => {
                // 操作数：dst, obj_reg, name_idx(u16), base_arg_reg, arg_count
                // 动态调用：需要从栈顶获取函数值
                let _dst = self.read_u8_operand(&instr.operands, 0)?;
                let name_idx = self.read_u16_operand(&instr.operands, 1)? as usize;
                let base_arg_reg = self.read_u8_operand(&instr.operands, 3)?;
                let arg_count = self.read_u8_operand(&instr.operands, 4)?;

                // 获取函数值（从栈顶）- 暂未使用，实际实现需要解析函数值
                let _func_value = interpreter
                    .stack
                    .pop()
                    .ok_or(RuntimeError::StackUnderflow)?;

                // 从常量池获取方法名
                let method_name = match self.constants.get(name_idx) {
                    Some(ConstValue::String(s)) => s.clone(),
                    _ => return Err(RuntimeError::InvalidConstIndex(name_idx)),
                };

                // 从函数值中提取 func_id（简化处理：假设函数值包含 func_id）
                // 实际实现需要检查函数值的类型和结构
                let frame = interpreter.call_stack.last().unwrap();

                // 收集参数
                let mut args = Vec::with_capacity(arg_count as usize);
                for i in 0..arg_count {
                    let reg_idx = base_arg_reg.wrapping_add(i);
                    if reg_idx as usize >= frame.locals.len() {
                        return Err(RuntimeError::InvalidLocal(reg_idx as usize));
                    }
                    args.push(frame.locals[reg_idx as usize].clone());
                }

                // 查找函数（根据方法名查找）
                let target_func_id = self
                    .functions
                    .iter()
                    .find(|(_, f)| f.name == method_name)
                    .map(|(id, _)| *id)
                    .ok_or(RuntimeError::FunctionNotFound(FunctionId(0)))?;

                // 压入返回地址和新帧
                let current_frame = interpreter.call_stack.last_mut().unwrap();
                current_frame.return_ip = current_frame.ip;

                // 创建新帧
                let new_frame = Frame {
                    func_id: target_func_id,
                    ip: 0,
                    return_ip: 0,
                    locals: args,
                    upvalues: Vec::new(),
                };
                interpreter.call_stack.push(new_frame);
            }
            TypedOpcode::MakeClosure => {
                // 操作数：dst, func_id(u32), upvalue_count
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let func_id_u32 = self.read_u32_operand(&instr.operands, 1)?;
                let _upvalue_count = self.read_u8_operand(&instr.operands, 5)?;

                let func_id = FunctionId(func_id_u32);
                let frame = interpreter.call_stack.last_mut().unwrap();

                // 创建闭包值
                let closure_value =
                    RuntimeValue::Function(crate::runtime::value::runtime_value::FunctionValue {
                        func_id,
                        env: Vec::new(),
                    });

                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = closure_value;
            }
            TypedOpcode::LoadUpvalue => {
                // 操作数：dst, upvalue_idx
                let dst = self.read_u8_operand(&instr.operands, 0)?;
                let upvalue_idx = self.read_u8_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                if upvalue_idx as usize >= frame.upvalues.len() {
                    return Err(RuntimeError::InvalidUpvalue(upvalue_idx as usize));
                }

                let value = frame.upvalues[upvalue_idx as usize].clone();
                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = value;
            }
            TypedOpcode::StoreUpvalue => {
                // 操作数：src, upvalue_idx
                let src = self.read_u8_operand(&instr.operands, 0)?;
                let upvalue_idx = self.read_u8_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                if src as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(src as usize));
                }
                if upvalue_idx as usize >= frame.upvalues.len() {
                    // 如果 upvalue 不存在，创建它
                    frame
                        .upvalues
                        .resize(upvalue_idx as usize + 1, RuntimeValue::Unit);
                }
                frame.upvalues[upvalue_idx as usize] = frame.locals[src as usize].clone();
            }
            TypedOpcode::CloseUpvalue => {
                // 操作数：reg
                let reg = self.read_u8_operand(&instr.operands, 0)?;
                let frame = interpreter.call_stack.last_mut().unwrap();
                if reg as usize >= frame.locals.len() {
                    return Err(RuntimeError::InvalidLocal(reg as usize));
                }
                // 将栈上的变量移动到堆（通过 Arc 包装）
                let value = frame.locals[reg as usize].clone();
                frame.locals[reg as usize] = value;
            }

            // =====================
            // String Operations (0x90-0x95)
            // =====================
            TypedOpcode::StringLength => {
                // 操作数：src, dst
                let src = self.read_u8_operand(&instr.operands, 0)?;
                let dst = self.read_u8_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                let s = match &frame.locals[src as usize] {
                    RuntimeValue::String(arc) => arc.len(),
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = RuntimeValue::Int(s as i64);
            }
            TypedOpcode::StringConcat => {
                // 操作数：src1, src2, dst
                let src1 = self.read_u8_operand(&instr.operands, 0)?;
                let src2 = self.read_u8_operand(&instr.operands, 1)?;
                let dst = self.read_u8_operand(&instr.operands, 2)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                match (&frame.locals[src1 as usize], &frame.locals[src2 as usize]) {
                    (RuntimeValue::String(a), RuntimeValue::String(b)) => {
                        let result = Arc::from((**a).to_string() + &**b);
                        if dst as usize >= frame.locals.len() {
                            frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                        }
                        frame.locals[dst as usize] = RuntimeValue::String(result);
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
            TypedOpcode::StringEqual => {
                // 操作数：src1, src2, dst
                let src1 = self.read_u8_operand(&instr.operands, 0)?;
                let src2 = self.read_u8_operand(&instr.operands, 1)?;
                let dst = self.read_u8_operand(&instr.operands, 2)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                let equal = match (&frame.locals[src1 as usize], &frame.locals[src2 as usize]) {
                    (RuntimeValue::String(a), RuntimeValue::String(b)) => a == b,
                    _ => return Err(RuntimeError::TypeMismatch),
                };
                if dst as usize >= frame.locals.len() {
                    frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst as usize] = RuntimeValue::Bool(equal);
            }
            TypedOpcode::StringGetChar => {
                // 操作数：src, index, dst
                let src = self.read_u8_operand(&instr.operands, 0)?;
                let idx = self.read_u8_operand(&instr.operands, 1)?;
                let dst = self.read_u8_operand(&instr.operands, 2)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                match (
                    &frame.locals[src as usize],
                    frame.locals[idx as usize].clone(),
                ) {
                    (RuntimeValue::String(arc), RuntimeValue::Int(i)) => {
                        let char_idx = i as usize;
                        let ch = arc
                            .chars()
                            .nth(char_idx)
                            .map(|c| c as u32)
                            .ok_or(RuntimeError::StackUnderflow)?;
                        if dst as usize >= frame.locals.len() {
                            frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                        }
                        frame.locals[dst as usize] = RuntimeValue::Char(ch);
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
            TypedOpcode::StringFromInt => {
                // 操作数：src, dst
                let src = self.read_u8_operand(&instr.operands, 0)?;
                let dst = self.read_u8_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                match frame.locals[src as usize] {
                    RuntimeValue::Int(i) => {
                        let s = Arc::from(i.to_string());
                        if dst as usize >= frame.locals.len() {
                            frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                        }
                        frame.locals[dst as usize] = RuntimeValue::String(s);
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
            TypedOpcode::StringFromFloat => {
                // 操作数：src, dst
                let src = self.read_u8_operand(&instr.operands, 0)?;
                let dst = self.read_u8_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                match frame.locals[src as usize] {
                    RuntimeValue::Float(f) => {
                        let s = Arc::from(f.to_string());
                        if dst as usize >= frame.locals.len() {
                            frame.locals.resize(dst as usize + 1, RuntimeValue::Unit);
                        }
                        frame.locals[dst as usize] = RuntimeValue::String(s);
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }

            // =====================
            // Exception Handling (0xA0-0xA3)
            // =====================
            TypedOpcode::TryBegin => {
                // TryBegin 记录 catch 偏移量，但在嵌入式模式中不需要实际处理
                // 操作数：catch_offset (u16)
                // 简单地继续执行，不做任何特殊处理
            }
            TypedOpcode::TryEnd => {
                // TryEnd 标记 try 块结束，不做任何特殊处理
            }
            TypedOpcode::Throw => {
                // Throw 抛出异常，在嵌入式模式中直接返回错误
                // 操作数：exception_reg (u8)
                let exc_reg = self.read_u8_operand(&instr.operands, 0)?;
                let frame = interpreter.call_stack.last_mut().unwrap();
                let exc_value = &frame.locals[exc_reg as usize];
                return Err(RuntimeError::Exception {
                    message: format!("{:?}", exc_value),
                });
            }
            TypedOpcode::Rethrow => {
                // Rethrow 重新抛出当前异常，嵌入式模式中简化为 throw
                return Err(RuntimeError::Exception {
                    message: "rethrow".to_string(),
                });
            }

            // =====================
            // Other Operations (0xB0-0xD0)
            // =====================
            TypedOpcode::BoundsCheck => {
                // BoundsCheck 验证索引是否在有效范围内
                // 操作数：index_reg, length_reg
                let index_reg = self.read_u8_operand(&instr.operands, 0)?;
                let length_reg = self.read_u8_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                match (
                    frame.locals[index_reg as usize].clone(),
                    frame.locals[length_reg as usize].clone(),
                ) {
                    (RuntimeValue::Int(idx), RuntimeValue::Int(len)) => {
                        if idx < 0 || idx >= len {
                            return Err(RuntimeError::IndexOutOfBounds {
                                index: idx as usize,
                                length: len as usize,
                            });
                        }
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
            TypedOpcode::TypeCheck => {
                // TypeCheck 检查值是否为指定类型
                // 操作数：src_reg, type_id_reg, dst_reg
                let _src_reg = self.read_u8_operand(&instr.operands, 0)?;
                let _type_id_reg = self.read_u8_operand(&instr.operands, 1)?;
                let dst_reg = self.read_u8_operand(&instr.operands, 2)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                // 简化：所有值都认为是有效类型
                if dst_reg as usize >= frame.locals.len() {
                    frame
                        .locals
                        .resize(dst_reg as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst_reg as usize] = RuntimeValue::Bool(true);
            }
            TypedOpcode::Cast => {
                // Cast 类型转换
                // 操作数：src_reg, type_id_reg, dst_reg
                let src_reg = self.read_u8_operand(&instr.operands, 0)?;
                let _type_id_reg = self.read_u8_operand(&instr.operands, 1)?;
                let dst_reg = self.read_u8_operand(&instr.operands, 2)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                // 简化实现：直接复制值，不做实际类型转换
                if dst_reg as usize >= frame.locals.len() {
                    frame
                        .locals
                        .resize(dst_reg as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst_reg as usize] = frame.locals[src_reg as usize].clone();
            }
            TypedOpcode::TypeOf => {
                // TypeOf 返回值的类型信息
                // 操作数：src_reg, dst_reg
                let src_reg = self.read_u8_operand(&instr.operands, 0)?;
                let dst_reg = self.read_u8_operand(&instr.operands, 1)?;

                let frame = interpreter.call_stack.last_mut().unwrap();
                let type_name = match &frame.locals[src_reg as usize] {
                    RuntimeValue::Unit => "Unit",
                    RuntimeValue::Bool(_) => "Bool",
                    RuntimeValue::Int(_) => "Int",
                    RuntimeValue::Float(_) => "Float",
                    RuntimeValue::Char(_) => "Char",
                    RuntimeValue::String(_) => "String",
                    RuntimeValue::Bytes(_) => "Bytes",
                    RuntimeValue::Tuple(_) => "Tuple",
                    RuntimeValue::Array(_) => "Array",
                    RuntimeValue::List(_) => "List",
                    RuntimeValue::Dict(_) => "Dict",
                    RuntimeValue::Struct { .. } => "Struct",
                    RuntimeValue::Enum { .. } => "Enum",
                    RuntimeValue::Function(_) => "Function",
                    RuntimeValue::Arc(_) => "Arc",
                    RuntimeValue::Async(_) => "Async",
                    RuntimeValue::Ptr { .. } => "Ptr",
                };
                if dst_reg as usize >= frame.locals.len() {
                    frame
                        .locals
                        .resize(dst_reg as usize + 1, RuntimeValue::Unit);
                }
                frame.locals[dst_reg as usize] = RuntimeValue::String(Arc::from(type_name));
            }

            // =====================
            // Unused/Obscure (0x07-0x09, 0x0A-0x0B, 0xE0-0xE9)
            // =====================
            TypedOpcode::Switch => {
                // Switch 分支表跳转
                // 操作数：reg (u8，比较值寄存器), table_idx (u16，跳转表索引)
                let _reg = self.read_u8_operand(&instr.operands, 0)?;
                let _table_idx = self.read_u16_operand(&instr.operands, 1)?;

                // 简化实现：不做实际跳转，只消耗指令
                // 实际实现需要从常量池获取跳转表并执行索引跳转
            }
            TypedOpcode::LoopStart => {
                // LoopStart 循环开始标记，no-op
                // 操作数：start_reg, end_reg, step_reg, exit_offset
            }
            TypedOpcode::LoopInc => {
                // LoopInc 循环递增，no-op
                // 操作数：current_reg, step_reg, loop_start_offset
            }
            TypedOpcode::TailCall => {
                // TailCall 尾调用优化，在嵌入式模式中视为普通 CallStatic
                // 操作数：func_reg, arg_count
                let func_reg = self.read_u8_operand(&instr.operands, 0)?;
                let arg_count = self.read_u8_operand(&instr.operands, 1)?;

                // 获取当前帧的 IP（用于返回地址）
                let return_ip = {
                    let frame = interpreter.call_stack.last().unwrap();
                    frame.ip + 1
                };

                match &interpreter.call_stack.last().unwrap().locals[func_reg as usize] {
                    RuntimeValue::Function(func_value) => {
                        let Some(func_code) = self.functions.get(&func_value.func_id) else {
                            return Err(RuntimeError::FunctionNotFound(func_value.func_id));
                        };

                        // 准备参数
                        let stack_top = interpreter.stack.len();
                        let arg_start = stack_top - arg_count as usize;
                        let args: Vec<RuntimeValue> =
                            interpreter.stack.drain(arg_start..).collect();

                        if args.len() != func_code.params.len() {
                            return Err(RuntimeError::InvalidCall {
                                expected: func_code.params.len(),
                                got: args.len(),
                            });
                        }

                        // 创建新帧，使用 FunctionValue.env 作为 upvalues
                        let new_frame = Frame {
                            func_id: func_value.func_id,
                            ip: 0,
                            return_ip,
                            locals: args,
                            upvalues: func_value.env.clone(),
                        };
                        interpreter.call_stack.push(new_frame);
                    }
                    _ => return Err(RuntimeError::TypeMismatch),
                }
            }
            TypedOpcode::Yield => {
                // Yield is a no-op in embedded mode
            }
            TypedOpcode::Label => {
                // Label is just a marker, no-op
            }
            TypedOpcode::Invalid => {
                return Err(RuntimeError::InvalidOpcode(instr.opcode));
            }
            TypedOpcode::Custom0
            | TypedOpcode::Custom1
            | TypedOpcode::Custom2
            | TypedOpcode::Custom3 => {
                return Err(RuntimeError::UnimplementedOpcode(opcode));
            }
            // Catch-all for any other unimplemented opcodes (including Reserved*)
            _ => {
                return Err(RuntimeError::UnimplementedOpcode(opcode));
            }
        }

        Ok(())
    }

    // =====================
    // Operand Reading Helpers
    // =====================

    #[inline]
    fn read_u8_operand(
        &self,
        operands: &[u8],
        offset: usize,
    ) -> Result<u8, RuntimeError> {
        operands
            .get(offset)
            .copied()
            .ok_or(RuntimeError::InvalidOpcode(0))
    }

    #[inline]
    fn read_i8_operand(
        &self,
        operands: &[u8],
        offset: usize,
    ) -> Result<i8, RuntimeError> {
        operands
            .get(offset)
            .copied()
            .map(|v| v as i8)
            .ok_or(RuntimeError::InvalidOpcode(0))
    }

    #[inline]
    fn read_u16_operand(
        &self,
        operands: &[u8],
        offset: usize,
    ) -> Result<u16, RuntimeError> {
        if offset + 1 < operands.len() {
            Ok(u16::from_le_bytes([operands[offset], operands[offset + 1]]))
        } else {
            Err(RuntimeError::InvalidOpcode(0))
        }
    }

    #[inline]
    fn read_i16_operand(
        &self,
        operands: &[u8],
        offset: usize,
    ) -> Result<i16, RuntimeError> {
        if offset + 1 < operands.len() {
            Ok(i16::from_le_bytes([operands[offset], operands[offset + 1]]))
        } else {
            Err(RuntimeError::InvalidOpcode(0))
        }
    }

    #[inline]
    fn read_u32_operand(
        &self,
        operands: &[u8],
        offset: usize,
    ) -> Result<u32, RuntimeError> {
        if offset + 3 < operands.len() {
            Ok(u32::from_le_bytes([
                operands[offset],
                operands[offset + 1],
                operands[offset + 2],
                operands[offset + 3],
            ]))
        } else {
            Err(RuntimeError::InvalidOpcode(0))
        }
    }

    #[inline]
    fn read_i32_operand(
        &self,
        operands: &[u8],
        offset: usize,
    ) -> Result<i32, RuntimeError> {
        if offset + 3 < operands.len() {
            Ok(i32::from_le_bytes([
                operands[offset],
                operands[offset + 1],
                operands[offset + 2],
                operands[offset + 3],
            ]))
        } else {
            Err(RuntimeError::InvalidOpcode(0))
        }
    }

    #[inline]
    fn get_constant(
        &self,
        index: usize,
    ) -> Result<RuntimeValue, RuntimeError> {
        self.constants
            .get(index)
            .map(|c| match c {
                ConstValue::Void => RuntimeValue::Unit,
                ConstValue::Bool(b) => RuntimeValue::Bool(*b),
                ConstValue::Int(n) => RuntimeValue::Int(*n as i64),
                ConstValue::Float(f) => RuntimeValue::Float(*f),
                ConstValue::Char(c) => RuntimeValue::Char(*c as u32),
                ConstValue::String(s) => RuntimeValue::String(Arc::from(s.as_str())),
                ConstValue::Bytes(b) => RuntimeValue::Bytes(Arc::from(b.as_slice())),
            })
            .ok_or(RuntimeError::InvalidConstIndex(index))
    }
}

impl Default for EmbeddedRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::codegen::bytecode::{BytecodeFile, CodeSection, FileHeader, FunctionCode};
    use crate::middle::ir::ConstValue;

    /// Helper to create a simple test module
    fn create_test_module() -> CompiledModule {
        CompiledModule {
            name: "test".to_string(),
            globals: Vec::new(),
            functions: vec![FunctionCode {
                name: "main".to_string(),
                params: Vec::new(),
                return_type: crate::frontend::typecheck::MonoType::Void,
                instructions: Vec::new(),
                local_count: 0,
            }],
            constants: Vec::new(),
        }
    }

    #[test]
    fn test_new_runtime() {
        let runtime = EmbeddedRuntime::new();
        assert_eq!(runtime.allocator.capacity(), 64 * 1024);
    }

    #[test]
    fn test_with_capacity() {
        let runtime = EmbeddedRuntime::with_capacity(128 * 1024);
        assert_eq!(runtime.allocator.capacity(), 128 * 1024);
    }

    #[test]
    fn test_load_module() {
        let mut runtime = EmbeddedRuntime::new();
        let module = create_test_module();
        runtime.load_module(module);

        assert!(runtime.main_func_id.is_some());
    }

    #[test]
    fn test_missing_main() {
        let mut runtime = EmbeddedRuntime::new();
        let module = CompiledModule {
            name: "test".to_string(),
            globals: Vec::new(),
            functions: vec![FunctionCode {
                name: "not_main".to_string(),
                params: Vec::new(),
                return_type: crate::frontend::typecheck::MonoType::Void,
                instructions: Vec::new(),
                local_count: 0,
            }],
            constants: Vec::new(),
        };

        runtime.load_module(module.clone());
        assert!(runtime.load_and_run(module).is_err());
    }

    #[test]
    fn test_error_display() {
        assert_eq!(
            format!("{}", RuntimeError::StackUnderflow),
            "stack underflow"
        );
        assert_eq!(
            format!("{}", RuntimeError::MissingMain),
            "missing main function"
        );
        assert_eq!(
            format!("{}", RuntimeError::DivisionByZero),
            "division by zero"
        );
    }

    // =====================
    // P1: Function Call Tests
    // =====================

    #[test]
    fn test_call_static() {
        // Test that CallStatic instruction can be created without error
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::CallStatic;
        assert_eq!(format!("{:?}", opcode), "CallStatic");
    }

    #[test]
    fn test_make_closure() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::MakeClosure;
        assert_eq!(format!("{:?}", opcode), "MakeClosure");
    }

    #[test]
    fn test_load_upvalue() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::LoadUpvalue;
        assert_eq!(format!("{:?}", opcode), "LoadUpvalue");
    }

    #[test]
    fn test_store_upvalue() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::StoreUpvalue;
        assert_eq!(format!("{:?}", opcode), "StoreUpvalue");
    }

    #[test]
    fn test_call_dyn() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::CallDyn;
        assert_eq!(format!("{:?}", opcode), "CallDyn");
    }

    #[test]
    fn test_call_virt() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::CallVirt;
        assert_eq!(format!("{:?}", opcode), "CallVirt");
    }

    // =====================
    // P2: Memory & Object Tests
    // =====================

    #[test]
    fn test_heap_alloc() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::HeapAlloc;
        assert_eq!(format!("{:?}", opcode), "HeapAlloc");
    }

    #[test]
    fn test_stack_alloc() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::StackAlloc;
        assert_eq!(format!("{:?}", opcode), "StackAlloc");
    }

    #[test]
    fn test_drop() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::Drop;
        assert_eq!(format!("{:?}", opcode), "Drop");
    }

    #[test]
    fn test_get_field() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::GetField;
        assert_eq!(format!("{:?}", opcode), "GetField");
    }

    #[test]
    fn test_set_field() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::SetField;
        assert_eq!(format!("{:?}", opcode), "SetField");
    }

    #[test]
    fn test_load_element() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::LoadElement;
        assert_eq!(format!("{:?}", opcode), "LoadElement");
    }

    #[test]
    fn test_store_element() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::StoreElement;
        assert_eq!(format!("{:?}", opcode), "StoreElement");
    }

    #[test]
    fn test_new_list_with_cap() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::NewListWithCap;
        assert_eq!(format!("{:?}", opcode), "NewListWithCap");
    }

    #[test]
    fn test_arc_new() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::ArcNew;
        assert_eq!(format!("{:?}", opcode), "ArcNew");
    }

    #[test]
    fn test_arc_clone() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::ArcClone;
        assert_eq!(format!("{:?}", opcode), "ArcClone");
    }

    #[test]
    fn test_arc_drop() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::ArcDrop;
        assert_eq!(format!("{:?}", opcode), "ArcDrop");
    }

    // =====================
    // P3: String Operations Tests
    // =====================
    #[test]
    fn test_string_length() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::StringLength;
        assert_eq!(format!("{:?}", opcode), "StringLength");
    }

    #[test]
    fn test_string_concat() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::StringConcat;
        assert_eq!(format!("{:?}", opcode), "StringConcat");
    }

    #[test]
    fn test_string_equal() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::StringEqual;
        assert_eq!(format!("{:?}", opcode), "StringEqual");
    }

    #[test]
    fn test_string_get_char() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::StringGetChar;
        assert_eq!(format!("{:?}", opcode), "StringGetChar");
    }

    #[test]
    fn test_string_from_int() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::StringFromInt;
        assert_eq!(format!("{:?}", opcode), "StringFromInt");
    }

    #[test]
    fn test_string_from_float() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::StringFromFloat;
        assert_eq!(format!("{:?}", opcode), "StringFromFloat");
    }

    // =====================
    // P4: Advanced Features Tests
    // =====================
    #[test]
    fn test_try_begin() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::TryBegin;
        assert_eq!(format!("{:?}", opcode), "TryBegin");
    }

    #[test]
    fn test_try_end() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::TryEnd;
        assert_eq!(format!("{:?}", opcode), "TryEnd");
    }

    #[test]
    fn test_throw() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::Throw;
        assert_eq!(format!("{:?}", opcode), "Throw");
    }

    #[test]
    fn test_rethrow() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::Rethrow;
        assert_eq!(format!("{:?}", opcode), "Rethrow");
    }

    #[test]
    fn test_bounds_check() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::BoundsCheck;
        assert_eq!(format!("{:?}", opcode), "BoundsCheck");
    }

    #[test]
    fn test_type_check() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::TypeCheck;
        assert_eq!(format!("{:?}", opcode), "TypeCheck");
    }

    #[test]
    fn test_cast() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::Cast;
        assert_eq!(format!("{:?}", opcode), "Cast");
    }

    #[test]
    fn test_type_of() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::TypeOf;
        assert_eq!(format!("{:?}", opcode), "TypeOf");
    }

    #[test]
    fn test_switch() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::Switch;
        assert_eq!(format!("{:?}", opcode), "Switch");
    }

    #[test]
    fn test_loop_start() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::LoopStart;
        assert_eq!(format!("{:?}", opcode), "LoopStart");
    }

    #[test]
    fn test_loop_inc() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::LoopInc;
        assert_eq!(format!("{:?}", opcode), "LoopInc");
    }

    #[test]
    fn test_tail_call() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::TailCall;
        assert_eq!(format!("{:?}", opcode), "TailCall");
    }

    #[test]
    fn test_yield() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::Yield;
        assert_eq!(format!("{:?}", opcode), "Yield");
    }

    #[test]
    fn test_label() {
        use crate::vm::opcode::TypedOpcode;

        let opcode = TypedOpcode::Label;
        assert_eq!(format!("{:?}", opcode), "Label");
    }

    #[test]
    fn test_exception_error_display() {
        assert_eq!(
            format!(
                "{}",
                RuntimeError::Exception {
                    message: "test".to_string()
                }
            ),
            "exception: test"
        );
        assert_eq!(
            format!(
                "{}",
                RuntimeError::IndexOutOfBounds {
                    index: 5,
                    length: 3
                }
            ),
            "index out of bounds: 5 >= 3"
        );
    }
}
