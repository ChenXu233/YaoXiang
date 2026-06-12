//! Debugger implementation and tests for YaoXiang bytecode interpreter
//!
//! This module contains the DebuggableExecutor trait implementation and unit tests.

use crate::backends::DebuggableExecutor;
use crate::backends::ExecutorResult;
use super::executor::Interpreter;

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
        todo!("step debugging not implemented")
    }

    fn step_over(&mut self) -> ExecutorResult<()> {
        todo!("step_over debugging not implemented")
    }

    fn step_out(&mut self) -> ExecutorResult<()> {
        todo!("step_out debugging not implemented")
    }

    fn run(&mut self) -> ExecutorResult<()> {
        todo!("run debugging not implemented")
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
