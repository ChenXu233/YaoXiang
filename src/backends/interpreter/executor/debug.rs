//! Debugger implementation for YaoXiang bytecode interpreter
//!
//! This module contains the DebuggableExecutor trait implementation and the
//! core stepping engine (step_one / execute_instr / run_until_stop).

use crate::backends::{DebuggableExecutor, ExecutorError, ExecutorResult};
use crate::backends::common::RuntimeValue;
use crate::middle::bytecode::{BytecodeInstr, FunctionRef, ConstValue, Label, Reg};
use super::executor::Interpreter;
use crate::backends::interpreter::Frame;

/// Outcome of a single instruction execution.
pub(super) enum StepOutcome {
    /// Instruction executed normally; continue to next.
    Continue,
    /// Function returned (frame already popped).
    Returned,
}

/// Reason the debugger stopped execution.
pub(super) enum StopReason {
    Breakpoint,
    Returned,
    Completed,
}

impl Interpreter {
    /// Decode a Label into a signed offset for relative jumps.
    fn decode_label_offset(label: Label) -> i32 {
        i32::from_le_bytes([
            label.0 as u8,
            (label.0 >> 8) as u8,
            (label.0 >> 16) as u8,
            (label.0 >> 24) as u8,
        ])
    }

    /// Execute a single instruction. The core of the stepping engine.
    ///
    /// Pops the top frame, executes one instruction, and pushes it back
    /// (unless the instruction was a Return).
    pub(super) fn step_one(&mut self) -> ExecutorResult<StepOutcome> {
        if self.call_stack.is_empty() {
            return Ok(StepOutcome::Returned);
        }

        // Cache stack-trace info before popping
        if let Some(frame) = self.call_stack.last() {
            self.current_frame_info = Some((frame.function.name.clone(), frame.ip));
        }

        // Pop frame — self is fully available
        let mut frame = self.pop_frame().unwrap();

        if frame.ip >= frame.function.instructions.len() {
            self.current_frame_info = None;
            self.push_frame(frame)?;
            return Ok(StepOutcome::Returned);
        }

        let depth_before = self.call_stack.len();
        let instr = frame.function.instructions[frame.ip].clone();
        let outcome = self.execute_instr(&mut frame, &instr)?;

        // Detect if a function call was executed (depth increased then restored)
        self.called_func = self.call_stack.len() > depth_before;

        // Don't push back on Return — frame is already consumed
        if !matches!(outcome, StepOutcome::Returned) {
            self.push_frame(frame)?;
        }

        self.current_frame_info = None;
        Ok(outcome)
    }

    /// Execute until a stop condition (breakpoint, return, or completion).
    pub(super) fn run_until_stop(&mut self) -> ExecutorResult<StopReason> {
        loop {
            if self.has_breakpoint() {
                return Ok(StopReason::Breakpoint);
            }
            match self.step_one()? {
                StepOutcome::Continue => {}
                StepOutcome::Returned => {
                    if self.call_stack.is_empty() {
                        return Ok(StopReason::Completed);
                    }
                    return Ok(StopReason::Returned);
                }
            }
        }
    }

