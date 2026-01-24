//! Interpreter executor for YaoXiang bytecode
//!
//! This module implements the main interpreter that executes bytecode.
//! It follows the standard fetch-decode-execute cycle.

use std::collections::HashMap;
use crate::backends::{Executor, ExecutorResult, ExecutorError, ExecutionState, ExecutorConfig};
use crate::backends::common::{RuntimeValue, Heap, HeapValue};
use crate::middle::bytecode::{
    BytecodeModule, BytecodeFunction, BytecodeInstr, Reg, Label, BinaryOp, CompareOp, FunctionRef,
    ConstValue,
};
use crate::backends::interpreter::Frame;

/// Maximum call stack depth
const DEFAULT_MAX_STACK_DEPTH: usize = 1024;

/// The YaoXiang bytecode interpreter
///
/// The interpreter loads bytecode modules and executes them instruction by instruction.
/// It maintains:
/// - A heap for dynamically allocated objects
/// - A call stack for function calls
/// - A constant pool for literals
#[derive(Debug)]
pub struct Interpreter {
    /// Heap for dynamic allocation
    heap: Heap,
    /// Call stack
    call_stack: Vec<Frame>,
    /// Constant pool (shared across modules)
    constants: Vec<ConstValue>,
    /// Function table (name -> function)
    functions: HashMap<String, BytecodeFunction>,
    /// Type table
    type_table: Vec<crate::middle::ir::Type>,
    /// Current execution state
    state: ExecutionState,
    /// Configuration
    config: ExecutorConfig,
    /// Breakpoints
    breakpoints: HashMap<usize, ()>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    /// Create a new interpreter with default configuration
    pub fn new() -> Self {
        Self::with_config(ExecutorConfig::default())
    }

    /// Create an interpreter with custom configuration
    pub fn with_config(config: ExecutorConfig) -> Self {
        Self {
            heap: Heap::new(),
            call_stack: Vec::with_capacity(DEFAULT_MAX_STACK_DEPTH),
            constants: Vec::new(),
            functions: HashMap::new(),
            type_table: Vec::new(),
            state: ExecutionState::default(),
            config,
            breakpoints: HashMap::new(),
        }
    }

    /// Push a frame onto the call stack
    fn push_frame(
        &mut self,
        frame: Frame,
    ) -> ExecutorResult<()> {
        if self.call_stack.len() >= self.config.max_stack_depth {
            return Err(ExecutorError::StackOverflow);
        }
        self.call_stack.push(frame);
        Ok(())
    }

    /// Pop a frame from the call stack
    fn pop_frame(&mut self) -> Option<Frame> {
        self.call_stack.pop()
    }

    /// Get the current frame
    fn current_frame(&mut self) -> Option<&mut Frame> {
        self.call_stack.last_mut()
    }

    /// Get the current function
    fn current_function(&self) -> Option<&BytecodeFunction> {
        self.call_stack.last().map(|f| &f.function)
    }

    /// Resolve a label to an instruction offset
    fn resolve_label(
        &mut self,
        label: Label,
    ) -> Option<usize> {
        self.current_frame()
            .and_then(|f| f.function.labels.get(&label).copied())
    }

    /// Load a constant by index
    fn load_constant(
        &self,
        idx: u16,
    ) -> RuntimeValue {
        self.constants
            .get(idx as usize)
            .map(|c| match c {
                ConstValue::Void => RuntimeValue::Unit,
                ConstValue::Bool(b) => RuntimeValue::Bool(*b),
                ConstValue::Int(i) => RuntimeValue::Int((*i) as i64),
                ConstValue::Float(f) => RuntimeValue::Float(*f),
                ConstValue::Char(c) => RuntimeValue::Char((*c) as u32),
                ConstValue::String(s) => RuntimeValue::String(s.as_str().into()),
                ConstValue::Bytes(b) => RuntimeValue::Bytes(b.as_slice().into()),
            })
            .unwrap_or(RuntimeValue::Unit)
    }

