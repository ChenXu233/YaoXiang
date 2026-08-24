//! Debugger implementation for YaoXiang bytecode interpreter
//!
//! This module contains the DebuggableExecutor trait implementation and the
//! core stepping engine (step_one / execute_instr / run_until_stop).

use crate::backends::{DebuggableExecutor, ExecutorError, ExecutorResult};
use crate::backends::common::RuntimeValue;
use crate::middle::bytecode::{BytecodeInstr, ConstValue, Label, Reg};
use crate::backends::common::value::FunctionId;
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

/// #279：索引值 → usize；非 Int 报类型错误，负数报运行时错误（不再静默当 0）
fn index_arg(
    idx: &RuntimeValue,
    what: &str,
) -> ExecutorResult<usize> {
    let i = idx
        .to_int()
        .ok_or_else(|| ExecutorError::type_only(format!("{what} index must be an Int")))?;
    usize::try_from(i).map_err(|_| {
        // #299 §4: 负索引归并到 IndexOutOfBounds（E6003），不再落通用 E6007
        ExecutorError::IndexOutOfBounds {
            max: 0,
            index: (-i) as usize,
            stack: None,
        }
    })
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
                self.last_return_value = RuntimeValue::Void;
                // Frame is NOT pushed back — caller handles this
                Ok(StepOutcome::Returned)
            }
            BytecodeInstr::ReturnValue { value } => {
                let result = frame
                    .get_slot(value.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
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
                let c = self.force_slot(frame, *cond)?.to_bool().ok_or_else(|| {
                    ExecutorError::type_error("JmpIf 条件值不是布尔类型", self.capture_stack())
                })?;
                if c {
                    let offset = Self::decode_label_offset(*target);
                    frame.ip = ((frame.ip as i32) + offset) as usize;
                } else {
                    frame.advance();
                }
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::JmpIfNot { cond, target } => {
                let c = self.force_slot(frame, *cond)?.to_bool().ok_or_else(|| {
                    ExecutorError::type_error("JmpIfNot 条件值不是布尔类型", self.capture_stack())
                })?;
                if !c {
                    let offset = Self::decode_label_offset(*target);
                    frame.ip = ((frame.ip as i32) + offset) as usize;
                } else {
                    frame.advance();
                }
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::Switch { value, targets } => {
                let val = self.force_slot(frame, *value)?;
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
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                frame.set_slot(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadConst { dst, const_idx } => {
                let val = self.load_constant(*const_idx);
                frame.set_slot(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadLocal { dst, local_idx } => {
                let val = frame
                    .get_slot(*local_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                frame.set_slot(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreLocal { local_idx, src } => {
                let val = frame
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                frame.set_slot(*local_idx as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadArg { dst, arg_idx } => {
                // Args are stored in locals by Frame::with_args
                let val = frame
                    .get_slot(*arg_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                frame.set_slot(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadUpvalue { dst, upvalue_idx } => {
                let val = frame
                    .get_upvalue(*upvalue_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                frame.set_slot(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreUpvalue { src, upvalue_idx } => {
                let val = frame
                    .get_slot(src.0 as usize)
                    .cloned()
                    .expect("register index out of bounds");
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
                let val = self.force_slot(frame, *src)?;
                let result = match (op, val) {
                    (crate::middle::bytecode::UnaryOp::Neg, RuntimeValue::Int(n)) => {
                        RuntimeValue::Int(-n)
                    }
                    (crate::middle::bytecode::UnaryOp::Neg, RuntimeValue::Float(f)) => {
                        RuntimeValue::Float(-f)
                    }
                    (crate::middle::bytecode::UnaryOp::Neg, RuntimeValue::Bool(b)) => {
                        RuntimeValue::Bool(!b)
                    }
                    (crate::middle::bytecode::UnaryOp::Not, RuntimeValue::Int(n)) => {
                        RuntimeValue::Int(!n)
                    }
                    (crate::middle::bytecode::UnaryOp::Not, RuntimeValue::Bool(b)) => {
                        RuntimeValue::Bool(!b)
                    }
                    _ => {
                        let stack = self.capture_stack();
                        return Err(ExecutorError::type_error(
                            format!("type mismatch in unary operation {:?}", op),
                            stack,
                        ));
                    }
                };
                frame.set_slot(dst.0 as usize, result);
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Function calls ──────────────────────────────────
            BytecodeInstr::CallStatic {
                dst,
                func: func_idx,
                args: arg_regs,
            } => {
                let func_id = FunctionId(*func_idx);
                let func_label = self
                    .functions_by_id
                    .get(*func_idx as usize)
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| format!("fn_{}", func_idx));

                let call_args: Vec<RuntimeValue> = arg_regs
                    .iter()
                    .map(|r| {
                        frame
                            .get_slot(r.0 as usize)
                            .cloned()
                            .unwrap_or(RuntimeValue::Void)
                    })
                    .collect();

                let runtime = self.runtime_config.mode;

                if matches!(runtime, crate::backends::runtime::RuntimeMode::Embedded) {
                    let result = self.call_static_by_id(func_id, &call_args)?;
                    if let Some(dst_reg) = dst {
                        frame.set_slot(dst_reg.index() as usize, result);
                    }
                    frame.advance();
                    return Ok(StepOutcome::Continue);
                }

                use crate::backends::runtime::engine::{ResourceKey, TaskMeta};
                use std::sync::Arc;

                let deps = self.deps_from_args(&call_args);

                let task_id = self.schedule_task(
                    super::executor::InterpreterTask::Static {
                        func_id,
                        args: call_args.clone(),
                    },
                    TaskMeta {
                        deps,
                        resources: Vec::<ResourceKey>::new(),
                        label: Some(Arc::<str>::from(func_label.as_str())),
                    },
                )?;

                self.drive_dag_until(Some(task_id))?;
                let mut v = self.make_async_pending(task_id);
                self.force_value_in_place(&mut v)?;
                if let Some(dst_reg) = dst {
                    frame.set_slot(dst_reg.index() as usize, v);
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
                            .get_slot(r.0 as usize)
                            .cloned()
                            .unwrap_or(RuntimeValue::Void)
                    })
                    .collect();

                let runtime = self.runtime_config.mode;

                if matches!(runtime, crate::backends::runtime::RuntimeMode::Embedded) {
                    let result = self
                        .call_native_with_ffi_meta(func_name, mechanism, lib, symbol, &call_args)?;
                    if let Some(dst_reg) = dst {
                        frame.set_slot(dst_reg.index() as usize, result);
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
                    frame.set_slot(dst_reg.index() as usize, v);
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
                let obj_val = self.force_slot(frame, *obj)?;

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
                        call_args.push(self.force_slot(frame, *r)?);
                    }
                    let result = self.call_function_by_id(func_value.func_id, &call_args)?;
                    if let Some(dst_reg) = dst {
                        frame.set_slot(dst_reg.index() as usize, result);
                    }
                } else {
                    if let Some(dst_reg) = dst {
                        frame.set_slot(dst_reg.index() as usize, RuntimeValue::Void);
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
                let closure_val = self.force_slot(frame, *obj)?;

                if let RuntimeValue::Function(func_value) = closure_val {
                    let env_args: Vec<RuntimeValue> = func_value.env.clone();
                    let mut call_args = Vec::with_capacity(args.len());
                    for r in args {
                        call_args.push(self.force_slot(frame, *r)?);
                    }
                    let mut final_args = env_args;
                    final_args.extend(call_args);
                    let result = self.call_function_by_id(func_value.func_id, &final_args)?;
                    if let Some(dst_reg) = dst {
                        frame.set_slot(dst_reg.index() as usize, result);
                    }
                } else {
                    if let Some(dst_reg) = dst {
                        frame.set_slot(dst_reg.index() as usize, RuntimeValue::Void);
                    }
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            BytecodeInstr::Spawn {
                dst: _,
                closures,
                task_deps,
                task_resources,
            } => {
                let closures = closures.clone();
                let task_deps = task_deps.clone();
                let task_resources = task_resources.clone();
                let runtime = self.runtime_config.mode;

                if matches!(runtime, crate::backends::runtime::RuntimeMode::Embedded) {
                    for func_reg in closures.iter() {
                        let closure_val = self.force_slot(frame, *func_reg)?;
                        let RuntimeValue::Function(func_value) = closure_val else {
                            let stack = self.capture_stack();
                            return Err(ExecutorError::type_error(
                                "spawn expects a function value".to_string(),
                                stack,
                            ));
                        };
                        // #254：call_closure 设 upvalues（闭包体 LoadUpvalue 读 env），
                        // env 同时作 args 前段（LoadArg 读）。call_function_by_id 不设
                        // upvalues → 闭包捕获读 Void。
                        let _result = self.call_closure(
                            func_value.func_id,
                            &func_value.env,
                            &func_value.env,
                        )?;
                        frame.set_slot(func_reg.0 as usize, _result);
                    }
                } else {
                    use crate::backends::runtime::engine::{ResourceKey, TaskMeta};
                    use std::sync::Arc;

                    let mut task_ids: Vec<(Reg, crate::backends::common::value::TaskId)> =
                        Vec::new();

                    for (i, func_reg) in closures.iter().enumerate() {
                        let closure_val = self.force_slot(frame, *func_reg)?;
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
                        frame.set_slot(func_reg.0 as usize, v);
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

                let list_val = self.force_slot(frame, closures_list)?;
                let closures: Vec<RuntimeValue> = match list_val {
                    RuntimeValue::List(handle) => match &*handle.lock() {
                        crate::backends::common::HeapValue::List(items) => items.clone(),
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

                let runtime = self.runtime_config.mode;

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
                frame.set_slot(dst.0 as usize, RuntimeValue::Tuple(handle));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewListWithCap { dst, capacity } => {
                let handle = self.heap.allocate(crate::backends::common::HeapValue::List(
                    Vec::with_capacity(*capacity as usize),
                ));
                frame.set_slot(dst.0 as usize, RuntimeValue::List(handle));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewArray { dst, count } => {
                // #299 §2：定长数组——元素默认 Void 占位，后续由字面量/store 填充
                let items = vec![RuntimeValue::Void; *count as usize];
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Array(items));
                frame.set_slot(dst.0 as usize, RuntimeValue::Array(handle));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewDict { dst, keys, values } => {
                let mut map = std::collections::HashMap::new();
                for (key_reg, val_reg) in keys.iter().zip(values.iter()) {
                    let key = frame
                        .get_slot(key_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Void);
                    let val = frame
                        .get_slot(val_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Void);
                    map.insert(key, val);
                }
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Dict(map));
                frame.set_slot(dst.0 as usize, RuntimeValue::Dict(handle));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewTuple { dst, items } => {
                let mut tuple_items = Vec::with_capacity(items.len());
                for item_reg in items {
                    let item = frame
                        .get_slot(item_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Void);
                    tuple_items.push(item);
                }
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Tuple(tuple_items));
                frame.set_slot(dst.0 as usize, RuntimeValue::Tuple(handle));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            // #299 §3: membership 谓词——命中 true / 未命中 false，不报错（问 vs 断言）
            BytecodeInstr::Contains {
                dst,
                elem,
                container,
            } => {
                let e = self.force_slot(frame, *elem)?;
                let cval = self.force_slot(frame, *container)?;
                let found = match &cval {
                    RuntimeValue::List(h) => matches!(
                        &*h.lock(),
                        crate::backends::common::HeapValue::List(items)
                            if items.contains(&e)
                    ),
                    RuntimeValue::Array(h) => matches!(
                        &*h.lock(),
                        crate::backends::common::HeapValue::Array(items)
                            if items.contains(&e)
                    ),
                    RuntimeValue::Tuple(h) => matches!(
                        &*h.lock(),
                        crate::backends::common::HeapValue::Tuple(items)
                            if items.contains(&e)
                    ),
                    RuntimeValue::Dict(h) => matches!(
                        &*h.lock(),
                        crate::backends::common::HeapValue::Dict(map)
                            if map.contains_key(&e)
                    ),
                    RuntimeValue::String(s) => match &e {
                        RuntimeValue::String(sub) => s.contains(sub.as_ref()),
                        RuntimeValue::Char(ch) => s.contains(char::from_u32(*ch).unwrap_or(' ')),
                        _ => false,
                    },
                    _ => false,
                };
                frame.set_slot(dst.0 as usize, RuntimeValue::Bool(found));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadElement { dst, array, index } => {
                let arr = self.force_slot(frame, *array)?;
                let idx_value = self.force_slot(frame, *index)?;

                match arr {
                    RuntimeValue::List(handle) => {
                        let idx = index_arg(&idx_value, "list")?;
                        if let crate::backends::common::HeapValue::List(items) = &*handle.lock() {
                            if idx < items.len() {
                                frame.set_slot(dst.0 as usize, items[idx].clone());
                            } else {
                                // #279：越界读不再静默返回 void；#280：报专用码 E6003
                                return Err(ExecutorError::index_out_of_bounds(
                                    items.len(),
                                    idx,
                                    None,
                                ));
                            }
                        }
                    }
                    RuntimeValue::Tuple(handle) => {
                        let idx = index_arg(&idx_value, "tuple")?;
                        if let crate::backends::common::HeapValue::Tuple(items) = &*handle.lock() {
                            if idx < items.len() {
                                frame.set_slot(dst.0 as usize, items[idx].clone());
                            } else {
                                // #279：越界读不再静默返回 void；#280：报专用码 E6003
                                return Err(ExecutorError::index_out_of_bounds(
                                    items.len(),
                                    idx,
                                    None,
                                ));
                            }
                        }
                    }
                    RuntimeValue::Array(handle) => {
                        let idx = index_arg(&idx_value, "array")?;
                        if let crate::backends::common::HeapValue::Array(items) = &*handle.lock() {
                            if idx < items.len() {
                                frame.set_slot(dst.0 as usize, items[idx].clone());
                            } else {
                                // #279：越界读不再静默返回 void；#280：报专用码 E6003
                                return Err(ExecutorError::index_out_of_bounds(
                                    items.len(),
                                    idx,
                                    None,
                                ));
                            }
                        }
                    }
                    RuntimeValue::Dict(handle) => {
                        if let crate::backends::common::HeapValue::Dict(map) = &*handle.lock() {
                            match map.get(&idx_value) {
                                Some(value) => frame.set_slot(dst.0 as usize, value.clone()),
                                // #299：缺键不再静默返回 void（同 #279 方向）
                                None => {
                                    return Err(ExecutorError::KeyNotFound {
                                        key: format!("{:?}", idx_value),
                                        stack: None,
                                    });
                                }
                            }
                        }
                    }
                    // #299：不可索引类型（如 String）不再静默 void
                    _ => {
                        return Err(ExecutorError::type_only(
                            "indexing not supported on this value type",
                        ))
                    }
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreElement {
                array,
                index,
                value,
            } => {
                let arr = self.force_slot(frame, *array)?;
                let idx_value = self.force_slot(frame, *index)?;
                let val = self.force_slot(frame, *value)?;

                match arr {
                    RuntimeValue::List(handle) => {
                        let idx = index_arg(&idx_value, "list")?;
                        if let crate::backends::common::HeapValue::List(items) = &mut *handle.lock()
                        {
                            if idx < items.len() {
                                items[idx] = val;
                            } else if idx == items.len() {
                                items.push(val);
                            } else {
                                // #279：越界写不再静默丢弃；#280：报专用码 E6003
                                return Err(ExecutorError::index_out_of_bounds(
                                    items.len(),
                                    idx,
                                    None,
                                ));
                            }
                        }
                    }
                    RuntimeValue::Array(handle) => {
                        let idx = index_arg(&idx_value, "array")?;
                        if let crate::backends::common::HeapValue::Array(items) =
                            &mut *handle.lock()
                        {
                            if idx < items.len() {
                                items[idx] = val;
                            } else {
                                // #279：越界写不再静默丢弃；#280：报专用码 E6003
                                return Err(ExecutorError::index_out_of_bounds(
                                    items.len(),
                                    idx,
                                    None,
                                ));
                            }
                        }
                    }
                    RuntimeValue::Dict(handle) => {
                        if let crate::backends::common::HeapValue::Dict(map) = &mut *handle.lock() {
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
                let obj = self.force_slot(frame, *src)?;
                if let RuntimeValue::Struct { fields, .. } = obj {
                    if let crate::backends::common::HeapValue::Tuple(items) = &*fields.lock() {
                        if (*field_idx as usize) < items.len() {
                            frame.set_slot(dst.0 as usize, items[*field_idx as usize].clone());
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
                let obj = self.force_slot(frame, *src)?;
                let val = self.force_slot(frame, *value)?;
                if let RuntimeValue::Struct { fields, .. } = obj {
                    if let crate::backends::common::HeapValue::Tuple(items) = &mut *fields.lock() {
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
                            .get_slot(reg.0 as usize)
                            .cloned()
                            .unwrap_or(RuntimeValue::Void)
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
                frame.set_slot(dst.0 as usize, struct_val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::BoundsCheck { array, index } => {
                let arr = self.force_slot(frame, *array)?;
                let idx = self.force_slot(frame, *index)?.to_int().unwrap_or(-1);
                let len = match &arr {
                    RuntimeValue::List(h) | RuntimeValue::Tuple(h) | RuntimeValue::Array(h) => {
                        match &*h.lock() {
                            crate::backends::common::HeapValue::List(list) => list.len() as i64,
                            crate::backends::common::HeapValue::Tuple(t) => t.len() as i64,
                            _ => -1,
                        }
                    }
                    _ => -1,
                };
                if idx < 0 || idx >= len {
                    let stack = self.capture_stack();
                    // #280：越界用专用码 E6003（原 E6007 通用）
                    return Err(ExecutorError::index_out_of_bounds(
                        len.max(0) as usize,
                        idx.max(0) as usize,
                        Some(stack),
                    ));
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── String operations ────────────────────────────────
            BytecodeInstr::StringConcat { dst, str1, str2 } => {
                let s1: String = match self.force_slot(frame, *str1)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let s2: String = match self.force_slot(frame, *str2)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                frame.set_slot(
                    dst.0 as usize,
                    RuntimeValue::String(format!("{}{}", s1, s2).into()),
                );
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringLength { dst, src } => {
                let s: String = match self.force_slot(frame, *src)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                frame.set_slot(dst.0 as usize, RuntimeValue::Int(s.len() as i64));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringEqual { dst, str1, str2 } => {
                let s1: String = match self.force_slot(frame, *str1)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let s2: String = match self.force_slot(frame, *str2)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                frame.set_slot(
                    dst.0 as usize,
                    RuntimeValue::Int(if s1 == s2 { 1 } else { 0 }),
                );
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringGetChar { dst, src, index } => {
                let s: String = match self.force_slot(frame, *src)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let result = s
                    .chars()
                    .nth(index.0 as usize)
                    .map(|c| RuntimeValue::Char(c as u32))
                    .unwrap_or(RuntimeValue::Void);
                frame.set_slot(dst.0 as usize, result);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringFromInt { dst, src } => {
                let val = self.force_slot(frame, *src)?.to_int().ok_or_else(|| {
                    ExecutorError::type_error(
                        "StringFromInt 操作数不是 Int 类型",
                        self.capture_stack(),
                    )
                })?;
                frame.set_slot(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringFromFloat { dst, src } => {
                let val = self.force_slot(frame, *src)?.to_float().ok_or_else(|| {
                    ExecutorError::type_error(
                        "StringFromFloat 操作数不是 Float 类型",
                        self.capture_stack(),
                    )
                })?;
                frame.set_slot(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            // ── Reference counting ──────────────────────────────
            BytecodeInstr::ArcNew { dst, src } => {
                let val = frame
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                frame.set_slot(dst.0 as usize, val.into_arc());
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::RcNew { dst, src } => {
                let val = frame
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                frame.set_slot(dst.0 as usize, val.into_arc());
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::ArcClone { dst, src } => {
                let val = frame
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                if let RuntimeValue::Arc(inner) = val {
                    frame.set_slot(dst.0 as usize, RuntimeValue::Arc(inner));
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::WeakNew { dst, src } => {
                let val = frame
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                if let RuntimeValue::Arc(arc) = val {
                    frame.set_slot(
                        dst.0 as usize,
                        RuntimeValue::Weak(std::sync::Arc::downgrade(&arc)),
                    );
                } else {
                    frame.set_slot(dst.0 as usize, RuntimeValue::Void);
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::WeakUpgrade { dst, src } => {
                let val = frame
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                if let RuntimeValue::Weak(weak) = val {
                    if let Some(arc) = weak.upgrade() {
                        frame.set_slot(dst.0 as usize, RuntimeValue::Arc(arc));
                    } else {
                        frame.set_slot(dst.0 as usize, RuntimeValue::Void);
                    }
                } else {
                    frame.set_slot(dst.0 as usize, RuntimeValue::Void);
                }
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Borrow (ZST, runtime equivalent to Mov) ─────────
            BytecodeInstr::Borrow { dst, src, .. } => {
                let val = frame
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                frame.set_slot(dst.0 as usize, val);
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Closures ────────────────────────────────────────
            BytecodeInstr::MakeClosure {
                dst,
                func: func_idx,
                env,
            } => {
                let func_id = if (*func_idx as usize) < self.functions_by_id.len() {
                    FunctionId(*func_idx)
                } else {
                    eprintln!(
                        "[warn] Closure: function index {} out of range ({}), fallback to id 0",
                        func_idx,
                        self.functions_by_id.len()
                    );
                    FunctionId(0)
                };
                let captured_env: Vec<RuntimeValue> = env
                    .iter()
                    .map(|r| {
                        frame
                            .get_slot(r.0 as usize)
                            .cloned()
                            .unwrap_or(RuntimeValue::Void)
                    })
                    .collect();
                let closure =
                    RuntimeValue::Function(crate::backends::common::value::FunctionValue {
                        func_id,
                        env: captured_env,
                    });
                frame.set_slot(dst.0 as usize, closure);
                frame.advance();
                Ok(StepOutcome::Continue)
            }

            // ── Type operations ──────────────────────────────────
            BytecodeInstr::TypeOf { dst, src } => {
                let val = self.force_slot(frame, *src)?;
                let type_name: &str = match &val {
                    RuntimeValue::Void => "Void",
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
                frame.set_slot(
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
                let val = self.force_slot(frame, *src)?;
                let result = match (val, *target_type_id) {
                    (RuntimeValue::Int(n), 1) => RuntimeValue::Float(n as f64),
                    (RuntimeValue::Float(f), 0) => RuntimeValue::Int(f as i64),
                    (RuntimeValue::Int(n), 2) => RuntimeValue::Bool(n != 0),
                    (RuntimeValue::Bool(b), 0) => RuntimeValue::Int(if b { 1 } else { 0 }),
                    (v, _) => v,
                };
                frame.set_slot(dst.0 as usize, result);
                frame.advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::TypeCheck { value, type_id } => {
                let val = self.force_slot(frame, *value)?;
                let actual_id: u16 = match val {
                    RuntimeValue::Int(_) => 0,
                    RuntimeValue::Float(_) => 1,
                    RuntimeValue::Bool(_) => 2,
                    RuntimeValue::String(_) => 3,
                    RuntimeValue::Char(_) => 4,
                    RuntimeValue::Void => 5,
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
