//! Executor trait implementation for YaoXiang bytecode interpreter
//!
//! This module contains the Executor trait implementation with the main bytecode execution loop.

use std::sync::Arc;
use crate::backends::{Executor, ExecutorResult, ExecutorError, ExecutionState};
use crate::backends::common::{RuntimeValue, Heap, HeapValue};
use crate::middle::bytecode::{
    BytecodeModule, BytecodeFunction, BytecodeInstr, FunctionRef, ConstValue,
};
use crate::backends::interpreter::Frame;
use crate::backends::interpreter::frames::MAX_LOCALS;
use crate::backends::runtime::RuntimeMode;
use crate::backends::runtime::engine::{LocalRuntime, ResourceKey, TaskMeta};
use crate::util::i18n::MSG;
use crate::tlog;
use super::executor::{Interpreter, InterpreterTask};

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
            let stack = self.capture_stack();
            return Err(ExecutorError::runtime(
                format!(
                    "Too many locals in function '{}': {}",
                    func.name, func.local_count
                ),
                stack,
            ));
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
                BytecodeInstr::Yield => {
                    // Reserved for cooperative scheduling; currently a no-op in the interpreter VM.
                    frame.advance();
                }
                BytecodeInstr::Return => {
                    // Structured-concurrency safety net: ensure all spawned tasks complete.
                    for task_id in frame.take_all_spawned_tasks() {
                        let mut v = self.make_async_pending(task_id);
                        self.force_value_in_place(&mut v)?;
                    }
                    self.pop_frame();
                    return Ok(RuntimeValue::Unit);
                }
                BytecodeInstr::ReturnValue { value } => {
                    let result = frame
                        .registers
                        .get(value.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    // Structured-concurrency safety net: ensure all spawned tasks complete.
                    for task_id in frame.take_all_spawned_tasks() {
                        let mut v = self.make_async_pending(task_id);
                        self.force_value_in_place(&mut v)?;
                    }
                    self.pop_frame();
                    return Ok(result);
                }
                BytecodeInstr::Spawn { dst, func, args } => {
                    // Spawn a dynamic call as a runtime task.
                    let dst = *dst;
                    let func = *func;
                    let args = args.clone();

                    let closure_val = self.force_register(&mut frame, func)?;
                    let RuntimeValue::Function(func_value) = closure_val else {
                        let stack = self.capture_stack();
                        return Err(ExecutorError::type_error(
                            "spawn expects a function value".to_string(),
                            stack,
                        ));
                    };

                    let mut call_args = Vec::with_capacity(args.len());
                    for r in &args {
                        call_args.push(self.force_register(&mut frame, *r)?);
                    }

                    let deps = self.deps_from_args(&call_args);
                    let task_id = self.schedule_task(
                        InterpreterTask::Dyn {
                            func: func_value.clone(),
                            args: call_args,
                        },
                        TaskMeta {
                            deps,
                            label: Some(Arc::<str>::from("spawn")),
                            ..TaskMeta::default()
                        },
                    )?;

                    frame.record_spawned_task(task_id);
                    frame.set_register(dst.0 as usize, self.make_async_pending(task_id));
                    frame.advance();
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
                    let cond = *cond;
                    let target = *target;
                    let c = self
                        .force_register(&mut frame, cond)?
                        .to_bool()
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
                    let cond = *cond;
                    let target = *target;
                    let c = self
                        .force_register(&mut frame, cond)?
                        .to_bool()
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
                BytecodeInstr::UnaryOp { dst, src, op } => {
                    let dst = *dst;
                    let src = *src;
                    let op = *op;
                    let val = self.force_register(&mut frame, src)?;
                    let result = match (op, val) {
                        (crate::middle::bytecode::UnaryOp::Neg, RuntimeValue::Int(n)) => {
                            RuntimeValue::Int(-n)
                        }
                        (crate::middle::bytecode::UnaryOp::Neg, RuntimeValue::Float(f)) => {
                            RuntimeValue::Float(-f)
                        }
                        (crate::middle::bytecode::UnaryOp::Not, RuntimeValue::Int(n)) => {
                            RuntimeValue::Int(!n)
                        }
                        (crate::middle::bytecode::UnaryOp::Not, RuntimeValue::Bool(b)) => {
                            RuntimeValue::Bool(!b)
                        }
                        _ => RuntimeValue::Unit,
                    };
                    frame.set_register(dst.0 as usize, result);
                    frame.advance();
                }
                BytecodeInstr::CallStatic {
                    dst,
                    func: func_ref,
                    args: arg_regs,
                } => {
                    let dst = *dst;
                    let arg_regs = arg_regs.clone();

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

                    let runtime = self.runtime_config.runtime;

                    if matches!(runtime, RuntimeMode::Embedded) {
                        let result = self.call_static_by_name(&func_name, &call_args)?;
                        if let Some(dst_reg) = dst {
                            frame.set_register(dst_reg.index() as usize, result);
                        }
                        frame.advance();
                        continue;
                    }

                    let is_ffi = self.ffi.has(&func_name);
                    let deps = self.deps_from_args(&call_args);
                    let resources = if is_ffi {
                        vec![ResourceKey::from("ffi")]
                    } else {
                        Vec::new()
                    };

                    let task_id = self.schedule_task(
                        if is_ffi {
                            InterpreterTask::Native {
                                func_name: func_name.clone(),
                                args: call_args.clone(),
                            }
                        } else {
                            InterpreterTask::Static {
                                func_name: func_name.clone(),
                                args: call_args.clone(),
                            }
                        },
                        TaskMeta {
                            deps,
                            resources,
                            label: Some(Arc::<str>::from(func_name.as_str())),
                        },
                    )?;

                    self.drive_dag_until(Some(task_id))?;
                    let mut v = self.make_async_pending(task_id);
                    self.force_value_in_place(&mut v)?;
                    if let Some(dst_reg) = dst {
                        frame.set_register(dst_reg.index() as usize, v);
                    }

                    frame.advance();
                }
                BytecodeInstr::CallNative {
                    dst,
                    func_name,
                    args: arg_regs,
                } => {
                    let dst = *dst;
                    let func_name = func_name.clone();
                    let arg_regs = arg_regs.clone();

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

                    let runtime = self.runtime_config.runtime;

                    if matches!(runtime, RuntimeMode::Embedded) {
                        let result = self.call_native_by_name(&func_name, &call_args)?;
                        if let Some(dst_reg) = dst {
                            frame.set_register(dst_reg.index() as usize, result);
                        }
                        frame.advance();
                        continue;
                    }

                    let deps = self.deps_from_args(&call_args);
                    let task_id = self.schedule_task(
                        InterpreterTask::Native {
                            func_name: func_name.clone(),
                            args: call_args.clone(),
                        },
                        TaskMeta {
                            deps,
                            resources: vec![ResourceKey::from("ffi")],
                            label: Some(Arc::<str>::from(func_name.as_str())),
                        },
                    )?;

                    // Native calls are always forced eagerly to preserve side effects.
                    self.drive_dag_until(Some(task_id))?;
                    let mut v = self.make_async_pending(task_id);
                    self.force_value_in_place(&mut v)?;
                    if let Some(dst_reg) = dst {
                        frame.set_register(dst_reg.index() as usize, v);
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
                    let dst = *dst;
                    let array = *array;
                    let index = *index;
                    let arr = self.force_register(&mut frame, array)?;
                    let idx_value = self.force_register(&mut frame, index)?;

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
                    let array = *array;
                    let index = *index;
                    let value = *value;
                    let arr = self.force_register(&mut frame, array)?;
                    let idx_value = self.force_register(&mut frame, index)?;
                    let val = self.force_register(&mut frame, value)?;

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
                    let dst = *dst;
                    let src = *src;
                    let field_idx = *field_idx;
                    let obj = self.force_register(&mut frame, src)?;
                    if let RuntimeValue::Struct { fields, .. } = obj {
                        if let Some(HeapValue::Tuple(items)) = self.heap.get(fields) {
                            if (field_idx as usize) < items.len() {
                                frame.set_register(
                                    dst.0 as usize,
                                    items[field_idx as usize].clone(),
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
                    let src = *src;
                    let field_idx = *field_idx;
                    let value = *value;
                    let obj = self.force_register(&mut frame, src)?;
                    let val = self.force_register(&mut frame, value)?;
                    if let RuntimeValue::Struct { fields, .. } = obj {
                        if let Some(HeapValue::Tuple(items)) = self.heap.get_mut(fields) {
                            if (field_idx as usize) < items.len() {
                                items[field_idx as usize] = val;
                            }
                        }
                    }
                    frame.advance();
                }
                BytecodeInstr::StringConcat { dst, str1, str2 } => {
                    let dst = *dst;
                    let str1 = *str1;
                    let str2 = *str2;
                    let s1: String = match self.force_register(&mut frame, str1)? {
                        RuntimeValue::String(s) => s.as_ref().to_string(),
                        _ => String::new(),
                    };
                    let s2: String = match self.force_register(&mut frame, str2)? {
                        RuntimeValue::String(s) => s.as_ref().to_string(),
                        _ => String::new(),
                    };

                    frame.set_register(
                        dst.0 as usize,
                        RuntimeValue::String(format!("{}{}", s1, s2).into()),
                    );
                    frame.advance();
                }
                BytecodeInstr::StringLength { dst, src } => {
                    let dst = *dst;
                    let src = *src;
                    let s: String = match self.force_register(&mut frame, src)? {
                        RuntimeValue::String(s) => s.as_ref().to_string(),
                        _ => String::new(),
                    };

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
                    type_name,
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

                    // 构建 vtable
                    let vtable = self.build_vtable(type_name);

                    // 创建结构体值
                    let struct_val = RuntimeValue::Struct {
                        type_id: crate::backends::common::value::TypeId(0),
                        fields: handle,
                        vtable,
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
                // Borrow token: ZST, runtime equivalent to Mov
                BytecodeInstr::Borrow { dst, src, .. } => {
                    let val = frame
                        .registers
                        .get(src.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    frame.set_register(dst.0 as usize, val);
                    frame.advance();
                }
                // Release borrow token: ZST, runtime equivalent to Nop
                BytecodeInstr::Release { .. } => {
                    frame.advance();
                }
                BytecodeInstr::MakeClosure {
                    dst,
                    func: func_ref,
                    env,
                } => {
                    let func_id = match func_ref {
                        FunctionRef::Static { name, .. } => {
                            // Find by name in functions_by_id
                            if let Some((idx, _)) = self
                                .functions_by_id
                                .iter()
                                .enumerate()
                                .find(|(_, f)| f.name == *name)
                            {
                                crate::backends::common::value::FunctionId(idx as u32)
                            } else if let Some(func) = self.functions.get(name.as_str()) {
                                let idx = self.functions_by_id.len();
                                self.functions_by_id.push(func.clone());
                                crate::backends::common::value::FunctionId(idx as u32)
                            } else {
                                eprintln!(
                                    "[warn] Closure: function '{}' not found, fallback to id 0",
                                    name
                                );
                                crate::backends::common::value::FunctionId(0)
                            }
                        }
                        FunctionRef::Index(idx) => {
                            // Direct index into functions_by_id
                            if (*idx as usize) < self.functions_by_id.len() {
                                crate::backends::common::value::FunctionId(*idx)
                            } else {
                                eprintln!(
                                    "[warn] Closure: function index {} out of range ({}), fallback to id 0",
                                    idx,
                                    self.functions_by_id.len()
                                );
                                crate::backends::common::value::FunctionId(0)
                            }
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
                    let dst = *dst;
                    let src = *src;
                    let val = self.force_register(&mut frame, src)?;
                    let type_name: &str = match &val {
                        RuntimeValue::Unit => "Void",
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
                        RuntimeValue::Weak(_) => "Weak",
                        RuntimeValue::Async(_) => "Async",
                        RuntimeValue::Ptr { .. } => "Ptr",
                        _ => "Unknown",
                    };
                    let result = RuntimeValue::String(Arc::from(type_name));
                    frame.set_register(dst.0 as usize, result);
                    frame.advance();
                }
                BytecodeInstr::Cast {
                    dst,
                    src,
                    target_type_id,
                } => {
                    let dst = *dst;
                    let src = *src;
                    let type_id = *target_type_id;
                    let val = self.force_register(&mut frame, src)?;
                    // 基本类型转换：Int↔Float, 其他透传
                    let result = match (val, type_id) {
                        (RuntimeValue::Int(n), 1) => RuntimeValue::Float(n as f64), // Int → Float
                        (RuntimeValue::Float(f), 0) => RuntimeValue::Int(f as i64), // Float → Int
                        (RuntimeValue::Int(n), 2) => RuntimeValue::Bool(n != 0),    // Int → Bool
                        (RuntimeValue::Bool(b), 0) => RuntimeValue::Int(if b { 1 } else { 0 }), // Bool → Int
                        (v, _) => v, // 未知类型，透传
                    };
                    frame.set_register(dst.0 as usize, result);
                    frame.advance();
                }
                BytecodeInstr::StringFromInt { dst, src } => {
                    let dst = *dst;
                    let src = *src;
                    let val = self.force_register(&mut frame, src)?.to_int().unwrap_or(0);
                    frame
                        .set_register(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
                    frame.advance();
                }
                BytecodeInstr::StringFromFloat { dst, src } => {
                    let dst = *dst;
                    let src = *src;
                    let val = self
                        .force_register(&mut frame, src)?
                        .to_float()
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
                    let stack = self.capture_stack();
                    return Err(ExecutorError::runtime(
                        "User thrown error".to_string(),
                        stack,
                    ));
                }
                BytecodeInstr::BoundsCheck { array, index } => {
                    let array = *array;
                    let index = *index;
                    let arr = self.force_register(&mut frame, array)?;
                    let idx = self
                        .force_register(&mut frame, index)?
                        .to_int()
                        .unwrap_or(-1);
                    let len = match &arr {
                        RuntimeValue::List(h) | RuntimeValue::Tuple(h) | RuntimeValue::Array(h) => {
                            match self.heap.get(*h) {
                                Some(HeapValue::List(list)) => list.len() as i64,
                                Some(HeapValue::Tuple(t)) => t.len() as i64,
                                _ => -1,
                            }
                        }
                        _ => -1,
                    };
                    if idx < 0 || idx >= len {
                        let stack = self.capture_stack();
                        return Err(ExecutorError::runtime(
                            format!("Index {} out of bounds for length {}", idx, len),
                            stack,
                        ));
                    }
                    frame.advance();
                }
                BytecodeInstr::TypeCheck { value, type_id } => {
                    let value = *value;
                    let type_id = *type_id;
                    let val = self.force_register(&mut frame, value)?;
                    // 基本类型 ID 映射：0=Int, 1=Float, 2=Bool, 3=String, 4=Char
                    let actual_id: u16 = match val {
                        RuntimeValue::Int(_) => 0,
                        RuntimeValue::Float(_) => 1,
                        RuntimeValue::Bool(_) => 2,
                        RuntimeValue::String(_) => 3,
                        RuntimeValue::Char(_) => 4,
                        RuntimeValue::Unit => 5,
                        _ => u16::MAX,
                    };
                    if actual_id != type_id && type_id != u16::MAX {
                        let stack = self.capture_stack();
                        return Err(ExecutorError::runtime(
                            format!(
                                "Type mismatch: expected type_id {}, got {}",
                                type_id, actual_id
                            ),
                            stack,
                        ));
                    }
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
                BytecodeInstr::Switch { value, targets } => {
                    let value = *value;
                    let targets = targets.clone();
                    let val = self.force_register(&mut frame, value)?;
                    let mut jumped = false;
                    for (case_val, target) in &targets {
                        if let Some(case_label) = case_val {
                            // case 值编码为 Label，解码为 i32 常量
                            let case_offset = i32::from_le_bytes([
                                case_label.0 as u8,
                                (case_label.0 >> 8) as u8,
                                (case_label.0 >> 16) as u8,
                                (case_label.0 >> 24) as u8,
                            ]);
                            let matches = match &val {
                                RuntimeValue::Int(n) => *n == case_offset as i64,
                                RuntimeValue::Bool(b) => *b == (case_offset != 0),
                                _ => false,
                            };
                            if matches {
                                let target_offset = i32::from_le_bytes([
                                    target.0 as u8,
                                    (target.0 >> 8) as u8,
                                    (target.0 >> 16) as u8,
                                    (target.0 >> 24) as u8,
                                ]);
                                let target_ip = ((frame.ip as i32) + target_offset) as usize;
                                frame.ip = target_ip;
                                jumped = true;
                                break;
                            }
                        }
                    }
                    // 没有匹配的 case，跳转到 default（targets 最后一个 None 入口）
                    if !jumped {
                        if let Some((None, default_target)) = targets.last() {
                            let offset = i32::from_le_bytes([
                                default_target.0 as u8,
                                (default_target.0 >> 8) as u8,
                                (default_target.0 >> 16) as u8,
                                (default_target.0 >> 24) as u8,
                            ]);
                            let target_ip = ((frame.ip as i32) + offset) as usize;
                            frame.ip = target_ip;
                        } else {
                            frame.advance();
                        }
                    }
                    continue;
                }
                BytecodeInstr::StackAlloc { dst: _, size: _ } => {
                    frame.advance();
                }
                BytecodeInstr::StringEqual { dst, str1, str2 } => {
                    let dst = *dst;
                    let str1 = *str1;
                    let str2 = *str2;
                    let s1: String = match self.force_register(&mut frame, str1)? {
                        RuntimeValue::String(s) => s.as_ref().to_string(),
                        _ => String::new(),
                    };
                    let s2: String = match self.force_register(&mut frame, str2)? {
                        RuntimeValue::String(s) => s.as_ref().to_string(),
                        _ => String::new(),
                    };

                    frame.set_register(
                        dst.0 as usize,
                        RuntimeValue::Int(if s1 == s2 { 1 } else { 0 }),
                    );
                    frame.advance();
                }
                BytecodeInstr::StringGetChar { dst, src, index } => {
                    let dst = *dst;
                    let src = *src;
                    let idx = index.0 as usize;
                    let s: String = match self.force_register(&mut frame, src)? {
                        RuntimeValue::String(s) => s.as_ref().to_string(),
                        _ => String::new(),
                    };

                    let result = s
                        .chars()
                        .nth(idx)
                        .map(|c| RuntimeValue::Char(c as u32))
                        .unwrap_or(RuntimeValue::Unit);
                    frame.set_register(dst.0 as usize, result);
                    frame.advance();
                }
                BytecodeInstr::CallVirt {
                    dst,
                    obj,
                    method_idx,
                    args,
                } => {
                    let dst = *dst;
                    let obj = *obj;
                    let method_idx = *method_idx;
                    let args = args.clone();

                    // Virtual call - 通过 vtable 查找方法并调用
                    let obj_val = self.force_register(&mut frame, obj)?;

                    // 从常量池获取方法名
                    let method_name = self
                        .constants
                        .get(method_idx as usize)
                        .and_then(|c| {
                            if let ConstValue::String(s) = c {
                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();

                    // 从对象的 vtable 中查找方法
                    if let Some(func_value) = obj_val.get_method(&method_name).cloned() {
                        // 收集参数
                        let mut call_args = Vec::with_capacity(args.len());
                        for r in args {
                            call_args.push(self.force_register(&mut frame, r)?);
                        }

                        // 调用方法
                        let result = self.call_function_by_id(func_value.func_id, &call_args)?;

                        // 保存返回值
                        if let Some(dst_reg) = dst {
                            frame.set_register(dst_reg.index() as usize, result);
                        }
                    } else {
                        // 方法未在 vtable 中找到，返回 Unit
                        if let Some(dst_reg) = dst {
                            frame.set_register(dst_reg.index() as usize, RuntimeValue::Unit);
                        }
                    }
                    frame.advance();
                }
                BytecodeInstr::CallDyn {
                    dst,
                    obj,
                    name_idx: _,
                    args,
                } => {
                    let dst = *dst;
                    let obj = *obj;
                    let args = args.clone();

                    // Dynamic call - 闭包调用
                    // obj 寄存器包含闭包值（FunctionValue）
                    let closure_val = self.force_register(&mut frame, obj)?;

                    if let RuntimeValue::Function(func_value) = closure_val {
                        // 收集参数（包括捕获的环境变量）
                        let env_args: Vec<RuntimeValue> = func_value.env.clone();
                        let mut call_args = Vec::with_capacity(args.len());
                        for r in args {
                            call_args.push(self.force_register(&mut frame, r)?);
                        }

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
        self.rt_dag = LocalRuntime::new();
        self.rt_tasks.clear();
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
    //! Borrow/Release 字节码指令执行测试
    //!
    //! 参考规范：RFC-009 v9 §4.3 Borrow/Release 运行时语义。
    //! 验证借用令牌（ZST）在解释器中的拷贝、释放及边界行为。

    use super::*;
    use crate::backends::Executor;
    use crate::middle::bytecode::{BytecodeFunction, BytecodeInstr, Reg, ConstValue};
    use std::collections::HashMap;

    fn make_function(instrs: Vec<BytecodeInstr>) -> BytecodeFunction {
        BytecodeFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: crate::middle::core::ir::Type::Void,
            local_count: 4,
            upvalue_count: 0,
            instructions: instrs,
            labels: HashMap::new(),
            exception_handlers: vec![],
            debug_map: HashMap::new(),
        }
    }

    /// 辅助函数：创建预装一个常量的解释器
    fn make_interp_with_const(val: ConstValue) -> Interpreter {
        let mut interp = Interpreter::new();
        interp.constants.push(val);
        interp
    }

    /// Borrow copies value from src register to dst register (immutable)
    #[test]
    fn test_borrow_copies_value_immutable() {
        let func = make_function(vec![
            // r0 = Int(42)
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            // r1 = borrow r0 (immutable)
            BytecodeInstr::Borrow {
                dst: Reg(1),
                src: Reg(0),
                mutable: false,
            },
            // return r1
            BytecodeInstr::ReturnValue { value: Reg(1) },
        ]);

        let mut interp = make_interp_with_const(ConstValue::Int(42));

        let result = interp.execute_function(&func, &[]).unwrap();
        assert_eq!(
            result,
            RuntimeValue::Int(42),
            "不可变借用应拷贝源寄存器的值"
        );
    }

    /// Borrow copies value from src register to dst register (mutable)
    #[test]
    fn test_borrow_copies_value_mutable() {
        let func = make_function(vec![
            // r0 = Int(99)
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            // r1 = borrow mut r0
            BytecodeInstr::Borrow {
                dst: Reg(1),
                src: Reg(0),
                mutable: true,
            },
            // return r1
            BytecodeInstr::ReturnValue { value: Reg(1) },
        ]);

        let mut interp = make_interp_with_const(ConstValue::Int(99));

        let result = interp.execute_function(&func, &[]).unwrap();
        assert_eq!(result, RuntimeValue::Int(99), "可变借用应拷贝源寄存器的值");
    }

    /// Borrow with mutable:false and mutable:true produce the same runtime result (ZST)
    #[test]
    fn test_borrow_mutable_flag_irrelevant_at_runtime() {
        // Both immutable and mutable borrow copy the value identically.
        for mutable in [false, true] {
            let func = make_function(vec![
                BytecodeInstr::LoadConst {
                    dst: Reg(0),
                    const_idx: 0,
                },
                BytecodeInstr::Borrow {
                    dst: Reg(1),
                    src: Reg(0),
                    mutable,
                },
                BytecodeInstr::ReturnValue { value: Reg(1) },
            ]);

            let mut interp = make_interp_with_const(ConstValue::String("hello".into()));

            let result = interp.execute_function(&func, &[]).unwrap();
            assert_eq!(
                result,
                RuntimeValue::String("hello".into()),
                "mutable={mutable} should produce the same value",
            );
        }
    }

    /// Borrow with empty src register yields Unit
    #[test]
    fn test_borrow_from_unset_register() {
        let func = make_function(vec![
            // r1 = borrow r0 (r0 is unset -> Unit)
            BytecodeInstr::Borrow {
                dst: Reg(1),
                src: Reg(0),
                mutable: false,
            },
            // return r1
            BytecodeInstr::ReturnValue { value: Reg(1) },
        ]);

        let mut interp = Interpreter::new();
        let result = interp.execute_function(&func, &[]).unwrap();
        assert_eq!(
            result,
            RuntimeValue::Unit,
            "从未设置的寄存器借用应得到 Unit"
        ); // just advances IP; does not corrupt registers
    }

    #[test]
    fn test_release_is_noop() {
        let func = make_function(vec![
            // r0 = Int(7)
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            // release r0 (should be a no-op)
            BytecodeInstr::Release { src: Reg(0) },
            // r1 = r0 (still valid)
            BytecodeInstr::Mov {
                dst: Reg(1),
                src: Reg(0),
            },
            // return r1
            BytecodeInstr::ReturnValue { value: Reg(1) },
        ]);

        let mut interp = make_interp_with_const(ConstValue::Int(7));

        let result = interp.execute_function(&func, &[]).unwrap();
        assert_eq!(result, RuntimeValue::Int(7), "Release 不应修改寄存器值");
    }

    /// Release on unset register is also a no-op (no panic)
    #[test]
    fn test_release_unset_register() {
        let func = make_function(vec![
            // release r5 (unset) — must not panic
            BytecodeInstr::Release { src: Reg(5) },
            BytecodeInstr::Return,
        ]);

        let mut interp = Interpreter::new();
        let result = interp.execute_function(&func, &[]).unwrap();
        assert_eq!(
            result,
            RuntimeValue::Unit,
            "对未设置寄存器执行 Release 不应 panic"
        );
    }

    /// Borrow followed by Release preserves the borrowed value
    #[test]
    fn test_borrow_then_release_preserves_value() {
        let func = make_function(vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            // r1 = borrow r0
            BytecodeInstr::Borrow {
                dst: Reg(1),
                src: Reg(0),
                mutable: true,
            },
            // release r1 (no-op)
            BytecodeInstr::Release { src: Reg(1) },
            // return r1 — value still intact
            BytecodeInstr::ReturnValue { value: Reg(1) },
        ]);

        let mut interp = make_interp_with_const(ConstValue::Bool(true));

        let result = interp.execute_function(&func, &[]).unwrap();
        assert_eq!(
            result,
            RuntimeValue::Bool(true),
            "Borrow 后 Release 应保留借用的值"
        );
    }
}
