//! Debugger implementation for YaoXiang bytecode interpreter
//!
//! This module contains the DebuggableExecutor trait implementation and the
//! core stepping engine (step_one / execute_instr / run_until_stop).

use crate::backends::{DebuggableExecutor, ExecutorError, ExecutorResult};
use crate::backends::common::RuntimeValue;
use crate::middle::bytecode::{BytecodeInstr, ConstValue, Label, Reg};
use crate::backends::common::value::FunctionId;
use crate::backends::common::value::TypeId;
use super::executor::Interpreter;

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
/// #300 D 项：len 为真实容器长度，负索引诊断携带原始 i64（不再是 max=0 哨兵）
/// 索引值 → usize；非 Int 报类型错误，负数报运行时错误（不再静默当 0）。
///
/// `index_slot` 记录索引所在的寄存器号，供诊断层反查局部变量名——
/// `a[k]` 越界时告诉用户“k 这个变量越界了”比只给数值有用。
/// 传 None 表示索引来自编译器临时寄存器（无源码名）。
fn index_arg_at(
    idx: &RuntimeValue,
    what: &str,
    len: usize,
    index_slot: Option<usize>,
) -> ExecutorResult<usize> {
    let i = idx
        .to_int()
        .ok_or_else(|| ExecutorError::type_only(format!("{what} index must be an Int")))?;
    usize::try_from(i).map_err(|_| {
        // #299 §4: 负索引归并到 IndexOutOfBounds（E6003），不再落通用 E6007
        ExecutorError::IndexOutOfBounds {
            max: len,
            index: i,
            index_slot,
            stack: None,
        }
    })
}

