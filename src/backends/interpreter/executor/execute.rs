//! Executor trait implementation for YaoXiang bytecode interpreter
//!
//! This module contains the Executor trait implementation with the main bytecode execution loop.

use crate::backends::{Executor, ExecutorResult, ExecutorError, ExecutionState};
use crate::backends::common::RuntimeValue;
use crate::backends::common::value::{FunctionId, FunctionValue};
use crate::middle::bytecode::{BytecodeModule, BytecodeFunction};
use crate::backends::interpreter::frames::MAX_LOCALS;
use std::sync::Arc;
use crate::backends::runtime::Runtime;
use crate::backends::runtime::facade::RuntimeConfig;
use crate::util::i18n::MSG;
use crate::tlog;
use super::executor::Interpreter;

impl Executor for Interpreter {
    fn execute_module(
        &mut self,
        module: &BytecodeModule,
    ) -> ExecutorResult<()> {
        // 装载期唯一持有 image（尚无任务共享），故 get_mut 必然成功。
        // 装载完成后 image 转为只读并可能被 Arc 共享给任务，不再改写。
        let image = Arc::get_mut(&mut self.image).expect("模块装载期 image 不应已被共享");

        // Add constants
        image.constants.extend(module.constants.clone());

        // Add functions
        for func in &module.functions {
            tlog!(debug, MSG::DebugLoadingFunction, &func.name);
            image.functions_by_id.push(func.clone());
        }
        tlog!(
            debug,
            MSG::DebugTotalFunctions,
            &image.functions_by_id.len()
        );
        tlog!(
            debug,
            MSG::DebugAvailableFunctions,
            &format!(
                "{:?}",
                image
                    .functions_by_id
                    .iter()
                    .map(|f| &f.name)
                    .collect::<Vec<_>>()
            )
        );

        // 从字节码的编译期 vtables 段直建 vtable 缓存：type_name → [(裸方法名, FunctionValue)]。
        // 方法 func_id 由 codegen 写入（函数表索引），加载期零解析。
        for (type_name, methods) in &module.vtables {
            let vt = methods
                .iter()
                .map(|(bare, func_idx)| {
                    (
                        bare.clone(),
                        FunctionValue {
                            func_id: FunctionId(*func_idx),
                            env: Vec::new(),
                        },
                    )
                })
                .collect();
            image.vtable_cache.insert(type_name.clone(), vt);
        }

        // Add types
        image.type_table.extend(module.type_table.clone());

        // T2：先执行模块初始化（顶层绑定求值 → 全局槽位写入），
        // 再执行入口。初始化函数由 codegen 合成（`__yx_module_init`）。
        if let Some(init_idx) = module.init_function {
            if let Some(init_func) = module.functions.get(init_idx) {
                self.execute_function(init_func, &[])?;
            }
        }

        // Execute entry point
        if let Some(entry_idx) = module.entry_point {
            if entry_idx < module.functions.len() {
                let entry_func = &module.functions[entry_idx];
                let result = self.execute_function(entry_func, &[])?;
                // Print result if not void
                if !matches!(result, RuntimeValue::Void) {
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
        // 公开 API 接受任意函数，含不在函数表内的临时构造函数（如单测直接构造的
        // `BytecodeFunction`）。按名字命中则复用其 func_id；未命中时向表中追加
        // 一份以取得 func_id。
        //
        // 追加用 `Arc::make_mut` 写时复制：装载完成后 image 可能已被并发任务共享，
        // 此时克隆一份（罕见路径，仅表外函数触发）；未被共享时原地修改，零拷贝。
        // 热路径（call_function_by_id / call_closure）不经此函数，直接走
        // execute_by_id，无查找、无克隆。
        let fid = match self
            .image
            .functions_by_id
            .iter()
            .position(|f| f.name == func.name)
        {
            Some(i) => i as u32,
            None => {
                let image = Arc::make_mut(&mut self.image);
                image.functions_by_id.push(func.clone());
                (image.functions_by_id.len() - 1) as u32
            }
        };
        self.execute_by_id(fid, func.local_count, args)
    }

    fn reset(&mut self) {
        self.heap.clear();
        self.call_stack.clear();
        self.state = ExecutionState::default();
        self.breakpoints.clear();
        self.rt = Runtime::new(RuntimeConfig {
            mode: self.runtime_config.mode,
            workers: self.runtime_config.workers,
        })
        .unwrap_or_else(|_| Runtime::new(RuntimeConfig::default()).unwrap());
    }

    fn state(&self) -> &ExecutionState {
        &self.state
    }
}
