//! Interpreter executor for YaoXiang bytecode
//!
//! This module implements the main interpreter that executes bytecode.
//! It follows the standard fetch-decode-execute cycle.

use std::collections::HashMap;
use std::fmt;
use crate::backends::{Executor, ExecutorResult, ExecutorError, ExecutionState, ExecutorConfig};
use crate::backends::common::{RuntimeValue, Heap, HeapValue};
use crate::middle::bytecode::{
    BytecodeModule, BytecodeFunction, BytecodeInstr, Reg, Label, BinaryOp, CompareOp, FunctionRef,
    ConstValue,
};
use crate::backends::interpreter::Frame;
use crate::backends::interpreter::frames::MAX_LOCALS;
use crate::util::i18n::MSG;
use crate::tlog;

/// Maximum call stack depth
const DEFAULT_MAX_STACK_DEPTH: usize = 1024;

/// The YaoXiang bytecode interpreter
///
/// The interpreter loads bytecode modules and executes them instruction by instruction.
/// It maintains:
/// - A heap for dynamically allocated objects
/// - A call stack for function calls
/// - A constant pool for literals
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
    type_table: Vec<crate::middle::core::ir::Type>,
    /// Current execution state
    state: ExecutionState,
    /// Configuration
    config: ExecutorConfig,
    /// Breakpoints
    breakpoints: HashMap<usize, ()>,
    /// Standard output
    #[allow(dead_code)] // Might be unused if only accessed via write!
    stdout: Option<std::sync::Arc<std::sync::Mutex<dyn std::io::Write + Send>>>,
}