impl Interpreter {
    /// 从常量池解析字符串（变体组名等）
    fn const_string(
        &self,
        idx: u16,
    ) -> String {
        self.image
            .constants
            .get(idx as usize)
            .and_then(|c| {
                if let ConstValue::String(s) = c {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

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
    /// ## 帧原地驻留（不再 pop/push）
    ///
    /// 此前实现把顶部帧从 `call_stack` 弹出、执行、再压回：在帧上执行需要
    /// `&mut self`（改堆/运行时），而帧存于 `self.call_stack`，二者借用冲突，
    /// 故只能先弹出。代价是每条指令搬一次 `Frame`（含 3 个 `Vec` 与槽位数组）。
    ///
    /// 现改为按索引就地访问：所有对帧的读写都是 `self.call_stack[fi]` 形式的
    /// 瞬时借用，与 `&mut self.heap` 等顺序执行、互不重叠，因此无需弹出。
    /// 附带修正：调用者帧全程留在栈上，栈帧捕获可拿到完整调用链。
    ///
    /// 指令数据统一经 `image` 读取——`Frame` 只持 `func_id`，不按值携带函数体。
    pub(super) fn step_one(&mut self) -> ExecutorResult<StepOutcome> {
        if self.call_stack.is_empty() {
            return Ok(StepOutcome::Returned);
        }
        let fi = self.call_stack.len() - 1;

        let fid = self.call_stack[fi].func_id as usize;
        if fid >= self.image.functions_by_id.len() {
            let stack = self.capture_stack();
            return Err(ExecutorError::function_not_found(
                format!("Frame holds invalid func_id {fid}"),
                stack,
            ));
        }

        if self.call_stack[fi].ip >= self.image.functions_by_id[fid].instructions.len() {
            self.call_stack.pop();
            return Ok(StepOutcome::Returned);
        }

        let ip = self.call_stack[fi].ip;
        let instr = self.image.functions_by_id[fid].instructions[ip].clone();
        self.execute_instr(fi, &instr)
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

    /// Execute a single instruction on the frame at `call_stack[fi]`.
    ///
    /// This is the instruction dispatcher — all instruction logic lives here.
    ///
    /// 帧不再作为 `&mut Frame` 传入，而是用索引定位：所有对帧的访问都是
    /// `self.call_stack[fi]` 的瞬时借用，因此 `&mut self`（堆、运行时、
    /// 甚至嵌套调用时向同一 `call_stack` 压帧）始终可用。
    fn execute_instr(
        &mut self,
        fi: usize,
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
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── Return ──────────────────────────────────────────
            BytecodeInstr::Return => {
                for task_id in self.call_stack[fi].take_all_spawned_tasks() {
                    let mut v = self.make_async_pending(task_id);
                    self.force_value_in_place(&mut v)?;
                }
                self.last_return_value = RuntimeValue::Void;
                // 帧原地驻留后，返回由指令自身弹出（旧实现由 step_one 省略 push 完成）
                self.call_stack.pop();
                Ok(StepOutcome::Returned)
            }
            BytecodeInstr::ReturnValue { value } => {
                let result = self.call_stack[fi]
                    .get_slot(value.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                for task_id in self.call_stack[fi].take_all_spawned_tasks() {
                    let mut v = self.make_async_pending(task_id);
                    self.force_value_in_place(&mut v)?;
                }
                self.last_return_value = result;
                // 帧原地驻留后，返回由指令自身弹出
                self.call_stack.pop();
                Ok(StepOutcome::Returned)
            }

            // ── Jumps ───────────────────────────────────────────
            BytecodeInstr::Jmp { target } => {
                let offset = Self::decode_label_offset(*target);
                self.call_stack[fi].ip = ((self.call_stack[fi].ip as i32) + offset) as usize;
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::JmpIf { cond, target } => {
                let c = self.force_slot(fi, *cond)?.to_bool().ok_or_else(|| {
                    ExecutorError::type_error("JmpIf 条件值不是布尔类型", self.capture_stack())
                })?;
                if c {
                    let offset = Self::decode_label_offset(*target);
                    self.call_stack[fi].ip = ((self.call_stack[fi].ip as i32) + offset) as usize;
                } else {
                    self.call_stack[fi].advance();
                }
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::JmpIfNot { cond, target } => {
                let c = self.force_slot(fi, *cond)?.to_bool().ok_or_else(|| {
                    ExecutorError::type_error("JmpIfNot 条件值不是布尔类型", self.capture_stack())
                })?;
                if !c {
                    let offset = Self::decode_label_offset(*target);
                    self.call_stack[fi].ip = ((self.call_stack[fi].ip as i32) + offset) as usize;
                } else {
                    self.call_stack[fi].advance();
                }
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::Switch { value, targets } => {
                let val = self.force_slot(fi, *value)?;
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
                            self.call_stack[fi].ip =
                                ((self.call_stack[fi].ip as i32) + offset) as usize;
                            jumped = true;
                            break;
                        }
                    }
                }
                if !jumped {
                    if let Some((None, default_target)) = targets.last() {
                        let offset = Self::decode_label_offset(*default_target);
                        self.call_stack[fi].ip =
                            ((self.call_stack[fi].ip as i32) + offset) as usize;
                    } else {
                        self.call_stack[fi].advance();
                    }
                }
                Ok(StepOutcome::Continue)
            }

            // ── Register operations ─────────────────────────────
            BytecodeInstr::Mov { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadConst { dst, const_idx } => {
                let val = self.load_constant(*const_idx);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadLocal { dst, local_idx } => {
                let val = self.call_stack[fi]
                    .get_slot(*local_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreLocal { local_idx, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(*local_idx as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadArg { dst, arg_idx } => {
                // Args are stored in locals by Frame::with_args
                let val = self.call_stack[fi]
                    .get_slot(*arg_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadGlobal { dst, global_idx } => {
                let val = self
                    .global_slots
                    .get(*global_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreGlobal { global_idx, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                let idx = *global_idx as usize;
                if idx >= self.global_slots.len() {
                    self.global_slots.resize(idx + 1, RuntimeValue::Void);
                }
                self.global_slots[idx] = val;
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadUpvalue { dst, upvalue_idx } => {
                let val = self.call_stack[fi]
                    .get_upvalue(*upvalue_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreUpvalue { src, upvalue_idx } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .expect("register index out of bounds");
                self.call_stack[fi].set_upvalue(*upvalue_idx as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── Arithmetic / comparison ─────────────────────────
            BytecodeInstr::BinaryOp { dst, lhs, rhs, op } => {
                self.exec_binary_op(*dst, *lhs, *rhs, *op, fi)?;
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::Compare { dst, lhs, rhs, cmp } => {
                self.exec_compare(*dst, *lhs, *rhs, *cmp, fi)?;
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::UnaryOp { dst, src, op } => {
                let val = self.force_slot(fi, *src)?;
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
                self.call_stack[fi].set_slot(dst.0 as usize, result);
                self.call_stack[fi].advance();
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
                    .image
                    .functions_by_id
                    .get(*func_idx as usize)
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| format!("fn_{}", func_idx));

                let call_args: Vec<RuntimeValue> = arg_regs
                    .iter()
                    .map(|r| {
                        self.call_stack[fi]
                            .get_slot(r.0 as usize)
                            .cloned()
                            .unwrap_or(RuntimeValue::Void)
                    })
                    .collect();

                let runtime = self.runtime_config.mode;

                if matches!(runtime, crate::backends::runtime::RuntimeMode::Embedded) {
                    let result = self.call_static_by_id(func_id, &call_args)?;
                    if let Some(dst_reg) = dst {
                        self.call_stack[fi].set_slot(dst_reg.index() as usize, result);
                    }
                    self.call_stack[fi].advance();
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
                    self.call_stack[fi].set_slot(dst_reg.index() as usize, v);
                }

                self.call_stack[fi].advance();
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
                        self.call_stack[fi]
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
                        self.call_stack[fi].set_slot(dst_reg.index() as usize, result);
                    }
                    self.call_stack[fi].advance();
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
                    self.call_stack[fi].set_slot(dst_reg.index() as usize, v);
                }

                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::CallVirt {
                dst,
                obj,
                method_idx,
                args,
            } => {
                let obj_val = self.force_slot(fi, *obj)?;

                let method_name = self
                    .image
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
                        call_args.push(self.force_slot(fi, *r)?);
                    }
                    let result = self.call_function_by_id(func_value.func_id, &call_args)?;
                    if let Some(dst_reg) = dst {
                        self.call_stack[fi].set_slot(dst_reg.index() as usize, result);
                    }
                } else {
                    // RFC-011a 阶段3 加固：vtable 缺方法不再静默写 Void——
                    // 静默路径曾把「分发错位」掩盖成空值（错误数据类别）。
                    // 与存在类型 VariantTag/VariantPayload 的运行时守卫对称。
                    let obj_desc = format!("{:?}", obj_val.value_type_simple());
                    return Err(ExecutorError::runtime_only(format!(
                        "虚表分发找不到方法 '{}'（接收者类型 {}）",
                        method_name, obj_desc
                    )));
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::CallDyn {
                dst,
                obj,
                name_idx: _,
                args,
            } => {
                let closure_val = self.force_slot(fi, *obj)?;

                if let RuntimeValue::Function(func_value) = closure_val {
                    let env_args: Vec<RuntimeValue> = func_value.env.clone();
                    let mut call_args = Vec::with_capacity(args.len());
                    for r in args {
                        call_args.push(self.force_slot(fi, *r)?);
                    }
                    let mut final_args = env_args.clone();
                    final_args.extend(call_args);
                    // env 同时作为 upvalues 注入新帧（LoadUpvalue 读）。
                    // 此前走 call_function_by_id 只传 args、不设 upvalues，
                    // 导致闭包体 `LoadUpvalue` 恒读 Void（实测 `adder(10)(5)` 得 10 而非 15）。
                    let result = self.call_closure(func_value.func_id, &final_args, &env_args)?;
                    if let Some(dst_reg) = dst {
                        self.call_stack[fi].set_slot(dst_reg.index() as usize, result);
                    }
                } else {
                    if let Some(dst_reg) = dst {
                        self.call_stack[fi].set_slot(dst_reg.index() as usize, RuntimeValue::Void);
                    }
                }
                self.call_stack[fi].advance();
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
                        let closure_val = self.force_slot(fi, *func_reg)?;
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
                        self.call_stack[fi].set_slot(func_reg.0 as usize, _result);
                    }
                } else {
                    use crate::backends::runtime::engine::{ResourceKey, TaskMeta};
                    use std::sync::Arc;

                    let mut task_ids: Vec<(Reg, crate::backends::common::value::TaskId)> =
                        Vec::new();

                    for (i, func_reg) in closures.iter().enumerate() {
                        let closure_val = self.force_slot(fi, *func_reg)?;
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

                        self.call_stack[fi].record_spawned_task(task_id);
                        task_ids.push((*func_reg, task_id));
                    }

                    for (func_reg, task_id) in &task_ids {
                        let mut v = self.make_async_pending(*task_id);
                        self.force_value_in_place(&mut v)?;
                        self.call_stack[fi].set_slot(func_reg.0 as usize, v);
                    }
                }

                self.call_stack[fi].advance();
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

                let list_val = self.force_slot(fi, closures_list)?;
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

                        self.call_stack[fi].record_spawned_task(task_id);
                        spawned_tasks.push(task_id);
                    }

                    for task_id in &spawned_tasks {
                        let mut v = self.make_async_pending(*task_id);
                        self.force_value_in_place(&mut v)?;
                    }
                }

                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── Heap / collection operations ─────────────────────
            BytecodeInstr::HeapAlloc { dst, type_id: _ } => {
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Tuple(Vec::new()));
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Tuple(handle));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewListWithCap { dst, capacity } => {
                let handle = self.heap.allocate(crate::backends::common::HeapValue::List(
                    Vec::with_capacity(*capacity as usize),
                ));
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::List(handle));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewArray { dst, count } => {
                // #299 §2：定长数组——元素默认 Void 占位，后续由字面量/store 填充
                let items = vec![RuntimeValue::Void; *count as usize];
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Array(items));
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Array(handle));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewDict { dst, keys, values } => {
                let mut map = std::collections::HashMap::new();
                for (key_reg, val_reg) in keys.iter().zip(values.iter()) {
                    let key = self.call_stack[fi]
                        .get_slot(key_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Void);
                    let val = self.call_stack[fi]
                        .get_slot(val_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Void);
                    map.insert(key, val);
                }
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Dict(map));
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Dict(handle));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewTuple { dst, items } => {
                let mut tuple_items = Vec::with_capacity(items.len());
                for item_reg in items {
                    let item = self.call_stack[fi]
                        .get_slot(item_reg.0 as usize)
                        .cloned()
                        .unwrap_or(RuntimeValue::Void);
                    tuple_items.push(item);
                }
                let handle = self
                    .heap
                    .allocate(crate::backends::common::HeapValue::Tuple(tuple_items));
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Tuple(handle));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::NewRange {
                dst,
                start,
                end,
                step,
            } => {
                // #302：三标量内联记录，构造点已拦 step=0（字面量）；动态零走 std.range.contains 显式错误
                let read = |r: &Reg| -> i64 {
                    match self.call_stack[fi].get_slot(r.0 as usize) {
                        Some(RuntimeValue::Int(n)) => *n,
                        _ => 0,
                    }
                };
                let (s, e, p) = (read(start), read(end), read(step));
                self.call_stack[fi].set_slot(
                    dst.0 as usize,
                    RuntimeValue::Range {
                        start: s,
                        end: e,
                        step: p,
                    },
                );
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // RFC-011a §6: 包装具体值为存在类型变体（Animal$Group.Dog(payload)）
            BytecodeInstr::CreateVariant {
                dst,
                group_idx,
                variant,
                payload,
            } => {
                let payload_val = self.force_slot(fi, *payload)?;
                let group = self.const_string(*group_idx);
                let _ = group;
                self.call_stack[fi].set_slot(
                    dst.0 as usize,
                    RuntimeValue::Enum {
                        type_id: TypeId::ENUM,
                        variant_id: *variant,
                        payload: Box::new(payload_val),
                    },
                );
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // RFC-011a §6: 变体号提取。守卫：obj 必须是变体值——漏包装在此显式
            // 报错（四层防御第③层），绝不静默产出错误数据
            BytecodeInstr::VariantTag {
                dst,
                obj,
                group_idx,
            } => {
                let obj_val = self.force_slot(fi, *obj)?;
                let group = self.const_string(*group_idx);
                let tag = match &obj_val {
                    RuntimeValue::Enum { variant_id, .. } => *variant_id as i64,
                    other => {
                        let actual = format!("{:?}", other.value_type_simple());
                        return Err(ExecutorError::runtime_only(format!(
                            "存在类型分发收到未包装的值（期望 {} 的变体，实际是 {}）——                             值未经包装进入存在类型位置，属编译器包装点遗漏",
                            group, actual
                        )));
                    }
                };
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Int(tag));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // RFC-011a §6: 变体负载提取（守卫同 VariantTag）
            BytecodeInstr::VariantPayload {
                dst,
                obj,
                group_idx,
            } => {
                let obj_val = self.force_slot(fi, *obj)?;
                let group = self.const_string(*group_idx);
                let payload = match obj_val {
                    RuntimeValue::Enum { payload, .. } => *payload,
                    other => {
                        let actual = format!("{:?}", other.value_type_simple());
                        return Err(ExecutorError::runtime_only(format!(
                            "存在类型分发收到未包装的值（期望 {} 的变体，实际是 {}）——                             值未经包装进入存在类型位置，属编译器包装点遗漏",
                            group, actual
                        )));
                    }
                };
                self.call_stack[fi].set_slot(dst.0 as usize, payload);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // #299 §3: membership 谓词——命中 true / 未命中 false，不报错（问 vs 断言）
            BytecodeInstr::Contains {
                dst,
                elem,
                container,
            } => {
                let e = self.force_slot(fi, *elem)?;
                let cval = self.force_slot(fi, *container)?;
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
                        RuntimeValue::Char(ch) => match char::from_u32(*ch) {
                            Some(c) => s.contains(c),
                            // 无效码点不可能存在于合法 Char 值，落到即运行时数据损坏
                            None => {
                                return Err(ExecutorError::runtime_only(format!(
                                    "internal: invalid Char code point {ch} in 'in' check"
                                )))
                            }
                        },
                        // #300 B 项：非 String/Char 探 String 是类型层漏拦，不静默 false
                        _ => {
                            return Err(ExecutorError::runtime_only(format!(
                                "internal: 'in' on String with non-string element ({e}) — typecheck missed this"
                            )))
                        }
                    },
                    // #300 B 项：非容器落到这里是类型层漏拦（白名单已拦），
                    // 静默 false 是 #279 同款疾病，改为内部错误
                    _ => {
                        return Err(ExecutorError::runtime_only(format!(
                            "internal: 'in' on non-container value ({cval}) — typecheck missed this"
                        )))
                    }
                };
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Bool(found));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadElement { dst, array, index } => {
                let arr = self.force_slot(fi, *array)?;
                let idx_value = self.force_slot(fi, *index)?;

                match arr {
                    RuntimeValue::List(handle) => {
                        let len = handle.lock().len();
                        let idx = index_arg_at(&idx_value, "list", len, Some(index.0 as usize))?;
                        if let crate::backends::common::HeapValue::List(items) = &*handle.lock() {
                            if idx < items.len() {
                                self.call_stack[fi].set_slot(dst.0 as usize, items[idx].clone());
                            } else {
                                // #279：越界读不再静默返回 void；#280：报专用码 E6003
                                // 带上索引寄存器号，供诊断层回溯源码变量名
                                return Err(ExecutorError::IndexOutOfBounds {
                                    max: items.len(),
                                    index: idx as i64,
                                    index_slot: Some(index.0 as usize),
                                    stack: Some(self.capture_stack()),
                                });
                            }
                        }
                    }
                    RuntimeValue::Tuple(handle) => {
                        let len = handle.lock().len();
                        let idx = index_arg_at(&idx_value, "tuple", len, Some(index.0 as usize))?;
                        if let crate::backends::common::HeapValue::Tuple(items) = &*handle.lock() {
                            if idx < items.len() {
                                self.call_stack[fi].set_slot(dst.0 as usize, items[idx].clone());
                            } else {
                                // #279：越界读不再静默返回 void；#280：报专用码 E6003
                                // 带上索引寄存器号，供诊断层回溯源码变量名
                                return Err(ExecutorError::IndexOutOfBounds {
                                    max: items.len(),
                                    index: idx as i64,
                                    index_slot: Some(index.0 as usize),
                                    stack: Some(self.capture_stack()),
                                });
                            }
                        }
                    }
                    RuntimeValue::Array(handle) => {
                        let len = handle.lock().len();
                        let idx = index_arg_at(&idx_value, "array", len, Some(index.0 as usize))?;
                        if let crate::backends::common::HeapValue::Array(items) = &*handle.lock() {
                            if idx < items.len() {
                                self.call_stack[fi].set_slot(dst.0 as usize, items[idx].clone());
                            } else {
                                // #279：越界读不再静默返回 void；#280：报专用码 E6003
                                // 带上索引寄存器号，供诊断层回溯源码变量名
                                return Err(ExecutorError::IndexOutOfBounds {
                                    max: items.len(),
                                    index: idx as i64,
                                    index_slot: Some(index.0 as usize),
                                    stack: Some(self.capture_stack()),
                                });
                            }
                        }
                    }
                    RuntimeValue::Dict(handle) => {
                        if let crate::backends::common::HeapValue::Dict(map) = &*handle.lock() {
                            match map.get(&idx_value) {
                                Some(value) => {
                                    self.call_stack[fi].set_slot(dst.0 as usize, value.clone())
                                }
                                // #299：缺键不再静默返回 void（同 #279 方向）
                                None => {
                                    return Err(ExecutorError::KeyNotFound {
                                        key: format!("{}", idx_value),
                                        stack: Some(self.capture_stack()),
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
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreElement {
                array,
                index,
                value,
            } => {
                let arr = self.force_slot(fi, *array)?;
                let idx_value = self.force_slot(fi, *index)?;
                let val = self.force_slot(fi, *value)?;

                match arr {
                    RuntimeValue::List(handle) => {
                        let len = handle.lock().len();
                        let idx = index_arg_at(&idx_value, "list", len, Some(index.0 as usize))?;
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
                                    idx as i64,
                                    Some(self.capture_stack()),
                                ));
                            }
                        }
                    }
                    RuntimeValue::Array(handle) => {
                        let len = handle.lock().len();
                        let idx = index_arg_at(&idx_value, "array", len, Some(index.0 as usize))?;
                        if let crate::backends::common::HeapValue::Array(items) =
                            &mut *handle.lock()
                        {
                            if idx < items.len() {
                                items[idx] = val;
                            } else {
                                // #279：越界写不再静默丢弃；#280：报专用码 E6003
                                return Err(ExecutorError::index_out_of_bounds(
                                    items.len(),
                                    idx as i64,
                                    Some(self.capture_stack()),
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
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::GetField {
                dst,
                src,
                field_idx,
            } => {
                let obj = self.force_slot(fi, *src)?;
                if let RuntimeValue::Struct { fields, .. } = obj {
                    if let crate::backends::common::HeapValue::Tuple(items) = &*fields.lock() {
                        if (*field_idx as usize) < items.len() {
                            self.call_stack[fi]
                                .set_slot(dst.0 as usize, items[*field_idx as usize].clone());
                        }
                    }
                } else if let RuntimeValue::Range { start, end, step } = obj {
                    // #302：Range 具名字段（start=0, end=1, step=2）
                    let v = match field_idx {
                        0 => start,
                        1 => end,
                        _ => step,
                    };
                    self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Int(v));
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::SetField {
                src,
                field_idx,
                value,
            } => {
                let obj = self.force_slot(fi, *src)?;
                let val = self.force_slot(fi, *value)?;
                if let RuntimeValue::Struct { fields, .. } = obj {
                    if let crate::backends::common::HeapValue::Tuple(items) = &mut *fields.lock() {
                        if (*field_idx as usize) < items.len() {
                            items[*field_idx as usize] = val;
                        }
                    }
                }
                self.call_stack[fi].advance();
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
                        self.call_stack[fi]
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
                self.call_stack[fi].set_slot(dst.0 as usize, struct_val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::BoundsCheck { array, index } => {
                let arr = self.force_slot(fi, *array)?;
                let idx = self.force_slot(fi, *index)?.to_int().unwrap_or(-1);
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
                        idx,
                        Some(stack),
                    ));
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── String operations ────────────────────────────────
            BytecodeInstr::StringConcat { dst, str1, str2 } => {
                let s1: String = match self.force_slot(fi, *str1)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let s2: String = match self.force_slot(fi, *str2)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                self.call_stack[fi].set_slot(
                    dst.0 as usize,
                    RuntimeValue::String(format!("{}{}", s1, s2).into()),
                );
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringLength { dst, src } => {
                let s: String = match self.force_slot(fi, *src)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Int(s.len() as i64));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringEqual { dst, str1, str2 } => {
                let s1: String = match self.force_slot(fi, *str1)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let s2: String = match self.force_slot(fi, *str2)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                self.call_stack[fi].set_slot(
                    dst.0 as usize,
                    RuntimeValue::Int(if s1 == s2 { 1 } else { 0 }),
                );
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringGetChar { dst, src, index } => {
                let s: String = match self.force_slot(fi, *src)? {
                    RuntimeValue::String(s) => s.as_ref().to_string(),
                    _ => String::new(),
                };
                let result = s
                    .chars()
                    .nth(index.0 as usize)
                    .map(|c| RuntimeValue::Char(c as u32))
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, result);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringFromInt { dst, src } => {
                let val = self.force_slot(fi, *src)?.to_int().ok_or_else(|| {
                    ExecutorError::type_error(
                        "StringFromInt 操作数不是 Int 类型",
                        self.capture_stack(),
                    )
                })?;
                self.call_stack[fi]
                    .set_slot(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StringFromFloat { dst, src } => {
                let val = self.force_slot(fi, *src)?.to_float().ok_or_else(|| {
                    ExecutorError::type_error(
                        "StringFromFloat 操作数不是 Float 类型",
                        self.capture_stack(),
                    )
                })?;
                self.call_stack[fi]
                    .set_slot(dst.0 as usize, RuntimeValue::String(val.to_string().into()));
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // ── Reference counting ──────────────────────────────
            BytecodeInstr::ArcNew { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val.into_arc());
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::RcNew { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val.into_arc());
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::ArcClone { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                if let RuntimeValue::Arc(inner) = val {
                    self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Arc(inner));
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::WeakNew { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                if let RuntimeValue::Arc(arc) = val {
                    self.call_stack[fi].set_slot(
                        dst.0 as usize,
                        RuntimeValue::Weak(std::sync::Arc::downgrade(&arc)),
                    );
                } else {
                    self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Void);
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::WeakUpgrade { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                if let RuntimeValue::Weak(weak) = val {
                    if let Some(arc) = weak.upgrade() {
                        self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Arc(arc));
                    } else {
                        self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Void);
                    }
                } else {
                    self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Void);
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── Borrow (ZST, runtime equivalent to Mov) ─────────
            BytecodeInstr::Borrow { dst, src, .. } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── Closures ────────────────────────────────────────
            BytecodeInstr::MakeClosure {
                dst,
                func: func_idx,
                env,
            } => {
                let func_id = if (*func_idx as usize) < self.image.functions_by_id.len() {
                    FunctionId(*func_idx)
                } else {
                    eprintln!(
                        "[warn] Closure: function index {} out of range ({}), fallback to id 0",
                        func_idx,
                        self.image.functions_by_id.len()
                    );
                    FunctionId(0)
                };
                let captured_env: Vec<RuntimeValue> = env
                    .iter()
                    .map(|r| {
                        self.call_stack[fi]
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
                self.call_stack[fi].set_slot(dst.0 as usize, closure);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── Type operations ──────────────────────────────────
            BytecodeInstr::TypeOf { dst, src } => {
                let val = self.force_slot(fi, *src)?;
                let type_name: &str = match &val {
                    RuntimeValue::Void => "Void",
                    RuntimeValue::Bool(_) => "Bool",
                    RuntimeValue::Int(_) => "Int",
                    RuntimeValue::Float(_) => "Float",
                    RuntimeValue::Char(_) => "Char",
                    RuntimeValue::String(_) => "String",
                    RuntimeValue::Bytes(_) => "Bytes",
                    RuntimeValue::Tuple(_) => "Tuple",
                    RuntimeValue::Range { .. } => "Range",
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
                self.call_stack[fi].set_slot(
                    dst.0 as usize,
                    RuntimeValue::String(std::sync::Arc::from(type_name)),
                );
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::Cast {
                dst,
                src,
                target_type_id,
            } => {
                let val = self.force_slot(fi, *src)?;
                let result = match (val, *target_type_id) {
                    (RuntimeValue::Int(n), 1) => RuntimeValue::Float(n as f64),
                    (RuntimeValue::Float(f), 0) => RuntimeValue::Int(f as i64),
                    (RuntimeValue::Int(n), 2) => RuntimeValue::Bool(n != 0),
                    (RuntimeValue::Bool(b), 0) => RuntimeValue::Int(if b { 1 } else { 0 }),
                    (v, _) => v,
                };
                self.call_stack[fi].set_slot(dst.0 as usize, result);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::TypeCheck { value, type_id } => {
                let val = self.force_slot(fi, *value)?;
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
                self.call_stack[fi].advance();
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
        self.call_stack
            .last()
            .and_then(|f| self.image.function_name(f.func_id as usize))
    }

    fn breakpoints(&self) -> Vec<usize> {
        self.breakpoints.keys().copied().collect()
    }
}
