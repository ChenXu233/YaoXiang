//! 调用与并发指令族：CallStatic / CallNative / CallVirt / CallDyn / Spawn
//!
//! `execute_instr` 按指令族拆分的一部分。arm 体自原单函数逐字搬迁，
//! 仅去掉外层 `match` 包裹并统一以 `return` 形式返回，语义不变。

use crate::backends::common::value::FunctionId;
use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::debug::StepOutcome;
use crate::backends::interpreter::executor::executor::InterpreterTask;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::ExecutorError;
use crate::middle::bytecode::{BytecodeInstr, ConstValue, Reg};

impl Interpreter {
    /// 执行「调用与并发」族指令。
    ///
    /// 由 `execute_instr` 分派调用；`#[inline]` 保证不引入额外调用开销。
    #[inline]
    pub(in crate::backends::interpreter::executor) fn exec_call(
        &mut self,
        fi: usize,
        instr: &BytecodeInstr,
    ) -> crate::backends::interpreter::executor::debug::ExecResult {
        let _ = fi;
        match instr {
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
                    InterpreterTask::Static {
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
                            InterpreterTask::Dyn {
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
                            InterpreterTask::Dyn {
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
            // 分派层（`execute_instr`）保证只有本族指令到达此处。
            // 出现其他指令说明分派表与该族定义不一致，属编译器内部错误。
            other => Err(ExecutorError::runtime_only(format!(
                "exec_call 收到非本族指令: {other:?}"
            ))),
        }
    }
}