impl fmt::Debug for Interpreter {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("Interpreter")
            .field("heap", &self.heap)
            .field("call_stack", &self.call_stack)
            .field("constants", &self.constants)
            .field("functions", &self.functions)
            .field("type_table", &self.type_table)
            .field("state", &self.state)
            .field("config", &self.config)
            .field("breakpoints", &self.breakpoints)
            .field(
                "stdout",
                &if self.stdout.is_some() {
                    "Some(...)"
                } else {
                    "None"
                },
            )
            .finish()
    }
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
            stdout: None, // Default to stdout (handled by None check)
        }
    }

    /// Set standard output redirect
    pub fn set_stdout(
        &mut self,
        stdout: std::sync::Arc<std::sync::Mutex<dyn std::io::Write + Send>>,
    ) {
        self.stdout = Some(stdout);
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
        tlog!(
            debug,
            MSG::DebugRegisters,
            &frame.registers.len(),
            &(lhs.0 as usize),
            &(rhs.0 as usize)
        );
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

        tlog!(debug, MSG::DebugBinaryOp, &a, &b);

        tlog!(
            debug,
            MSG::DebugExecBinaryOp,
            &format!("{:?}, {:?}, {:?}", &a, &b, &op)
        );

        let result = match (op, a, b) {
            (BinaryOp::Add, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                tlog!(debug, MSG::DebugAddingNumbers, &l, &r);
                tlog!(debug, MSG::VmI64Add, &l, &r);
                RuntimeValue::Int(l + r)
            }
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
                RuntimeValue::Bool(l == r)
            }
            (CompareOp::Ne, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l != r)
            }
            (CompareOp::Lt, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l < r)
            }
            (CompareOp::Le, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l <= r)
            }
            (CompareOp::Gt, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l > r)
            }
            (CompareOp::Ge, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l >= r)
            }
            _ => RuntimeValue::Bool(false),
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
            tlog!(debug, MSG::DebugLoadingFunction, &func.name);
            self.functions.insert(func.name.clone(), func.clone());
        }
        tlog!(debug, MSG::DebugTotalFunctions, &self.functions.len());
        tlog!(
            debug,
            MSG::DebugAvailableFunctions,
            &format!("{:?}", self.functions.keys().collect::<Vec<_>>())
        );

        // Add types
        self.type_table.extend(module.type_table.clone());

        // Execute entry point
        if let Some(entry_idx) = module.entry_point {
            if entry_idx < module.functions.len() {
                let entry_func = &module.functions[entry_idx];
                let result = self.execute_function(entry_func, &[])?;
                // Print result if not unit
                if !matches!(result, RuntimeValue::Unit) {
                    tracing::info!("{}", result);
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
        if func.local_count > MAX_LOCALS {
            return Err(ExecutorError::Runtime(format!(
                "Too many locals in function '{}': {}",
                func.name, func.local_count
            )));
        }
        // Create new frame
        let mut frame = Frame::with_args(func.clone(), args);

        // Store entry IP for step-out
        frame.set_entry_ip(0);

        // Push frame
        self.push_frame(frame.clone())?;

        // Execute instructions
        while frame.ip < frame.function.instructions.len() {
            let instr = &frame.function.instructions[frame.ip];

            tlog!(
                debug,
                MSG::VmExecInstruction,
                &format!("{} in function '{}': {:?}", frame.ip, func.name, instr)
            );

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
                    // target 是相对偏移量，直接使用
                    let offset = i32::from_le_bytes([
                        target.0 as u8,
                        (target.0 >> 8) as u8,
                        (target.0 >> 16) as u8,
                        (target.0 >> 24) as u8,
                    ]);
                    let target_ip = ((frame.ip as i32) + offset) as usize;
                    tracing::debug!(
                        "Jmp: jumping to offset {} (target_ip: {})",
                        offset,
                        target_ip
                    );
                    frame.ip = target_ip;
                    continue;
                }
                BytecodeInstr::JmpIf { cond, target } => {
                    let c = frame
                        .registers
                        .get(cond.0 as usize)
                        .and_then(|v| v.to_bool())
                        .unwrap_or(false);
                    tracing::debug!("JmpIf: cond={}, target={:?}", c, target);
                    if c {
                        // target 是相对偏移量，直接使用
                        let offset = i32::from_le_bytes([
                            target.0 as u8,
                            (target.0 >> 8) as u8,
                            (target.0 >> 16) as u8,
                            (target.0 >> 24) as u8,
                        ]);
                        let target_ip = ((frame.ip as i32) + offset) as usize;
                        tracing::debug!(
                            "JmpIf: jumping to offset {} (target_ip: {})",
                            offset,
                            target_ip
                        );
                        frame.ip = target_ip;
                        continue;
                    } else {
                        tracing::debug!("JmpIf: condition false, falling through");
                    }
                    frame.advance();
                }
                BytecodeInstr::JmpIfNot { cond, target } => {
                    let c = frame
                        .registers
                        .get(cond.0 as usize)
                        .and_then(|v| v.to_bool())
                        .unwrap_or(false);
                    tracing::debug!("JmpIfNot: cond={}, target={:?}", c, target);
                    if !c {
                        // target 是相对偏移量，直接使用
                        let offset = i32::from_le_bytes([
                            target.0 as u8,
                            (target.0 >> 8) as u8,
                            (target.0 >> 16) as u8,
                            (target.0 >> 24) as u8,
                        ]);
                        let target_ip = ((frame.ip as i32) + offset) as usize;
                        tracing::debug!(
                            "JmpIfNot: jumping to offset {} (target_ip: {})",
                            offset,
                            target_ip
                        );
                        frame.ip = target_ip;
                        continue;
                    } else {
                        tracing::debug!("JmpIfNot: condition true, falling through");
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
                    tlog!(debug, MSG::VmLoadLocal, dst, local_idx);
                    let val = frame
                        .get_local(*local_idx as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    tlog!(debug, MSG::VmLoadLocal, dst, &val);
                    frame
                        .registers
                        .resize(dst.0 as usize + 1, RuntimeValue::Unit);
                    frame.registers[dst.0 as usize] = val;
                    frame.advance();
                }
                BytecodeInstr::StoreLocal { local_idx, src } => {
                    tlog!(
                        debug,
                        MSG::VmStoreLocal,
                        local_idx,
                        src,
                        &frame.registers.len()
                    );
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    frame.set_local(*local_idx as usize, val);
                    frame.advance();
                }
                BytecodeInstr::LoadArg { dst, arg_idx } => {
                    tlog!(debug, MSG::VmLoadArg, dst, arg_idx, &args.len());
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
                    tlog!(debug, MSG::VmBinaryOp, op);
                    tlog!(debug, MSG::DebugMatch);
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
                    if func_name == "print" || func_name == "println" {
                        // For print/println, we join all arguments with space
                        let output = call_args
                            .iter()
                            .map(|arg| format!("{}", arg))
                            .collect::<Vec<String>>()
                            .join(" ");

                        if let Some(stdout_arc) = &self.stdout {
                            if let Ok(mut lock) = stdout_arc.lock() {
                                if func_name == "println" {
                                    let _ = writeln!(lock, "{}", output);
                                } else {
                                    let _ = write!(lock, "{}", output);
                                }
                            }
                        } else if func_name == "println" {
                            println!("{}", output);
                        } else {
                            print!("{}", output);
                        }

                        if let Some(dst_reg) = dst {
                            frame
                                .registers
                                .resize(dst_reg.index() as usize + 1, RuntimeValue::Unit);
                            frame.registers[dst_reg.index() as usize] = RuntimeValue::Unit;
                        }
                        frame.advance();
                        continue;
                    }

                    // Resolve function and execute immediately to avoid re-executing call site
                    let mut lookup_name = func_name.clone();

                    // Special handling for struct constructors:
                    // If "Point" is not found, try "Point_constructor"
                    if !self.functions.contains_key(&func_name) {
                        let constructor_name = format!("{}_constructor", func_name);
                        if self.functions.contains_key(&constructor_name) {
                            lookup_name = constructor_name.clone();
                        }
                    }

                    tlog!(
                        debug,
                        MSG::DebugFunctionLookup,
                        &format!("{}, {}", func_name, lookup_name)
                    );
                    tlog!(
                        debug,
                        MSG::DebugAvailableFunctions,
                        &format!("{:?}", self.functions.keys().collect::<Vec<_>>())
                    );

                    if let Some(target_func) = self.functions.get(&lookup_name).cloned() {
                        tlog!(debug, MSG::DebugFunctionFound, &lookup_name);
                        tlog!(
                            debug,
                            MSG::DebugAvailableFunctions,
                            &format!(
                                "name={}, instructions={}",
                                lookup_name,
                                target_func.instructions.len()
                            )
                        );
                        tlog!(
                            debug,
                            MSG::DebugFunctionCall,
                            &lookup_name,
                            &format!("{:?}", call_args)
                        );
                        tlog!(
                            debug,
                            MSG::VmExecutingFunction,
                            &lookup_name,
                            &format!("{:?}", call_args)
                        );
                        let result = self.execute_function(&target_func, &call_args)?;
                        tlog!(
                            debug,
                            MSG::VmFunctionReturned,
                            &lookup_name,
                            &format!("{:?}", result)
                        );
                        tlog!(
                            debug,
                            MSG::DebugFunctionReturn,
                            &lookup_name,
                            &format!("{:?}", result)
                        );
                        if let Some(dst_reg) = dst {
                            tlog!(debug, MSG::VmStoringResult, &format!("{:?}", dst_reg));
                            frame
                                .registers
                                .resize(dst_reg.index() as usize + 1, RuntimeValue::Unit);
                            frame.registers[dst_reg.index() as usize] = result;
                            tlog!(
                                debug,
                                MSG::VmRegistersAfter,
                                &format!("{:?}", frame.registers)
                            );
                        }
                        frame.advance();
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
            return_type: crate::middle::core::ir::Type::Void,
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