    /// Execute a single instruction on the given frame.
    ///
    /// This is the instruction dispatcher — all instruction logic lives here.
    /// `frame` is a local variable (not on `self.call_stack`), so `self` is
    /// fully available for helper method calls.
    fn execute_instr(
        &mut self,
        frame: &mut Frame,
        instr: &BytecodeInstr,
    ) -> ExecutorResult<StepOutcome> {
        match instr {
            // ── No-ops ──────────────────────────────────────────
            BytecodeInstr::Nop
            | BytecodeInstr::Yield
            | BytecodeInstr::Drop { .. }
            | BytecodeInstr::Release { .. }
            | BytecodeInstr::StackAlloc { .. }
            | BytecodeInstr::TryBegin { .. }
            | BytecodeInstr::TryEnd
            | BytecodeInstr::ArcDrop { .. }
            | BytecodeInstr::CloseUpvalue { .. } => {
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Return ──────────────────────────────────────────
            BytecodeInstr::Return => {
                for task_id in frame.take_all_spawned_tasks() {
                    let mut v = self.make_async_pending(task_id);
                    self.force_value_in_place(&mut v)?;
                }
                self.last_return_value = RuntimeValue::Unit;
                // Frame is NOT pushed back — caller handles this
                Ok(StepOutcome::Returned)
            }
            BytecodeInstr::ReturnValue { value } => {
                let result = frame
                    .registers
                    .get(value.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                for task_id in frame.take_all_spawned_tasks() {
                    let mut v = self.make_async_pending(task_id);
                    self.force_value_in_place(&mut v)?;
                }
                self.last_return_value = result;
                Ok(StepOutcome::Returned)
            }

            // ── Jumps ───────────────────────────────────────────
            BytecodeInstr::Jmp { target } => {
                let offset = Self::decode_label_offset(*target);
                frame.ip = ((frame.ip as i32) + offset) as usize;
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::JmpIf { cond, target } => {
                let c = self
                    .force_register(frame, *cond)?
                    .to_bool()
                    .unwrap_or(false);
                if c {
                    let offset = Self::decode_label_offset(*target);
                    frame.ip = ((frame.ip as i32) + offset) as usize;
                } else {
                    frame.advance();
                }
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::JmpIfNot { cond, target } => {
                let c = self
                    .force_register(frame, *cond)?
                    .to_bool()
                    .unwrap_or(false);
                if !c {
                    let offset = Self::decode_label_offset(*target);
                    frame.ip = ((frame.ip as i32) + offset) as usize;
                } else {
                    frame.advance();
                }
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::Switch { value, targets } => {
                let val = self.force_register(frame, *value)?;
                let mut jumped = false;
                for (case_val, target) in targets {
                    if let Some(case_label) = case_val {
                        let case_offset = Self::decode_label_offset(*case_label);
                        let matches = match &val {
                            RuntimeValue::Int(n) => *n == case_offset as i64,
                            RuntimeValue::Bool(b) => *b == (case_offset != 0),
                            RuntimeValue::Enum { variant_id, .. } => {
                                *variant_id == case_offset as u32
                            }
                            _ => false,
                        };
                        if matches {
                            let offset = Self::decode_label_offset(*target);
                            frame.ip = ((frame.ip as i32) + offset) as usize;
                            jumped = true;
                            break;
                        }
                    }
                }
                if !jumped {
                    if let Some((None, default_target)) = targets.last() {
                        let offset = Self::decode_label_offset(*default_target);
                        frame.ip = ((frame.ip as i32) + offset) as usize;
                    } else {
                        frame.advance();
                    }
                }
                Ok(StepOutcome::Continue)
            }

            // ── Register operations ─────────────────────────────
            BytecodeInstr::Mov { dst, src } => {
                let val = frame
                    .registers
                    .get(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                frame.set_register(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadConst { dst, const_idx } => {
                let val = self.load_constant(*const_idx);
                frame.set_register(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadLocal { dst, local_idx } => {
                let val = frame
                    .get_local(*local_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                frame.set_register(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreLocal { local_idx, src } => {
                let val = frame
                    .registers
                    .get(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                frame.set_local(*local_idx as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadArg { dst, arg_idx } => {
                // Args are stored in locals by Frame::with_args
                let val = frame
                    .get_local(*arg_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                frame.set_register(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadUpvalue { dst, upvalue_idx } => {
                let val = frame
                    .get_upvalue(*upvalue_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                frame.set_register(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreUpvalue { src, upvalue_idx } => {
                let val = frame
                    .registers
                    .get(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                frame.set_upvalue(*upvalue_idx as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Arithmetic / comparison ─────────────────────────
            BytecodeInstr::BinaryOp { dst, lhs, rhs, op } => {
                self.exec_binary_op(*dst, *lhs, *rhs, *op, frame)?;
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::Compare { dst, lhs, rhs, cmp } => {
                self.exec_compare(*dst, *lhs, *rhs, *cmp, frame)?;
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::UnaryOp { dst, src, op } => {
                let val = self.force_register(frame, *src)?;
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
                Ok(StepOutcome::Continue)
            }

            // ── Function calls ──────────────────────────────────
            BytecodeInstr::CallStatic {
                dst,
                func: func_ref,
                args: arg_regs,
            } => {
                let func_name = match func_ref {
                    FunctionRef::Static { name, .. } => name.clone(),
                    FunctionRef::Index(idx) => {
                        if let Some(ConstValue::String(s)) = self.constants.get(*idx as usize) {
                            s.clone()
                        } else {
                            format!("fn_{}", idx)
                        }
                    }
                };

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

                if matches!(runtime, crate::backends::runtime::RuntimeMode::Embedded) {
                    let result = self.call_static_by_name(&func_name, &call_args)?;
                    if let Some(dst_reg) = dst {
                        frame.set_register(dst_reg.index() as usize, result);
                    }
                    frame.advance();
                    return Ok(StepOutcome::Continue);
                }

                use crate::backends::runtime::engine::{ResourceKey, TaskMeta};
                use std::sync::Arc;

                let is_ffi = self.ffi.has(&func_name);
                let deps = self.deps_from_args(&call_args);
                let resources = if is_ffi {
                    vec![ResourceKey::from("ffi")]
                } else {
                    Vec::new()
                };

                let task_id = self.schedule_task(
                    if is_ffi {
                        super::executor::InterpreterTask::Native {
                            func_name: func_name.clone(),
                            args: call_args.clone(),
                        }
                    } else {
                        super::executor::InterpreterTask::Static {
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
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::CallNative {
                dst,
                func_name,
                mechanism,
                lib,
                symbol,
                args: arg_regs,
            } => {
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

                if matches!(runtime, crate::backends::runtime::RuntimeMode::Embedded) {
                    let result = self.call_native_with_ffi_meta(
                        func_name, mechanism, lib, symbol, &call_args
                    )?;
                    if let Some(dst_reg) = dst {
                        frame.set_register(dst_reg.index() as usize, result);
                    }
                    frame.advance();
                    return Ok(StepOutcome::Continue);
                }

                use crate::backends::runtime::engine::{ResourceKey, TaskMeta};
                use std::sync::Arc;

                let deps = self.deps_from_args(&call_args);
                let task_id = self.schedule_task(
                    super::executor::InterpreterTask::Native {
                        func_name: func_name.clone(),
                        args: call_args.clone(),
                    },
                    TaskMeta {
                        deps,
                        resources: vec![ResourceKey::from("ffi")],
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
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::CallVirt {
                dst,
                obj,
                method_idx,
                args,
            } => {
                let obj_val = self.force_register(frame, *obj)?;

                let method_name = self
                    .constants
                    .get(*method_idx as usize)
                    .and_then(|c| {
                        if let ConstValue::String(s) = c {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                if let Some(func_value) = obj_val.get_method(&method_name).cloned() {
                    let mut call_args = Vec::with_capacity(args.len());
                    for r in args {
                        call_args.push(self.force_register(frame, *r)?);
                    }
                    let result = self.call_function_by_id(func_value.func_id, &call_args)?;
                    if let Some(dst_reg) = dst {
                        frame.set_register(dst_reg.index() as usize, result);
                    }
                } else {
                    if let Some(dst_reg) = dst {
                        frame.set_register(dst_reg.index() as usize, RuntimeValue::Unit);
                    }
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::CallDyn {
                dst,
                obj,
                name_idx: _,
                args,
            } => {
                let closure_val = self.force_register(frame, *obj)?;

                if let RuntimeValue::Function(func_value) = closure_val {
                    let env_args: Vec<RuntimeValue> = func_value.env.clone();
                    let mut call_args = Vec::with_capacity(args.len());
                    for r in args {
                        call_args.push(self.force_register(frame, *r)?);
                    }
                    let mut final_args = env_args;
                    final_args.extend(call_args);
                    let result = self.call_function_by_id(func_value.func_id, &final_args)?;
                    if let Some(dst_reg) = dst {
                        frame.set_register(dst_reg.index() as usize, result);
                    }
                } else {
                    if let Some(dst_reg) = dst {
                        frame.set_register(dst_reg.index() as usize, RuntimeValue::Unit);
                    }
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Concurrency ─────────────────────────────────────
            BytecodeInstr::Spawn {
                dst: _,
                closures,
                task_deps,
                task_resources,
            } => {
                let closures = closures.clone();
                let task_deps = task_deps.clone();
                let task_resources = task_resources.clone();
                let runtime = self.runtime_config.runtime;

                if matches!(runtime, crate::backends::runtime::RuntimeMode::Embedded) {
                    for func_reg in closures.iter() {
                        let closure_val = self.force_register(frame, *func_reg)?;
                        let RuntimeValue::Function(func_value) = closure_val else {
                            let stack = self.capture_stack();
                            return Err(ExecutorError::type_error(
                                "spawn expects a function value".to_string(),
                                stack,
                            ));
                        };
                        let _result =
                            self.call_function_by_id(func_value.func_id, &func_value.env)?;
                        frame.set_register(func_reg.0 as usize, _result);
                    }
                } else {
                    use crate::backends::runtime::engine::{ResourceKey, TaskMeta};
                    use std::sync::Arc;

                    let mut task_ids: Vec<(Reg, crate::backends::common::value::TaskId)> =
                        Vec::new();

                    for (i, func_reg) in closures.iter().enumerate() {
                        let closure_val = self.force_register(frame, *func_reg)?;
                        let RuntimeValue::Function(func_value) = closure_val else {
                            let stack = self.capture_stack();
                            return Err(ExecutorError::type_error(
                                "spawn expects a function value".to_string(),
                                stack,
                            ));
                        };

                        let call_args: Vec<RuntimeValue> = func_value.env.clone();
                        let mut deps = self.deps_from_args(&call_args);

                        if let Some(task_dep_indices) = task_deps.get(i) {
                            for &dep_idx in task_dep_indices {
                                if let Some((_, dep_task_id)) = task_ids.get(dep_idx as usize) {
                                    deps.push(*dep_task_id);
                                }
                            }
                        }

                        let resources: Vec<ResourceKey> = task_resources
                            .get(i)
                            .map(|rs| rs.iter().map(|r| ResourceKey::new(r.as_str())).collect())
                            .unwrap_or_default();

                        let task_id = self.schedule_task(
                            super::executor::InterpreterTask::Dyn {
                                func: func_value.clone(),
                                args: call_args,
                            },
                            TaskMeta {
                                deps,
                                resources,
                                label: Some(Arc::<str>::from("spawn")),
                            },
                        )?;

                        frame.record_spawned_task(task_id);
                        task_ids.push((*func_reg, task_id));
                    }

                    for (func_reg, task_id) in &task_ids {
                        let mut v = self.make_async_pending(*task_id);
                        self.force_value_in_place(&mut v)?;
                        frame.set_register(func_reg.0 as usize, v);
                    }
                }

                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::SpawnFromList {
                dst: _,
                closures_list,
                task_deps,
                task_resources,
            } => {
                let closures_list = *closures_list;
                let task_deps = task_deps.clone();
                let task_resources = task_resources.clone();

                let list_val = self.force_register(frame, closures_list)?;
                let closures: Vec<RuntimeValue> = match list_val {
                    RuntimeValue::List(handle) => match self.heap.get(handle) {
                        Some(crate::backends::common::HeapValue::List(items)) => items.clone(),
                        _ => {
                            let stack = self.capture_stack();
                            return Err(ExecutorError::type_error(
                                "spawn_from_list expects a list value".to_string(),
                                stack,
                            ));
                        }
                    },
                    _ => {
                        let stack = self.capture_stack();
                        return Err(ExecutorError::type_error(
                            "spawn_from_list expects a list value".to_string(),
                            stack,
                        ));
                    }
                };

                let runtime = self.runtime_config.runtime;

                if matches!(runtime, crate::backends::runtime::RuntimeMode::Embedded) {
                    for closure_val in closures.iter() {
                        let RuntimeValue::Function(func_value) = closure_val else {
                            let stack = self.capture_stack();
                            return Err(ExecutorError::type_error(
                                "spawn_from_list expects function values in list".to_string(),
                                stack,
                            ));
                        };
                        let _result =
                            self.call_function_by_id(func_value.func_id, &func_value.env)?;
                    }
                } else {
                    use crate::backends::runtime::engine::{ResourceKey, TaskMeta};
                    use std::sync::Arc;

                    let mut spawned_tasks: Vec<crate::backends::common::value::TaskId> = Vec::new();

                    for (i, closure_val) in closures.iter().enumerate() {
                        let RuntimeValue::Function(func_value) = closure_val else {
                            let stack = self.capture_stack();
                            return Err(ExecutorError::type_error(
                                "spawn_from_list expects function values in list".to_string(),
                                stack,
                            ));
                        };

                        let call_args: Vec<RuntimeValue> = func_value.env.clone();
                        let mut deps = self.deps_from_args(&call_args);

                        if let Some(task_dep_indices) = task_deps.get(i) {
                            for &dep_idx in task_dep_indices {
                                if let Some(dep_task_id) = spawned_tasks.get(dep_idx as usize) {
                                    deps.push(*dep_task_id);
                                }
                            }
                        }

                        let resources: Vec<ResourceKey> = task_resources
                            .get(i)
                            .map(|rs| rs.iter().map(|r| ResourceKey::new(r.as_str())).collect())
                            .unwrap_or_default();

                        let task_id = self.schedule_task(
                            super::executor::InterpreterTask::Dyn {
                                func: func_value.clone(),
                                args: call_args,
                            },
                            TaskMeta {
                                deps,
                                resources,
                                label: Some(Arc::<str>::from("spawn_from_list")),
                            },
                        )?;

                        frame.record_spawned_task(task_id);
                        spawned_tasks.push(task_id);
                    }

                    for task_id in &spawned_tasks {
                        let mut v = self.make_async_pending(*task_id);
                        self.force_value_in_place(&mut v)?;
                    }
                }

                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Heap / collection operations ─────────────────────
            BytecodeInstr::HeapAlloc { dst, type_id: _ } => {
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Tuple(Vec::new()));
                frame.set_register(dst.0 as usize, RuntimeValue::Tuple(handle));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewListWithCap { dst, capacity } => {
                let handle = self.heap.allocate(crate::backends::common::HeapValue::List(
                    Vec::with_capacity(*capacity as usize),
                ));
                frame.set_register(dst.0 as usize, RuntimeValue::List(handle));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewDict { dst, keys, values } => {
                let mut map = std::collections::HashMap::new();
                for (key_reg, val_reg) in keys.iter().zip(values.iter()) {
                    let key = frame
                        .registers
                        .get(key_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    let val = frame
                        .registers
                        .get(val_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Unit);
                    map.insert(key, val);
                }
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Dict(map));
                frame.set_register(dst.0 as usize, RuntimeValue::Dict(handle));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadElement { dst, array, index } => {
                let arr = self.force_register(frame, *array)?;
                let idx_value = self.force_register(frame, *index)?;

                match arr {
                    RuntimeValue::List(handle) => {
                        let idx = idx_value.to_int().unwrap_or(0) as usize;
                        if let Some(crate::backends::common::HeapValue::List(items)) =
                            self.heap.get(handle)
                        {
                            if idx < items.len() {
                                frame.set_register(dst.0 as usize, items[idx].clone());
                            }
                        }
                    }
                    RuntimeValue::Tuple(handle) => {
                        let idx = idx_value.to_int().unwrap_or(0) as usize;
                        if let Some(crate::backends::common::HeapValue::Tuple(items)) =
                            self.heap.get(handle)
                        {
                            if idx < items.len() {
                                frame.set_register(dst.0 as usize, items[idx].clone());
                            }
                        }
                    }
                    RuntimeValue::Array(handle) => {
                        let idx = idx_value.to_int().unwrap_or(0) as usize;
                        if let Some(crate::backends::common::HeapValue::Array(items)) =
                            self.heap.get(handle)
                        {
                            if idx < items.len() {
                                frame.set_register(dst.0 as usize, items[idx].clone());
                            }
                        }
                    }
                    RuntimeValue::Dict(handle) => {
                        if let Some(crate::backends::common::HeapValue::Dict(map)) =
                            self.heap.get(handle)
                        {
                            if let Some(value) = map.get(&idx_value) {
                                frame.set_register(dst.0 as usize, value.clone());
                            }
                        }
                    }
                    _ => {}
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreElement {
                array,
                index,
                value,
            } => {
                let arr = self.force_register(frame, *array)?;
                let idx_value = self.force_register(frame, *index)?;
                let val = self.force_register(frame, *value)?;

                match arr {
                    RuntimeValue::List(handle) => {
                        let idx = idx_value.to_int().unwrap_or(0) as usize;
                        if let Some(crate::backends::common::HeapValue::List(items)) =
                            self.heap.get_mut(handle)
                        {
                            if idx < items.len() {
                                items[idx] = val;
                            } else if idx == items.len() {
                                items.push(val);
                            }
                        }
                    }
                    RuntimeValue::Array(handle) => {
                        let idx = idx_value.to_int().unwrap_or(0) as usize;
                        if let Some(crate::backends::common::HeapValue::Array(items)) =
                            self.heap.get_mut(handle)
                        {
                            if idx < items.len() {
                                items[idx] = val;
                            }
                        }
                    }
                    RuntimeValue::Dict(handle) => {
                        if let Some(crate::backends::common::HeapValue::Dict(map)) =
                            self.heap.get_mut(handle)
                        {
                            map.insert(idx_value, val);
                        }
                    }
                    _ => {}
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::GetField {
                dst,
                src,
                field_idx,
            } => {
                let obj = self.force_register(frame, *src)?;
                if let RuntimeValue::Struct { fields, .. } = obj {
                    if let Some(crate::backends::common::HeapValue::Tuple(items)) =
                        self.heap.get(fields)
                    {
                        if (*field_idx as usize) < items.len() {
                            frame.set_register(dst.0 as usize, items[*field_idx as usize].clone());
                        }
                    }
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::SetField {
                src,
                field_idx,
                value,
            } => {
                let obj = self.force_register(frame, *src)?;
                let val = self.force_register(frame, *value)?;
                if let RuntimeValue::Struct { fields, .. } = obj {
                    if let Some(crate::backends::common::HeapValue::Tuple(items)) =
                        self.heap.get_mut(fields)
                    {
                        if (*field_idx as usize) < items.len() {
                            items[*field_idx as usize] = val;
                        }
                    }
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::CreateStruct {
                dst,
                type_name,
                fields,
            } => {
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
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Tuple(field_values));
                let vtable = self.build_vtable(type_name);
                let struct_val = RuntimeValue::Struct {
                    type_id: crate::backends::common::value::TypeId(0),
                    fields: handle,
                    vtable,
                };
                frame.set_register(dst.0 as usize, struct_val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::BoundsCheck { array, index } => {
                let arr = self.force_register(frame, *array)?;
                let idx = self.force_register(frame, *index)?.to_int().unwrap_or(-1);
                let len = match &arr {
                    RuntimeValue::List(h) | RuntimeValue::Tuple(h) | RuntimeValue::Array(h) => {
                        match self.heap.get(*h) {
                            Some(crate::backends::common::HeapValue::List(list)) => {
                                list.len() as i64
                            }
                            Some(crate::backends::common::HeapValue::Tuple(t)) => t.len() as i64,
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
                Ok(StepOutcome::Continue)
            }

            // ── String operations ────────────────────────────────
            BytecodeInstr::StringConcat { dst, str1, str2 } => {
                let s1: String = match self.force_register(frame, *str1)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let s2: String = match self.force_register(frame, *str2)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                frame.set_register(
                    dst.0 as usize,
                    RuntimeValue::String(format!("{}{}", s1, s2).into()),
                );
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringLength { dst, src } => {
                let s: String = match self.force_register(frame, *src)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                frame.set_register(dst.0 as usize, RuntimeValue::Int(s.len() as i64));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringEqual { dst, str1, str2 } => {
                let s1: String = match self.force_register(frame, *str1)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let s2: String = match self.force_register(frame, *str2)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                frame.set_register(
                    dst.0 as usize,
                    RuntimeValue::Int(if s1 == s2 { 1 } else { 0 }),
                );
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringGetChar { dst, src, index } => {
                let s: String = match self.force_register(frame, *src)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let result = s
                    .chars()
                    .nth(index.0 as usize)
                    .map(|c| RuntimeValue::Char(c as u32))
                    .unwrap_or(RuntimeValue::Unit);
                frame.set_register(dst.0 as usize, result);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringFromInt { dst, src } => {
                let val = self.force_register(frame, *src)?.to_int().unwrap_or(0);
                frame.set_register(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringFromFloat { dst, src } => {
                let val = self.force_register(frame, *src)?.to_float().unwrap_or(0.0);
                frame.set_register(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Reference counting ──────────────────────────────
            BytecodeInstr::ArcNew { dst, src } => {
                let val = frame
                    .registers
                    .get(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                frame.set_register(dst.0 as usize, val.into_arc());
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::RcNew { dst, src } => {
                let val = frame
                    .registers
                    .get(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                frame.set_register(dst.0 as usize, val.into_arc());
                frame.advance();
                Ok(StepOutcome::Continue)
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
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::WeakNew { dst, src } => {
                let val = frame
                    .registers
                    .get(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                if let RuntimeValue::Arc(arc) = val {
                    frame.set_register(
                        dst.0 as usize,
                        RuntimeValue::Weak(std::sync::Arc::downgrade(&arc)),
                    );
                } else {
                    frame.set_register(dst.0 as usize, RuntimeValue::Unit);
                }
                frame.advance();
                Ok(StepOutcome::Continue)
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
                        frame.set_register(dst.0 as usize, RuntimeValue::Unit);
                    }
                } else {
                    frame.set_register(dst.0 as usize, RuntimeValue::Unit);
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Borrow (ZST, runtime equivalent to Mov) ─────────
            BytecodeInstr::Borrow { dst, src, .. } => {
                let val = frame
                    .registers
                    .get(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Unit);
                frame.set_register(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Closures ────────────────────────────────────────
            BytecodeInstr::MakeClosure {
                dst,
                func: func_ref,
                env,
            } => {
                let func_id = match func_ref {
                    FunctionRef::Static { name, .. } => {
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
                Ok(StepOutcome::Continue)
            }

            // ── Type operations ──────────────────────────────────
            BytecodeInstr::TypeOf { dst, src } => {
                let val = self.force_register(frame, *src)?;
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
                    RuntimeValue::OpaqueHandle { .. } => "OpaqueHandle",
                };
                frame.set_register(
                    dst.0 as usize,
                    RuntimeValue::String(std::sync::Arc::from(type_name)),
                );
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::Cast {
                dst,
                src,
                target_type_id,
            } => {
                let val = self.force_register(frame, *src)?;
                let result = match (val, *target_type_id) {
                    (RuntimeValue::Int(n), 1) => RuntimeValue::Float(n as f64),
                    (RuntimeValue::Float(f), 0) => RuntimeValue::Int(f as i64),
                    (RuntimeValue::Int(n), 2) => RuntimeValue::Bool(n != 0),
                    (RuntimeValue::Bool(b), 0) => RuntimeValue::Int(if b { 1 } else { 0 }),
                    (v, _) => v,
                };
                frame.set_register(dst.0 as usize, result);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::TypeCheck { value, type_id } => {
                let val = self.force_register(frame, *value)?;
                let actual_id: u16 = match val {
                    RuntimeValue::Int(_) => 0,
                    RuntimeValue::Float(_) => 1,
                    RuntimeValue::Bool(_) => 2,
                    RuntimeValue::String(_) => 3,
                    RuntimeValue::Char(_) => 4,
                    RuntimeValue::Unit => 5,
                    _ => u16::MAX,
                };
                if actual_id != *type_id && *type_id != u16::MAX {
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
                Ok(StepOutcome::Continue)
            }

            // ── Error handling ───────────────────────────────────
            BytecodeInstr::Throw { error: _ } => {
                let stack = self.capture_stack();
                Err(ExecutorError::runtime(
                    "User thrown error".to_string(),
                    stack,
                ))
            }
        }
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
        self.step_one()?;
        Ok(())
    }

    fn step_over(&mut self) -> ExecutorResult<()> {
        let depth = self.call_stack.len();
        // Execute the current instruction
        self.step_one()?;
        // If it was a function call, wait for it to complete
        while self.call_stack.len() > depth {
            match self.run_until_stop()? {
                StopReason::Breakpoint | StopReason::Completed => return Ok(()),
                StopReason::Returned => {}
            }
        }
        Ok(())
    }

    fn step_out(&mut self) -> ExecutorResult<()> {
        let depth = self.call_stack.len();
        loop {
            match self.run_until_stop()? {
                StopReason::Breakpoint | StopReason::Completed => return Ok(()),
                StopReason::Returned => {
                    if self.call_stack.len() < depth {
                        return Ok(());
                    }
                }
            }
        }
    }

    fn run(&mut self) -> ExecutorResult<()> {
        loop {
            match self.run_until_stop()? {
                StopReason::Breakpoint | StopReason::Completed => return Ok(()),
                StopReason::Returned => {
                    if self.call_stack.is_empty() {
                        return Ok(());
                    }
                }
            }
        }
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
