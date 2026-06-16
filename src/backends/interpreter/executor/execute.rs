//! Executor trait implementation for YaoXiang bytecode interpreter
//!
//! This module contains the Executor trait implementation with the main bytecode execution loop.

use crate::backends::{Executor, ExecutorResult, ExecutorError, ExecutionState};
use crate::backends::common::{RuntimeValue, Heap};
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

        // Create shared state for parallel task execution
        let shared = Box::new(SharedState {
            functions: self.functions.clone(),
            functions_by_id: self.functions_by_id.clone(),
            constants: self.constants.clone(),
            type_table: self.type_table.clone(),
            ffi: self.ffi.clone(),
        });
        self.shared = Box::into_raw(shared);

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
        // Create new frame and push onto call stack
        let mut frame = Frame::with_args(func.clone(), args);
        frame.set_entry_ip(0);
        self.push_frame(frame)?;

        // Execute via step_one loop — all instruction logic is in debug.rs
        loop {
            match self.step_one()? {
                super::debug::StepOutcome::Continue => {}
                super::debug::StepOutcome::Returned => {
                    return Ok(std::mem::replace(
                        &mut self.last_return_value,
                        RuntimeValue::Unit,
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
            work_stealing: self.runtime_config.work_stealing,
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