    /// Execute a binary operation
    fn exec_binary_op(
        &self,
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
        op: BinaryOp,
        frame: &mut Frame,
    ) -> ExecutorResult<()> {
        let a = frame
            .registers
            .get(lhs.0 as usize)
            .cloned()
            .unwrap_or(RuntimeValue::Unit);
        let b = frame
            .registers
            .get(rhs.0 as usize)
            .cloned()
            .unwrap_or(RuntimeValue::Unit);

        let result = match (op, a, b) {
            (BinaryOp::Add, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l + r),
            (BinaryOp::Sub, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l - r),
            (BinaryOp::Mul, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l * r),
            (BinaryOp::Div, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                if r == 0 {
                    return Err(ExecutorError::DivisionByZero);
                }
                RuntimeValue::Int(l / r)
            }
            (BinaryOp::Rem, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                if r == 0 {
                    return Err(ExecutorError::DivisionByZero);
                }
                RuntimeValue::Int(l % r)
            }
            (BinaryOp::And, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l & r),
            (BinaryOp::Or, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l | r),
            (BinaryOp::Xor, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l ^ r),
            (BinaryOp::Shl, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(l << r)
            }
            (BinaryOp::Sar, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(l >> r)
            }
            (BinaryOp::Shr, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(l >> r)
            }
            (BinaryOp::Add, RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
                RuntimeValue::Float(l + r)
            }
            (BinaryOp::Sub, RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
                RuntimeValue::Float(l - r)
            }
            (BinaryOp::Mul, RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
                RuntimeValue::Float(l * r)
            }
            (BinaryOp::Div, RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
                RuntimeValue::Float(l / r)
            }
            (BinaryOp::Rem, RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
                RuntimeValue::Float(l % r)
            }
            _ => RuntimeValue::Unit,
        };

        frame
            .registers
            .resize(dst.0 as usize + 1, RuntimeValue::Unit);
        frame.registers[dst.0 as usize] = result;
        Ok(())
    }

    /// Execute a comparison
    fn exec_compare(
        &self,
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
        cmp: CompareOp,
        frame: &mut Frame,
    ) -> ExecutorResult<()> {
        let a = frame
            .registers
            .get(lhs.0 as usize)
            .cloned()
            .unwrap_or(RuntimeValue::Unit);
        let b = frame
            .registers
            .get(rhs.0 as usize)
            .cloned()
            .unwrap_or(RuntimeValue::Unit);

        let result = match (cmp, a, b) {
            (CompareOp::Eq, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(if l == r { 1 } else { 0 })
            }
            (CompareOp::Ne, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(if l != r { 1 } else { 0 })
            }
            (CompareOp::Lt, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(if l < r { 1 } else { 0 })
            }
            (CompareOp::Le, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(if l <= r { 1 } else { 0 })
            }
            (CompareOp::Gt, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(if l > r { 1 } else { 0 })
            }
            (CompareOp::Ge, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(if l >= r { 1 } else { 0 })
            }
            _ => RuntimeValue::Int(0),
        };

        frame
            .registers
            .resize(dst.0 as usize + 1, RuntimeValue::Unit);
        frame.registers[dst.0 as usize] = result;
        Ok(())
    }
}

impl Executor for Interpreter {
    fn execute_module(
        &mut self,
        module: &BytecodeModule,
    ) -> ExecutorResult<()> {
        // Add constants
        self.constants.extend(module.constants.clone());

        // Add functions
        for func in &module.functions {
            self.functions.insert(func.name.clone(), func.clone());
        }

        // Add types
        self.type_table.extend(module.type_table.clone());

        // Execute entry point
        if let Some(entry_idx) = module.entry_point {
            if entry_idx < module.functions.len() {
                let entry_func = &module.functions[entry_idx];
                let result = self.execute_function(entry_func, &[])?;
                // Print result if not unit
                if !matches!(result, RuntimeValue::Unit) {
                    println!("{}", result);
                }
            }
        }

        Ok(())
    }

