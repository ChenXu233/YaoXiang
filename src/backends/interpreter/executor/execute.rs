//! Executor trait implementation for YaoXiang bytecode interpreter
//!
//! This module contains the Executor trait implementation with the main bytecode execution loop.

use crate::backends::{Executor, ExecutorResult, ExecutorError, ExecutionState};
use crate::backends::common::{RuntimeValue, Heap};
use crate::backends::common::value::{FunctionId, FunctionValue};
use crate::middle::bytecode::{BytecodeModule, BytecodeFunction};
use crate::backends::interpreter::Frame;
use crate::backends::interpreter::frames::MAX_LOCALS;
use crate::backends::runtime::Runtime;
use crate::backends::runtime::facade::RuntimeConfig;
use crate::util::i18n::MSG;
use crate::tlog;
use super::executor::{Interpreter, SharedState};

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
            self.functions_by_id.push(func.clone());
        }
        tlog!(debug, MSG::DebugTotalFunctions, &self.functions_by_id.len());
        tlog!(
            debug,
            MSG::DebugAvailableFunctions,
            &format!(
                "{:?}",
                self.functions_by_id
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
            self.vtable_cache.insert(type_name.clone(), vt);
        }

        // Add types
        self.type_table.extend(module.type_table.clone());

        // Create shared state for parallel task execution
        let shared = Box::new(SharedState {
            functions_by_id: self.functions_by_id.clone(),
            constants: self.constants.clone(),
            type_table: self.type_table.clone(),
            vtable_cache: self.vtable_cache.clone(),
            ffi: self.ffi.clone(),
        });
        self.shared = Box::into_raw(shared);

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
        let mut frame = Frame::with_args(func.clone(), args);
        frame.set_entry_ip(0);
        self.push_frame(frame)?;
        loop {
            match self.step_one()? {
                super::debug::StepOutcome::Continue => {}
                super::debug::StepOutcome::Returned => {
                    return Ok(std::mem::replace(
                        &mut self.last_return_value,
                        RuntimeValue::Void,
                    ))
                }
            }
        }
    }

    fn reset(&mut self) {
        self.heap.clear();
        self.call_stack.clear();
        self.state = ExecutionState::default();
        self.breakpoints.clear();
        self.current_frame_info = None;
        self.called_func = false;
        self.rt = Runtime::new(RuntimeConfig {
            mode: self.runtime_config.runtime,
            workers: self.runtime_config.workers,
        })
        .unwrap_or_else(|_| Runtime::new(RuntimeConfig::default()).unwrap());
    }

    fn state(&self) -> &ExecutionState {
        &self.state
    }

    fn heap(&self) -> &Heap {
        &self.heap
    }
}
