//! 调用与并发指令族：CallStatic / CallNative / CallVirt / CallDyn / Spawn
//!
//! `execute_instr` 按指令族拆分的一部分。arm 体自原单函数逐字搬迁，
//! 仅去掉外层 `match` 包裹并统一以 `return` 形式返回，语义不变。

use crate::backends::common::value::FunctionId;
use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::debug::StepOutcome;
use crate::backends::interpreter::executor::executor::InterpreterTask;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::interpreter::PendingCall;
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
                // 注：函数名标签只在下方 Standard 模式的任务调度里用到
                // （TaskMeta.label）。此前在此无条件克隆函数名，而默认的
                // Embedded 模式（蹦床压帧）根本不用它——
                // 每次 CallStatic 都白做一次 String 分配。故移到使用处。
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
                    // 蹦床（WBS 3.9.1）：实参解析后就地压入被调帧，返回值落点由帧携带
                    // （`ReturnTarget`）——不再进入宿主驱动循环 `drive_frame`，
                    // 解释型深度因此不消耗原生栈，深度守卫必然可达。
                    let resolved = self.resolve_args(&call_args)?;
                    // 调用者 ip 暂不越过调用指令：被调帧返回时由 `return_from_frame`
                    // 推进（期间 ip 停在调用点，栈帧捕获才能定位到调用处——与原递归实现一致）。
                    self.start_call(PendingCall::function(func_id, resolved, *dst))?;
                    return Ok(StepOutcome::Continue);
                }

                use crate::backends::runtime::engine::{ResourceKey, TaskMeta};
                use std::sync::Arc;

                let deps = self.deps_from_args(&call_args);

                let label: Arc<str> = Arc::from(
                    self.image
                        .function_name(*func_idx as usize)
                        .unwrap_or("fn_?"),
                );

                let task_id = self.schedule_task(
                    InterpreterTask::Static {
                        func_id,
                        args: call_args.clone(),
                    },
                    TaskMeta {
                        deps,
                        resources: Vec::<ResourceKey>::new(),
                        label: Some(label),
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
                    // 蹦床：虚表方法压帧后交还外层 step_one，返回值写回调者 dst 槽；
                    // ip 越过调用指令由 `return_from_frame` 在被调返回时推进。
                    self.start_call(PendingCall::function(func_value.func_id, call_args, *dst))?;
                    return Ok(StepOutcome::Continue);
                }
                // RFC-011a 阶段3 加固：vtable 缺方法不再静默写 Void——
                // 静默路径曾把「分发错位」掩盖成空值（错误数据类别）。
                // 与存在类型 VariantTag/VariantPayload 的运行时守卫对称。
                let obj_desc = format!("{:?}", obj_val.value_type_simple());
                Err(ExecutorError::runtime_only(format!(
                    "虚表分发找不到方法 '{}'（接收者类型 {}）",
                    method_name, obj_desc
                )))
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
                    // 蹦床：闭包帧压栈（upvalues = env），返回值写回调者 dst 槽；
                    // ip 越过调用指令由 `return_from_frame` 在被调返回时推进。
                    self.start_call(PendingCall::closure(
                        func_value.func_id,
                        final_args,
                        env_args,
                        *dst,
                    ))?;
                    return Ok(StepOutcome::Continue);
                } else if let RuntimeValue::NativeFunction(name) = closure_val {
                    // #492：原生函数作值——按名分发到 FFI 注册表。非 FFI（std 内建）
                    // 的 mechanism 为空串，与 `CallNative`「无元数据」同口径。
                    let mut call_args = Vec::with_capacity(args.len());
                    for r in args {
                        call_args.push(self.force_slot(fi, *r)?);
                    }
                    let result =
                        self.call_native_with_ffi_meta(name.as_ref(), "", "", "", &call_args)?;
                    if let Some(dst_reg) = dst {
                        self.call_stack[fi].set_slot(dst_reg.index() as usize, result);
                    }
                    self.call_stack[fi].advance();
                    return Ok(StepOutcome::Continue);
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
                    // 多闭包 spawn 是一条指令发起 N 个调用：首个立即压帧，其余排进
                    // 本帧待启动队列（`start_calls`），由返回路径逐个续跑——
                    // 一条指令体内不再有递归的解释型调用。
                    // 注：类型校验在压帧前一次性完成（此前是逐个执行中校验）；
                    // 对良构程序无差异，含非函数项的动态 spawn 仍在本指令报同一错误。
                    let mut calls = Vec::with_capacity(closures.len());
                    for func_reg in closures.iter() {
                        let closure_val = self.force_slot(fi, *func_reg)?;
                        let RuntimeValue::Function(func_value) = closure_val else {
                            let stack = self.capture_stack();
                            return Err(ExecutorError::type_error(
                                "spawn expects a function value".to_string(),
                                stack,
                            ));
                        };
                        // #254：闭包帧设 upvalues（闭包体 LoadUpvalue 读 env），
                        // env 同时作 args 前段（LoadArg 读）。
                        calls.push(PendingCall::closure(
                            func_value.func_id,
                            func_value.env.clone(),
                            func_value.env,
                            Some(*func_reg),
                        ));
                    }
                    // 多调用的 ip 推进在所有被调返回后统一发生（`return_from_frame`）
                    self.start_calls(calls)?;
                    return Ok(StepOutcome::Continue);
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
                    // 同 Spawn：一条指令发起 N 个调用 → 首次压帧 + 其余排队续跑。
                    // 结果丢弃（旧实现同样丢弃 `_result`）；不注入上值
                    // （与 call_function_by_id 一致）。
                    let mut calls = Vec::with_capacity(closures.len());
                    for closure_val in closures.iter() {
                        let RuntimeValue::Function(func_value) = closure_val else {
                            let stack = self.capture_stack();
                            return Err(ExecutorError::type_error(
                                "spawn_from_list expects function values in list".to_string(),
                                stack,
                            ));
                        };
                        calls.push(PendingCall::function(
                            func_value.func_id,
                            func_value.env.clone(),
                            None,
                        ));
                    }
                    // 多调用的 ip 推进在所有被调返回后统一发生（`return_from_frame`）
                    self.start_calls(calls)?;
                    return Ok(StepOutcome::Continue);
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
