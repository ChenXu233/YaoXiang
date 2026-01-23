//! Debugger for YaoXiang bytecode
//!
//! This module implements a command-line debugger with support for:
//! - Breakpoints
//! - Step-by-step execution
//! - Variable inspection
//! - Call stack navigation

use std::collections::HashSet;
use crate::backends::{DebuggableExecutor, StepMode, FrameInfo, ExecutorResult, Executor};
use crate::backends::common::{RuntimeValue, Heap};
use crate::backends::interpreter::Interpreter;
use crate::middle::bytecode::BytecodeModule;

/// Debugger state for a debugging session
#[derive(Debug, Clone, Default)]
pub struct DebuggerState {
    /// Current execution state
    pub state: crate::backends::ExecutionState,
    /// Breakpoint locations (function_name -> instruction_offsets)
    pub breakpoints: HashSet<(String, usize)>,
    /// Current breakpoint hit
    pub current_breakpoint: Option<String>,
    /// Step mode
    pub step_mode: StepMode,
    /// Step target IP
    pub step_target: Option<usize>,
    /// Breakpoints are enabled
    pub breakpoints_enabled: bool,
}

impl DebuggerState {
    /// Create a new debugger state
    pub fn new() -> Self {
        Self {
            state: crate::backends::ExecutionState::default(),
            breakpoints: HashSet::new(),
            current_breakpoint: None,
            step_mode: StepMode::Continue,
            step_target: None,
            breakpoints_enabled: true,
        }
    }
}

/// Debugger implementation wrapping an interpreter
///
/// The debugger provides a high-level interface for debugging YaoXiang programs.
/// It wraps an interpreter and adds debugging capabilities.
#[derive(Debug)]
pub struct Debugger {
    /// Inner interpreter
    interpreter: Interpreter,
    /// Debug state
    debug_state: DebuggerState,
    /// Loaded module
    loaded_module: Option<BytecodeModule>,
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}

impl Debugger {
    /// Create a new debugger
    pub fn new() -> Self {
        Self {
            interpreter: Interpreter::new(),
            debug_state: DebuggerState::new(),
            loaded_module: None,
        }
    }

    /// Load a module for debugging
    pub fn load_module(
        &mut self,
        module: &BytecodeModule,
    ) {
        self.loaded_module = Some(module.clone());
    }

    /// Set a breakpoint
    ///
    /// # Arguments
    /// * `function` - Function name
    /// * `offset` - Instruction offset
    pub fn set_breakpoint(
        &mut self,
        function: &str,
        offset: usize,
    ) {
        self.debug_state
            .breakpoints
            .insert((function.to_string(), offset));
    }

    /// Remove a breakpoint
    pub fn remove_breakpoint(
        &mut self,
        function: &str,
        offset: usize,
    ) {
        self.debug_state
            .breakpoints
            .remove(&(function.to_string(), offset));
    }

    /// Clear all breakpoints
    pub fn clear_breakpoints(&mut self) {
        self.debug_state.breakpoints.clear();
    }

    /// List all breakpoints
    pub fn list_breakpoints(&self) -> Vec<(String, usize)> {
        self.debug_state.breakpoints.iter().cloned().collect()
    }

    /// Enable/disable breakpoints
    pub fn set_breakpoints_enabled(
        &mut self,
        enabled: bool,
    ) {
        self.debug_state.breakpoints_enabled = enabled;
    }

    /// Step one instruction (internal)
    pub fn set_step_mode(
        &mut self,
        mode: StepMode,
    ) {
        self.debug_state.step_mode = mode;
    }

    /// Step one instruction (public)
    pub fn step_instruction(&mut self) {
        self.debug_state.step_mode = StepMode::Step;
    }

    /// Step over the next instruction (public)
    pub fn step_over_instruction(&mut self) {
        self.debug_state.step_mode = StepMode::StepOver;
    }

    /// Step out of the current function (public)
    pub fn step_out_instruction(&mut self) {
        self.debug_state.step_mode = StepMode::StepOut;
    }

    /// Continue execution (public)
    pub fn continue_execution(&mut self) {
        self.debug_state.step_mode = StepMode::Continue;
    }