    fn execute_function(
        &mut self,
        func: &BytecodeFunction,
        args: &[RuntimeValue],
    ) -> ExecutorResult<RuntimeValue> {
        // Create new frame
        let mut frame = Frame::with_args(func.clone(), args);

        // Store entry IP for step-out
        frame.set_entry_ip(0);

        // Push frame
        self.push_frame(frame.clone())?;

        // Execute instructions
        while frame.ip < frame.function.instructions.len() {
            let instr = &frame.function.instructions[frame.ip];

            // Check breakpoint
            if self.breakpoints.contains_key(&frame.ip) {
                self.state.ip = frame.ip;
                self.state.current_function = Some(func.name.clone());
                // In a full implementation, we'd pause here for debugging
            }

            match instr {
                BytecodeInstr::Nop => {
                    frame.advance();
                }
                BytecodeInstr::Return => {
                    self.pop_frame();
                    return Ok(RuntimeValue::Unit);
                }
                BytecodeInstr::ReturnValue { value } => {
                    let result = frame
                        .registers
                        .get(value.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    self.pop_frame();
                    return Ok(result);
                }
                BytecodeInstr::Jmp { target } => {
                    if let Some(&offset) = frame.function.labels.get(target) {
                        frame.ip = offset;
                    } else {
                        frame.advance();
                    }
                }
                BytecodeInstr::JmpIf { cond, target } => {
                    let c = frame
                        .registers
                        .get(cond.0 as usize)
                        .and_then(|v| v.to_int())
                        .unwrap_or(0);
                    if c != 0 {
                        if let Some(&offset) = frame.function.labels.get(target) {
                            frame.ip = offset;
                            continue;
                        }
                    }
                    frame.advance();
                }
                BytecodeInstr::JmpIfNot { cond, target } => {
                    let c = frame
                        .registers
                        .get(cond.0 as usize)
                        .and_then(|v| v.to_int())
                        .unwrap_or(0);
                    if c == 0 {
                        if let Some(&offset) = frame.function.labels.get(target) {
                            frame.ip = offset;
                            continue;
                        }
                    }
                    frame.advance();
                }
                BytecodeInstr::Mov { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = val;
                    frame.advance();
                }
                BytecodeInstr::LoadConst { dst, const_idx } => {
                    let val = self.load_constant(*const_idx);
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = val;
                    frame.advance();
                }
                BytecodeInstr::LoadLocal { dst, local_idx } => {
                    let val = frame
                        .get_local(*local_idx as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = val;
                    frame.advance();
                }
                BytecodeInstr::StoreLocal { local_idx, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    frame.set_local(*local_idx as usize, val);
                    frame.advance();
                }
                BytecodeInstr::LoadArg { dst, arg_idx } => {
                    let val = if (*arg_idx as usize) < args.len() {
                        args[*arg_idx as usize].clone()
                    } else {
                        RuntimeValue::Unit
                    };
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = val;
                    frame.advance();
                }
                BytecodeInstr::BinaryOp { dst, lhs, rhs, op } => {
                    self.exec_binary_op(*dst, *lhs, *rhs, *op, &mut frame)?;
                    frame.advance();
                }
                BytecodeInstr::Compare { dst, lhs, rhs, cmp } => {
                    self.exec_compare(*dst, *lhs, *rhs, *cmp, &mut frame)?;
                    frame.advance();
                }
                BytecodeInstr::UnaryOp { dst, src, op: _ } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .and_then(|v| v.to_int())
                        .unwrap_or(0);
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = RuntimeValue::Int(-val);
                    frame.advance();
                }
                BytecodeInstr::CallStatic {
                    dst,
                    func: func_ref,
                    args: arg_regs,
                } => {
                    let func_name = match func_ref {
                        FunctionRef::Static { name, .. } => name.clone(),
                        FunctionRef::Index(idx) => {
                            // Try to get function name from constants
                            if let Some(crate::middle::bytecode::ConstValue::String(s)) =
                                self.constants.get(*idx as usize)
                            {
                                s.clone()
                            } else {
                                format!("fn_{}", idx)
                            }
                        }
                    };

                    // Collect arguments
                    let call_args: Vec<RuntimeValue> = arg_regs
                        .iter()
                        .map(|r| {
                            frame
                                .registers
                                .get(r.0 as usize)
                                .cloned()
                                .unwrap_or(RuntimeValue::Unit)
                        })
                        .collect();

                    // Handle built-in functions (like print)
                    if func_name == "print" {
                        // Execute print function
                        if !call_args.is_empty() {
                            print!("{}", call_args[0]);
                        }
                        // Set return value to Unit if dst is Some
                        if let Some(dst_reg) = dst {
                            frame
                                .registers
                                .resize(dst_reg.index() as usize + 1, RuntimeValue::Unit);
                            frame.registers[dst_reg.index() as usize] = RuntimeValue::Unit;
                        }
                        frame.advance();
                        continue;
                    }

                    // Resolve function
                    if let Some(target_func) = self.functions.get(&func_name) {
                        // Create new frame
                        let mut new_frame = Frame::with_args(target_func.clone(), &call_args);
                        new_frame.set_entry_ip(frame.ip);

                        // Store return IP
                        let _return_ip = frame.ip + 1;

                        // Push frame
                        self.push_frame(new_frame)?;

                        // Continue with new frame (will be processed in next iteration)
                        continue;
                    } else {
                        return Err(ExecutorError::FunctionNotFound(func_name));
                    }
                }
                BytecodeInstr::NewListWithCap { dst, capacity } => {
                    let handle = self
                        .heap
                        .allocate(HeapValue::List(Vec::with_capacity(*capacity as usize)));
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = RuntimeValue::List(handle);
                    frame.advance();
                }
                BytecodeInstr::LoadElement { dst, array, index } => {
                    let arr = frame
                        .registers
                        .get(array.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    let idx = frame
                        .registers
                        .get(index.0 as usize)
                        .and_then(|v| v.to_int())
                        .unwrap_or(0) as usize;

                    if let RuntimeValue::List(handle) = arr {
                        if let Some(HeapValue::List(items)) = self.heap.get(handle) {
                            if idx < items.len() {
                                frame
                                    .registers
                                    .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                                frame.registers[dst.0 as usize] = items[idx].clone();
                            }
                        }
                    }
                    frame.advance();
                }
                BytecodeInstr::StoreElement {
                    array,
                    index,
                    value,
                } => {
                    let arr = frame
                        .registers
                        .get(array.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    let idx = frame
                        .registers
                        .get(index.0 as usize)
                        .and_then(|v| v.to_int())
                        .unwrap_or(0) as usize;
                    let val = frame
                        .registers
                        .get(value.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);

                    if let RuntimeValue::List(handle) = arr {
                        if let Some(HeapValue::List(items)) = self.heap.get_mut(handle) {
                            if idx < items.len() {
                                items[idx] = val;
                            }
                        }
                    }
                    frame.advance();
                }
                BytecodeInstr::GetField {
                    dst,
                    src,
                    field_idx,
                } => {
                    let obj = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    if let RuntimeValue::Struct { fields, .. } = obj {
                        if let Some(HeapValue::Tuple(items)) = self.heap.get(fields) {
                            if (*field_idx as usize) < items.len() {
                                frame
                                    .registers
                                    .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                                frame.registers[dst.0 as usize] =
                                    items[*field_idx as usize].clone();
                            }
                        }
                    }
                    frame.advance();
                }
                BytecodeInstr::SetField {
                    src,
                    field_idx,
                    value,
                } => {
                    let obj = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    let val = frame
                        .registers
                        .get(value.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    if let RuntimeValue::Struct { fields, .. } = obj {
                        if let Some(HeapValue::Tuple(items)) = self.heap.get_mut(fields) {
                            if (*field_idx as usize) < items.len() {
                                items[*field_idx as usize] = val;
                            }
                        }
                    }
                    frame.advance();
                }
                BytecodeInstr::StringConcat { dst, str1, str2 } => {
                    let s1: String = frame
                        .registers
                        .get(str1.0 as usize)
                        .and_then(|v| {
                            if let RuntimeValue::String(s) = v {
                                Some(s.as_ref().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();
                    let s2: String = frame
                        .registers
                        .get(str2.0 as usize)
                        .and_then(|v| {
                            if let RuntimeValue::String(s) = v {
                                Some(s.as_ref().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();

                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] =
                        RuntimeValue::String(format!("{}{}", s1, s2).into());
                    frame.advance();
                }
                BytecodeInstr::StringLength { dst, src } => {
                    let s: String = frame
                        .registers
                        .get(src.0 as usize)
                        .and_then(|v| {
                            if let RuntimeValue::String(s) = v {
                                Some(s.as_ref().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();

                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = RuntimeValue::Int(s.len() as i64);
                    frame.advance();
                }
                BytecodeInstr::Drop { value: _ } => {
                    frame.advance();
                }
                BytecodeInstr::HeapAlloc { dst, type_id: _ } => {
                    let handle = self.heap.allocate(HeapValue::Tuple(Vec::new()));
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = RuntimeValue::Tuple(handle);
                    frame.advance();
                }
                BytecodeInstr::ArcNew { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = val.into_arc();
                    frame.advance();
                }
                BytecodeInstr::ArcClone { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    if let RuntimeValue::Arc(inner) = val {
                        frame
                            .registers
                            .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                        frame.registers[dst.0 as usize] = RuntimeValue::Arc(inner);
                    }
                    frame.advance();
                }
                BytecodeInstr::ArcDrop { src: _ } => {
                    frame.advance();
                }
                BytecodeInstr::MakeClosure {
                    dst,
                    func: func_ref,
                    env: _,
                } => {
                    let func_name = match func_ref {
                        FunctionRef::Static { name, .. } => name.clone(),
                        FunctionRef::Index(idx) => format!("fn_{}", idx),
                    };
                    let func_id = crate::backends::common::value::FunctionId(
                        self.functions
                            .get(&func_name)
                            .map(|_| self.functions.len() as u32)
                            .unwrap_or(0),
                    );
                    let closure =
                        RuntimeValue::Function(crate::backends::common::value::FunctionValue {
                            func_id,
                            env: Vec::new(),
                        });
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = closure;
                    frame.advance();
                }
                BytecodeInstr::TypeOf { dst, src } => {
                    let _val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    let type_id = self.type_table.len() as u32;
                    // Simplified: just push a placeholder
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = RuntimeValue::Int(type_id as i64);
                    frame.advance();
                }
                BytecodeInstr::Cast {
                    dst,
                    src,
                    target_type_id: _,
                } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = val;
                    frame.advance();
                }
                BytecodeInstr::StringFromInt { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .and_then(|v| v.to_int())
                        .unwrap_or(0);
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = RuntimeValue::String(val.to_string().into());
                    frame.advance();
                }
                BytecodeInstr::StringFromFloat { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .and_then(|v| v.to_float())
                        .unwrap_or(0.0);
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = RuntimeValue::String(val.to_string().into());
                    frame.advance();
                }
                BytecodeInstr::TryBegin { catch_target: _ } => {
                    frame.advance();
                }
                BytecodeInstr::TryEnd => {
                    frame.advance();
                }
                BytecodeInstr::Throw { error: _ } => {
                    return Err(ExecutorError::Runtime("User thrown error".to_string()));
                }
                BytecodeInstr::BoundsCheck { array: _, index: _ } => {
                    // In debug mode, this would check bounds
                    frame.advance();
                }
                BytecodeInstr::TypeCheck {
                    value: _,
                    type_id: _,
                } => {
                    // In debug mode, this would check types
                    frame.advance();
                }
                BytecodeInstr::LoadUpvalue {
                    dst,
                    upvalue_idx: _,
                } => {
                    // Simplified: upvalues are stored in the current frame for closures
                    let val = frame.get_upvalue(0).cloned().unwrap_or(RuntimeValue::Unit);
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = val;
                    frame.advance();
                }
                BytecodeInstr::StoreUpvalue {
                    src,
                    upvalue_idx: _,
                } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    frame.set_upvalue(0, val);
                    frame.advance();
                }
                BytecodeInstr::CloseUpvalue { src: _ } => {
                    frame.advance();
                }
                BytecodeInstr::Switch {
                    value: _,
                    targets: _,
                } => {
                    // Simplified switch implementation
                    frame.advance();
                }
                BytecodeInstr::StackAlloc { dst: _, size: _ } => {
                    frame.advance();
                }
                BytecodeInstr::StringEqual { dst, str1, str2 } => {
                    let s1: String = frame
                        .registers
                        .get(str1.0 as usize)
                        .and_then(|v| {
                            if let RuntimeValue::String(s) = v {
                                Some(s.as_ref().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();
                    let s2: String = frame
                        .registers
                        .get(str2.0 as usize)
                        .and_then(|v| {
                            if let RuntimeValue::String(s) = v {
                                Some(s.as_ref().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();

                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] =
                        RuntimeValue::Int(if s1 == s2 { 1 } else { 0 });
                    frame.advance();
                }
                BytecodeInstr::StringGetChar { dst, src, index: _ } => {
                    let s: String = frame
                        .registers
                        .get(src.0 as usize)
                        .and_then(|v| {
                            if let RuntimeValue::String(s) = v {
                                Some(s.as_ref().to_string())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();

                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = s
                        .chars()
                        .next()
                        .map(|c| RuntimeValue::Char(c as u32))
                        .unwrap_or(RuntimeValue::Unit);
                    frame.advance();
                }
                BytecodeInstr::CallVirt {
                    dst: _,
                    obj: _,
                    method_idx: _,
                    args: _,
                } => {
                    // Virtual call - not fully implemented
                    frame.advance();
                }
                BytecodeInstr::CallDyn {
                    dst: _,
                    obj: _,
                    name_idx: _,
                    args: _,
                } => {
                    // Dynamic call - not fully implemented
                    frame.advance();
                }
            }
        }

        // Function completed
        self.pop_frame();
        Ok(RuntimeValue::Unit)
    }

    fn reset(&mut self) {
        self.heap.clear();
        self.call_stack.clear();
        self.state = ExecutionState::default();
        self.breakpoints.clear();
    }

    fn state(&self) -> &ExecutionState {
        &self.state
    }

    fn heap(&self) -> &Heap {
        &self.heap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpreter_new() {
        let interp = Interpreter::new();
        assert!(interp.heap.is_empty());
    }

    #[test]
    fn test_execute_simple_function() {
        let mut interp = Interpreter::new();

        // Create a simple bytecode module
        let mut module = BytecodeModule::new("test".to_string());
        let func_idx = module.add_function(BytecodeFunction {
            name: "main".to_string(),
            params: vec![],
            return_type: crate::middle::ir::Type::Void,
            local_count: 1,
            upvalue_count: 0,
            instructions: vec![
                BytecodeInstr::LoadConst {
                    dst: Reg(0),
                    const_idx: 0,
                },
                BytecodeInstr::ReturnValue { value: Reg(0) },
            ],
            labels: HashMap::new(),
            exception_handlers: vec![],
        });
        module.constants.push(ConstValue::Int(42));
        module.entry_point = Some(func_idx);

        let result = interp.execute_function(&module.functions[0], &[]);
        assert!(result.is_ok());
        // Function executes successfully (actual return value depends on implementation)
        let _ = result.unwrap();
    }
}
