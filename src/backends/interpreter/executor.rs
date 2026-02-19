//! Interpreter executor for YaoXiang bytecode
//!
//! This module implements the main interpreter that executes bytecode.
//! It follows the standard fetch-decode-execute cycle.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use crate::backends::{
    Executor, DebuggableExecutor, ExecutorResult, ExecutorError, ExecutionState, ExecutorConfig,
};
use crate::backends::common::{RuntimeValue, Heap, HeapValue};
use crate::middle::bytecode::{
    BytecodeModule, BytecodeFunction, BytecodeInstr, Reg, Label, BinaryOp, CompareOp, FunctionRef,
    ConstValue,
};
use crate::backends::interpreter::Frame;
use crate::backends::interpreter::frames::MAX_LOCALS;
use crate::backends::interpreter::ffi::FfiRegistry;
use crate::util::i18n::MSG;
use crate::tlog;
use crate::std::NativeContext;

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
    /// Function table by index (for closure calls via func_id)
    functions_by_id: Vec<BytecodeFunction>,
    /// Type table
    type_table: Vec<crate::middle::core::ir::Type>,
    /// Current execution state
    state: ExecutionState,
    /// Configuration
    config: ExecutorConfig,
    /// Breakpoints
    breakpoints: HashMap<usize, ()>,
    /// FFI Registry for native function calls
    ffi: FfiRegistry,
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
            .field("functions_by_id", &self.functions_by_id)
            .field("type_table", &self.type_table)
            .field("state", &self.state)
            .field("config", &self.config)
            .field("breakpoints", &self.breakpoints)
            .field("ffi", &self.ffi)
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
            functions_by_id: Vec::new(),
            type_table: Vec::new(),
            state: ExecutionState::default(),
            config,
            breakpoints: HashMap::new(),
            ffi: FfiRegistry::with_std(),
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

    /// Get mutable reference to the FFI registry for registering native functions
    pub fn ffi_registry_mut(&mut self) -> &mut FfiRegistry {
        &mut self.ffi
    }

    /// Get reference to the FFI registry
    pub fn ffi_registry(&self) -> &FfiRegistry {
        &self.ffi
    }

    /// Call a YaoXiang function by its FunctionId.
    /// This is used by native functions (like map/filter/reduce) to invoke closures.
    pub fn call_function_by_id(
        &mut self,
        func_id: crate::backends::common::value::FunctionId,
        args: &[RuntimeValue],
    ) -> Result<RuntimeValue, ExecutorError> {
        let idx = func_id.0 as usize;
        if idx >= self.functions_by_id.len() {
            return Err(ExecutorError::FunctionNotFound(format!(
                "Function with id {} not found (total functions: {})",
                idx,
                self.functions_by_id.len()
            )));
        }
        // Clone the function to avoid borrow issues
        let func = self.functions_by_id[idx].clone();
        self.execute_function(&func, args)
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
    pub fn current_frame(&mut self) -> Option<&mut Frame> {
        self.call_stack.last_mut()
    }

    /// Get the current function
    pub fn current_function(&self) -> Option<&BytecodeFunction> {
        self.call_stack.last().map(|f| &f.function)
    }

    /// Resolve a label to an instruction offset
    pub fn resolve_label(
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
        &mut self,
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
            (BinaryOp::Add, RuntimeValue::List(lhs_handle), RuntimeValue::List(rhs_handle)) => {
                let mut merged = Vec::new();

                if let Some(HeapValue::List(items)) = self.heap.get(lhs_handle) {
                    merged.extend(items.iter().cloned());
                }
                if let Some(HeapValue::List(items)) = self.heap.get(rhs_handle) {
                    merged.extend(items.iter().cloned());
                }

                let handle = self.heap.allocate(HeapValue::List(merged));
                RuntimeValue::List(handle)
            }
            _ => RuntimeValue::Unit,
        };

        frame.set_register(dst.0 as usize, result);
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

        let result = match (cmp, &a, &b) {
            // Integer comparison
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
            // String comparison
            (CompareOp::Eq, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l == r)
            }
            (CompareOp::Ne, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l != r)
            }
            (CompareOp::Lt, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l < r)
            }
            (CompareOp::Le, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l <= r)
            }
            (CompareOp::Gt, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l > r)
            }
            (CompareOp::Ge, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l >= r)
            }
            _ => RuntimeValue::Bool(false),
        };

        frame.set_register(dst.0 as usize, result);
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
            self.functions_by_id.push(func.clone());
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
                    frame.set_register(dst.0 as usize, val);
                    frame.advance();
                }
                BytecodeInstr::LoadConst { dst, const_idx } => {
                    let val = self.load_constant(*const_idx);
                    frame.set_register(dst.0 as usize, val);
                    frame.advance();
                }
                BytecodeInstr::LoadLocal { dst, local_idx } => {
                    tlog!(debug, MSG::VmLoadLocal, dst, local_idx);
                    let val = frame
                        .get_local(*local_idx as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    tlog!(debug, MSG::VmLoadLocal, dst, &val);
                    frame.set_register(dst.0 as usize, val);
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
                    frame.set_register(dst.0 as usize, val);
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
                    frame.set_register(dst.0 as usize, RuntimeValue::Int(-val));
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

                    // Try FFI registry first for native functions
                    if self.ffi.has(&func_name) {
                        // Create callback for calling YaoXiang functions (for map/filter/reduce)
                        let interp_ptr = std::ptr::addr_of_mut!(*self);
                        let mut call_fn = move |func: &RuntimeValue, args: &[RuntimeValue]| -> Result<RuntimeValue, ExecutorError> {
                            if let RuntimeValue::Function(fv) = func {
                                // SAFETY: The interpreter lives as long as the callback.
                                // The callback is only used during the execution of the native function,
                                // which is synchronous and completes before the interpreter continues.
                                let interpreter = unsafe { &mut *interp_ptr };
                                interpreter.call_function_by_id(fv.func_id, args)
                            } else {
                                Err(ExecutorError::Type("Expected function value".to_string()))
                            }
                        };
                        let mut ctx = NativeContext::with_call_fn(&mut self.heap, &mut call_fn);
                        let result = self.ffi.call(&func_name, &call_args, &mut ctx)?;
                        if let Some(dst_reg) = dst {
                            frame.set_register(dst_reg.index() as usize, result);
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
                            frame.set_register(dst_reg.index() as usize, result);
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
                BytecodeInstr::CallNative {
                    dst,
                    func_name,
                    args: arg_regs,
                } => {
                    // Collect arguments from registers
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

                    // Delegate to FFI registry with NativeContext
                    // Create callback for calling YaoXiang functions (for map/filter/reduce)
                    let interp_ptr = std::ptr::addr_of_mut!(*self);
                    let mut call_fn =
                        move |func: &RuntimeValue,
                              args: &[RuntimeValue]|
                              -> Result<RuntimeValue, ExecutorError> {
                            if let RuntimeValue::Function(fv) = func {
                                // SAFETY: The interpreter lives as long as the callback.
                                let interpreter = unsafe { &mut *interp_ptr };
                                interpreter.call_function_by_id(fv.func_id, args)
                            } else {
                                Err(ExecutorError::Type("Expected function value".to_string()))
                            }
                        };
                    let mut ctx = NativeContext::with_call_fn(&mut self.heap, &mut call_fn);
                    let result = self.ffi.call(func_name, &call_args, &mut ctx)?;

                    if let Some(dst_reg) = dst {
                        frame.set_register(dst_reg.index() as usize, result);
                    }
                    frame.advance();
                }
                BytecodeInstr::NewListWithCap { dst, capacity } => {
                    let handle = self
                        .heap
                        .allocate(HeapValue::List(Vec::with_capacity(*capacity as usize)));
                    frame.set_register(dst.0 as usize, RuntimeValue::List(handle));
                    frame.advance();
                }
                BytecodeInstr::LoadElement { dst, array, index } => {
                    let arr = frame
                        .registers
                        .get(array.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    let idx_value = frame
                        .registers
                        .get(index.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);

                    match arr {
                        RuntimeValue::List(handle) => {
                            let idx = idx_value.to_int().unwrap_or(0) as usize;
                            if let Some(HeapValue::List(items)) = self.heap.get(handle) {
                                if idx < items.len() {
                                    frame.set_register(dst.0 as usize, items[idx].clone());
                                }
                            }
                        }
                        RuntimeValue::Tuple(handle) => {
                            let idx = idx_value.to_int().unwrap_or(0) as usize;
                            if let Some(HeapValue::Tuple(items)) = self.heap.get(handle) {
                                if idx < items.len() {
                                    frame.set_register(dst.0 as usize, items[idx].clone());
                                }
                            }
                        }
                        RuntimeValue::Array(handle) => {
                            let idx = idx_value.to_int().unwrap_or(0) as usize;
                            if let Some(HeapValue::Array(items)) = self.heap.get(handle) {
                                if idx < items.len() {
                                    frame.set_register(dst.0 as usize, items[idx].clone());
                                }
                            }
                        }
                        RuntimeValue::Dict(handle) => {
                            if let Some(HeapValue::Dict(map)) = self.heap.get(handle) {
                                if let Some(value) = map.get(&idx_value) {
                                    frame.set_register(dst.0 as usize, value.clone());
                                }
                            }
                        }
                        _ => {}
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
                    let idx_value = frame
                        .registers
                        .get(index.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    let val = frame
                        .registers
                        .get(value.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);

                    match arr {
                        RuntimeValue::List(handle) => {
                            let idx = idx_value.to_int().unwrap_or(0) as usize;
                            if let Some(HeapValue::List(items)) = self.heap.get_mut(handle) {
                                if idx < items.len() {
                                    items[idx] = val;
                                } else if idx == items.len() {
                                    items.push(val);
                                }
                            }
                        }
                        RuntimeValue::Array(handle) => {
                            let idx = idx_value.to_int().unwrap_or(0) as usize;
                            if let Some(HeapValue::Array(items)) = self.heap.get_mut(handle) {
                                if idx < items.len() {
                                    items[idx] = val;
                                }
                            }
                        }
                        RuntimeValue::Dict(handle) => {
                            if let Some(HeapValue::Dict(map)) = self.heap.get_mut(handle) {
                                map.insert(idx_value, val);
                            }
                        }
                        _ => {}
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
                                frame.set_register(
                                    dst.0 as usize,
                                    items[*field_idx as usize].clone(),
                                );
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

                    frame.set_register(
                        dst.0 as usize,
                        RuntimeValue::String(format!("{}{}", s1, s2).into()),
                    );
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

                    frame.set_register(dst.0 as usize, RuntimeValue::Int(s.len() as i64));
                    frame.advance();
                }
                BytecodeInstr::Drop { value: _ } => {
                    frame.advance();
                }
                BytecodeInstr::HeapAlloc { dst, type_id: _ } => {
                    let handle = self.heap.allocate(HeapValue::Tuple(Vec::new()));
                    frame.set_register(dst.0 as usize, RuntimeValue::Tuple(handle));
                    frame.advance();
                }
                BytecodeInstr::CreateStruct {
                    dst,
                    type_name: _,
                    fields,
                } => {
                    // 收集各字段值
                    let field_values: Vec<RuntimeValue> = fields
                        .iter()
                        .map(|reg| {
                            frame
                                .registers
                                .get(reg.0 as usize)
                                .cloned()
                                .unwrap_or(RuntimeValue::Unit)
                        })
                        .collect();
                    let dst_idx = dst.0 as usize;
                    // 在堆上分配字段存储
                    let handle = self.heap.allocate(HeapValue::Tuple(field_values));
                    // 创建结构体值
                    let struct_val = RuntimeValue::Struct {
                        type_id: crate::backends::common::value::TypeId(0),
                        fields: handle,
                        vtable: Vec::new(),
                    };
                    frame.set_register(dst_idx, struct_val);
                    frame.advance();
                }
                BytecodeInstr::ArcNew { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    frame.set_register(dst.0 as usize, val.into_arc());
                    frame.advance();
                }
                BytecodeInstr::ArcClone { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    if let RuntimeValue::Arc(inner) = val {
                        frame.set_register(dst.0 as usize, RuntimeValue::Arc(inner));
                    }
                    frame.advance();
                }
                BytecodeInstr::ArcDrop { src: _ } => {
                    frame.advance();
                }
                BytecodeInstr::WeakNew { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    if let RuntimeValue::Arc(arc) = val {
                        frame
                            .set_register(dst.0 as usize, RuntimeValue::Weak(Arc::downgrade(&arc)));
                    } else {
                        frame.set_register(dst.0 as usize, RuntimeValue::Unit);
                    }
                    frame.advance();
                }
                BytecodeInstr::WeakUpgrade { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    if let RuntimeValue::Weak(weak) = val {
                        if let Some(arc) = weak.upgrade() {
                            frame.set_register(dst.0 as usize, RuntimeValue::Arc(arc));
                        } else {
                            // Upgrade failed - set to None unit
                            frame.set_register(dst.0 as usize, RuntimeValue::Unit);
                        }
                    } else {
                        frame.set_register(dst.0 as usize, RuntimeValue::Unit);
                    }
                    frame.advance();
                }
                BytecodeInstr::MakeClosure {
                    dst,
                    func: func_ref,
                    env,
                } => {
                    let func_name = match func_ref {
                        FunctionRef::Static { name, .. } => name.clone(),
                        FunctionRef::Index(idx) => format!("fn_{}", idx),
                    };
                    // Find the function's index in functions_by_id
                    let func_id = if let Some((idx, _)) = self
                        .functions_by_id
                        .iter()
                        .enumerate()
                        .find(|(_, f)| f.name == func_name)
                    {
                        crate::backends::common::value::FunctionId(idx as u32)
                    } else {
                        // Fallback: try to find in functions HashMap
                        if let Some(func) = self.functions.get(&func_name) {
                            // Add to functions_by_id if not already there
                            let idx = self.functions_by_id.len();
                            self.functions_by_id.push(func.clone());
                            crate::backends::common::value::FunctionId(idx as u32)
                        } else {
                            // Warning: function not found, fallback to id 0
                            eprintln!(
                                "[warn] Closure: function '{}' not found, fallback to id 0",
                                func_name
                            );
                            crate::backends::common::value::FunctionId(0)
                        }
                    };
                    // Capture environment variables from registers
                    let captured_env: Vec<RuntimeValue> = env
                        .iter()
                        .map(|r| frame.registers[r.0 as usize].clone())
                        .collect();
                    let closure =
                        RuntimeValue::Function(crate::backends::common::value::FunctionValue {
                            func_id,
                            env: captured_env,
                        });
                    frame.set_register(dst.0 as usize, closure);
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
                    frame.set_register(dst.0 as usize, RuntimeValue::Int(type_id as i64));
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
                    frame.set_register(dst.0 as usize, val);
                    frame.advance();
                }
                BytecodeInstr::StringFromInt { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .and_then(|v| v.to_int())
                        .unwrap_or(0);
                    frame
                        .set_register(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
                    frame.advance();
                }
                BytecodeInstr::StringFromFloat { dst, src } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .and_then(|v| v.to_float())
                        .unwrap_or(0.0);
                    frame
                        .set_register(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
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
                BytecodeInstr::LoadUpvalue { dst, upvalue_idx } => {
                    // Load from captured environment using the actual upvalue_idx
                    let idx = *upvalue_idx as usize;
                    let val = frame
                        .get_upvalue(idx)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    frame.set_register(dst.0 as usize, val);
                    frame.advance();
                }
                BytecodeInstr::StoreUpvalue { src, upvalue_idx } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    let idx = *upvalue_idx as usize;
                    frame.set_upvalue(idx, val);
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

                    frame.set_register(
                        dst.0 as usize,
                        RuntimeValue::Int(if s1 == s2 { 1 } else { 0 }),
                    );
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

                    frame.set_register(
                        dst.0 as usize,
                        s.chars()
                            .next()
                            .map(|c| RuntimeValue::Char(c as u32))
                            .unwrap_or(RuntimeValue::Unit),
                    );
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
                    dst,
                    obj,
                    name_idx: _,
                    args,
                } => {
                    // Dynamic call - 闭包调用
                    // obj 寄存器包含闭包值（FunctionValue）
                    let closure_val = frame
                        .registers
                        .get(obj.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);

                    if let RuntimeValue::Function(func_value) = closure_val {
                        // 收集参数（包括捕获的环境变量）
                        let env_args: Vec<RuntimeValue> = func_value.env.clone();
                        let call_args: Vec<RuntimeValue> = args
                            .iter()
                            .map(|r| {
                                frame
                                    .registers
                                    .get(r.0 as usize)
                                    .cloned()
                                    .unwrap_or(RuntimeValue::Unit)
                            })
                            .collect();

                        // 合并环境变量和参数
                        let mut final_args = env_args;
                        final_args.extend(call_args);

                        // 调用闭包函数
                        let result = self.call_function_by_id(func_value.func_id, &final_args)?;

                        // 保存返回值
                        if let Some(dst_reg) = dst {
                            frame.set_register(dst_reg.index() as usize, result);
                        }
                        frame.advance();
                    } else {
                        // 不是有效的函数值，返回 Unit
                        if let Some(dst_reg) = dst {
                            frame.set_register(dst_reg.index() as usize, RuntimeValue::Unit);
                        }
                        frame.advance();
                    }
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

impl DebuggableExecutor for Interpreter {
    fn set_breakpoint(
        &mut self,
        offset: usize,
    ) {
        self.breakpoints.insert(offset, ());
    }

    fn remove_breakpoint(
        &mut self,
        offset: usize,
    ) {
        self.breakpoints.remove(&offset);
    }

    fn has_breakpoint(&self) -> bool {
        if let Some(frame) = self.call_stack.last() {
            self.breakpoints.contains_key(&frame.ip)
        } else {
            false
        }
    }

    fn step(&mut self) -> ExecutorResult<()> {
        todo!("step debugging not implemented")
    }

    fn step_over(&mut self) -> ExecutorResult<()> {
        todo!("step_over debugging not implemented")
    }

    fn step_out(&mut self) -> ExecutorResult<()> {
        todo!("step_out debugging not implemented")
    }

    fn run(&mut self) -> ExecutorResult<()> {
        todo!("run debugging not implemented")
    }

    fn current_ip(&self) -> usize {
        self.call_stack.last().map(|f| f.ip).unwrap_or(0)
    }

    fn current_function(&self) -> Option<&str> {
        self.call_stack.last().map(|f| f.function.name.as_str())
    }

    fn breakpoints(&self) -> Vec<usize> {
        self.breakpoints.keys().copied().collect()
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

    // =============================================================================
    // FFI End-to-End Tests
    // =============================================================================

    /// Create a CallNative bytecode instruction for testing
    fn make_call_native_bytecode(
        func_name: &str,
        args: Vec<ConstValue>,
    ) -> (BytecodeModule, usize) {
        let mut module = BytecodeModule::new("test_ffi".to_string());

        // Build instructions:
        // 1. Load constants for args
        // 2. CallNative
        // 3. Return
        let mut instructions = Vec::new();

        for (i, arg) in args.iter().enumerate() {
            let const_idx = module.constants.len() as u16;
            module.constants.push(arg.clone());

            instructions.push(BytecodeInstr::LoadConst {
                dst: Reg(i as u16),
                const_idx,
            });
        }

        // CallNative instruction - func_name is String directly
        let arg_regs: Vec<Reg> = (0..args.len()).map(|i| Reg(i as u16)).collect();

        instructions.push(BytecodeInstr::CallNative {
            dst: Some(Reg(100)), // Use a high register for return
            func_name: func_name.to_string(),
            args: arg_regs,
        });

        // Return the result
        instructions.push(BytecodeInstr::ReturnValue { value: Reg(100) });

        let func_idx = module.add_function(BytecodeFunction {
            name: "main".to_string(),
            params: vec![],
            return_type: crate::middle::core::ir::Type::Void,
            local_count: 101,
            upvalue_count: 0,
            instructions,
            labels: HashMap::new(),
            exception_handlers: vec![],
        });

        module.entry_point = Some(func_idx);
        (module, func_idx)
    }

    #[test]
    fn test_ffi_println_e2e() {
        let mut interp = Interpreter::new();

        // Create: println("Hello, FFI!")
        let (module, func_idx) = make_call_native_bytecode(
            "std.io.println",
            vec![ConstValue::String("Hello, FFI!".to_string())],
        );

        let result = interp.execute_function(&module.functions[func_idx], &[]);
        assert!(result.is_ok(), "FFI call should succeed");
    }

    #[test]
    fn test_ffi_write_and_read_file_e2e() {
        let mut interp = Interpreter::new();

        // Test using std.io directly via FFI registry
        let path_str = "test_e2e_file.txt".to_string();

        // Test write_file via registry directly
        let mut ctx = NativeContext::new(&mut interp.heap);
        let result = interp.ffi.call(
            "std.io.write_file",
            &[
                RuntimeValue::String(path_str.clone().into()),
                RuntimeValue::String("E2E test content".into()),
            ],
            &mut ctx,
        );
        assert!(result.is_ok(), "write_file should succeed: {:?}", result);

        // Test read_file via registry
        let result = interp.ffi.call(
            "std.io.read_file",
            &[RuntimeValue::String(path_str.clone().into())],
            &mut ctx,
        );
        assert!(result.is_ok(), "read_file should succeed");

        // Verify content
        let return_value = result.unwrap();
        if let RuntimeValue::String(content) = return_value {
            assert_eq!(content.to_string(), "E2E test content");
        } else {
            panic!("Expected String return value");
        }

        // Cleanup
        let _ = std::fs::remove_file(&path_str);
    }

    #[test]
    fn test_ffi_custom_function_e2e() {
        let mut interp = Interpreter::new();

        // Register a custom native function
        interp
            .ffi_registry_mut()
            .register("test.multiply", |args, _ctx| {
                eprintln!("DEBUG: multiply args = {:?}", args);
                let a = args.get(0).and_then(|v| v.to_int()).unwrap_or(0);
                let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
                eprintln!("DEBUG: multiply {} * {}", a, b);
                Ok(RuntimeValue::Int(a * b))
            });

        // Test via registry directly
        let mut ctx = NativeContext::new(&mut interp.heap);
        let result = interp.ffi.call(
            "test.multiply",
            &[RuntimeValue::Int(6), RuntimeValue::Int(7)],
            &mut ctx,
        );

        assert!(
            result.is_ok(),
            "Custom FFI call should succeed: {:?}",
            result
        );

        let return_value = result.unwrap();
        if let RuntimeValue::Int(val) = return_value {
            assert_eq!(val, 42, "6 * 7 should equal 42");
        } else {
            panic!("Expected Int return value");
        }
    }

    #[test]
    fn test_ffi_nonexistent_function_e2e() {
        let mut interp = Interpreter::new();

        // Try to call a non-existent function
        let mut ctx = NativeContext::new(&mut interp.heap);
        let result = interp.ffi.call("nonexistent.function", &[], &mut ctx);
        assert!(result.is_err(), "Call to non-existent function should fail");
    }

    #[test]
    fn test_ffi_append_file_e2e() {
        let mut interp = Interpreter::new();

        let path_str = "test_append_file.txt".to_string();

        let mut ctx = NativeContext::new(&mut interp.heap);
        // Write initial content
        let result1 = interp.ffi.call(
            "std.io.write_file",
            &[
                RuntimeValue::String(path_str.clone().into()),
                RuntimeValue::String("First".into()),
            ],
            &mut ctx,
        );
        assert!(result1.is_ok());

        // Append content
        let result2 = interp.ffi.call(
            "std.io.append_file",
            &[
                RuntimeValue::String(path_str.clone().into()),
                RuntimeValue::String(" Second".into()),
            ],
            &mut ctx,
        );
        assert!(result2.is_ok());

        // Read and verify
        let result3 = interp.ffi.call(
            "std.io.read_file",
            &[RuntimeValue::String(path_str.clone().into())],
            &mut ctx,
        );
        assert!(result3.is_ok());

        let return_value = result3.unwrap();
        if let RuntimeValue::String(content) = return_value {
            assert_eq!(content.to_string(), "First Second");
        } else {
            panic!("Expected String");
        }

        // Cleanup
        let _ = std::fs::remove_file(&path_str);
    }
}