    /// Get the current state
    pub fn state(&self) -> &DebuggerState {
        &self.debug_state
    }

    /// Get the current function name
    pub fn current_function(&self) -> Option<&str> {
        self.interpreter.state().current_function.as_deref()
    }

    /// Get the current instruction pointer
    pub fn current_ip(&self) -> usize {
        self.interpreter.state().ip
    }

    /// Get the current frame info
    pub fn frame_info(&self) -> FrameInfo {
        FrameInfo {
            function: self
                .current_function()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            ip: self.current_ip(),
            locals: Vec::new(),
        }
    }

    /// Print the current location
    pub fn print_location(&self) {
        if let Some(func_name) = self.current_function() {
            println!("  at {}:{}", func_name, self.current_ip());
        }
    }

    /// Print local variables
    pub fn print_locals(&self) {
        println!("  Locals:");
        // In a full implementation, we'd print actual local values
    }

    /// Print the call stack
    pub fn print_backtrace(&self) {
        println!("  Call stack:");
        if let Some(func_name) = self.current_function() {
            println!("    #0  {} at {}", func_name, self.current_ip());
        }
    }
}

impl crate::backends::Executor for Debugger {
    fn execute_module(
        &mut self,
        module: &crate::middle::bytecode::BytecodeModule,
    ) -> ExecutorResult<()> {
        self.interpreter.execute_module(module)
    }

    fn execute_function(
        &mut self,
        func: &crate::middle::bytecode::BytecodeFunction,
        args: &[RuntimeValue],
    ) -> ExecutorResult<RuntimeValue> {
        self.interpreter.execute_function(func, args)
    }

    fn reset(&mut self) {
        self.interpreter.reset();
    }

    fn state(&self) -> &crate::backends::ExecutionState {
        self.interpreter.state()
    }

    fn heap(&self) -> &Heap {
        self.interpreter.heap()
    }
}

impl DebuggableExecutor for Debugger {
    fn set_breakpoint(
        &mut self,
        offset: usize,
    ) {
        if let Some(func_name) = self.current_function().map(|s| s.to_string()) {
            self.set_breakpoint(&func_name, offset);
        }
    }

    fn remove_breakpoint(
        &mut self,
        offset: usize,
    ) {
        if let Some(func_name) = self.current_function().map(|s| s.to_string()) {
            self.remove_breakpoint(&func_name, offset);
        }
    }

    fn has_breakpoint(&self) -> bool {
        if let Some(func_name) = self.current_function() {
            self.debug_state
                .breakpoints
                .contains(&(func_name.to_string(), self.current_ip()))
        } else {
            false
        }
    }

    fn step(&mut self) -> ExecutorResult<()> {
        self.step_instruction();
        Ok(())
    }

    fn step_over(&mut self) -> ExecutorResult<()> {
        self.step_over_instruction();
        Ok(())
    }

    fn step_out(&mut self) -> ExecutorResult<()> {
        self.step_out_instruction();
        Ok(())
    }

    fn run(&mut self) -> ExecutorResult<()> {
        self.continue_execution();
        Ok(())
    }

    fn current_ip(&self) -> usize {
        self.interpreter.state().ip
    }

    fn current_function(&self) -> Option<&str> {
        self.interpreter.state().current_function.as_deref()
    }

    fn breakpoints(&self) -> Vec<usize> {
        self.list_breakpoints()
            .iter()
            .filter(|(f, _)| Some(f.as_str()) == self.current_function())
            .map(|(_, o)| *o)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_new() {
        let debugger = Debugger::new();
        assert!(debugger.state().breakpoints.is_empty());
    }

    #[test]
    fn test_set_breakpoint() {
        let mut debugger = Debugger::new();
        debugger.set_breakpoint("main", 10);
        assert_eq!(debugger.list_breakpoints().len(), 1);
    }

    #[test]
    fn test_remove_breakpoint() {
        let mut debugger = Debugger::new();
        debugger.set_breakpoint("main", 10);
        debugger.remove_breakpoint("main", 10);
        assert!(debugger.list_breakpoints().is_empty());
    }
}
