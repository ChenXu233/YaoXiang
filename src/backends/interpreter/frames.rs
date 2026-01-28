//! Call frames for the interpreter
//!
//! This module provides the call frame structure used for function calls.

use crate::backends::common::RuntimeValue;
use crate::middle::bytecode::{BytecodeFunction, Label};

/// Maximum number of local variables
pub const MAX_LOCALS: usize = 256;

/// Call frame for function execution
///
/// A call frame contains all the state needed to execute a function,
/// including its registers, instruction pointer, and local variables.
#[derive(Debug, Clone)]
pub struct Frame {
    /// The function being executed
    pub function: BytecodeFunction,
    /// Instruction pointer (index into instructions)
    pub ip: usize,
    /// Register file for this frame
    pub registers: Vec<RuntimeValue>,
    /// Local variable values (flat array)
    locals: Vec<RuntimeValue>,
    /// Upvalue capture values
    upvalues: Vec<RuntimeValue>,
    /// Entry IP (for stack unwinding)
    entry_ip: usize,
}

impl Frame {
    /// Create a new frame for a function
    pub fn new(function: BytecodeFunction) -> Self {
        let local_count = function.local_count.max(1);
        Self {
            function,
            ip: 0,
            registers: Vec::with_capacity(32),
            locals: vec![RuntimeValue::Unit; local_count],
            upvalues: Vec::new(),
            entry_ip: 0,
        }
    }

    /// Create a new frame with arguments
    pub fn with_args(
        function: BytecodeFunction,
        args: &[RuntimeValue],
    ) -> Self {
        let mut frame = Self::new(function);
        for (i, arg) in args.iter().enumerate() {
            if i < frame.locals.len() {
                frame.locals[i] = arg.clone();
            }
        }
        frame
    }

    /// Get the current instruction
    pub fn current_instr(&self) -> Option<&crate::middle::bytecode::BytecodeInstr> {
        self.function.instructions.get(self.ip)
    }

    /// Get the next instruction (without advancing)
    pub fn next_instr(&self) -> Option<&crate::middle::bytecode::BytecodeInstr> {
        self.function.instructions.get(self.ip + 1)
    }

    /// Advance the instruction pointer
    pub fn advance(&mut self) {
        self.ip += 1;
    }

    /// Jump to a label
    pub fn jump(
        &mut self,
        label: Label,
    ) {
        if let Some(&offset) = self.function.labels.get(&label) {
            self.ip = offset;
        }
    }

    /// Get a local variable
    pub fn get_local(
        &self,
        index: usize,
    ) -> Option<&RuntimeValue> {
        self.locals.get(index)
    }

    /// Set a local variable
    pub fn set_local(
        &mut self,
        index: usize,
        value: RuntimeValue,
    ) {
        if index >= self.locals.len() {
            self.locals.resize(index + 1, RuntimeValue::Unit);
        }
        self.locals[index] = value;
    }

    /// Set a register value, extending the register file if necessary
    pub fn set_register(
        &mut self,
        index: usize,
        value: RuntimeValue,
    ) {
        if index >= self.registers.len() {
            self.registers.resize(index + 1, RuntimeValue::Unit);
        }
        self.registers[index] = value;
    }

    /// Get an upvalue
    pub fn get_upvalue(
        &self,
        index: usize,
    ) -> Option<&RuntimeValue> {
        self.upvalues.get(index)
    }

    /// Set an upvalue
    pub fn set_upvalue(
        &mut self,
        index: usize,
        value: RuntimeValue,
    ) {
        if index >= self.upvalues.len() {
            self.upvalues.resize(index + 1, RuntimeValue::Unit);
        }
        self.upvalues[index] = value;
    }

    /// Get the function name
    pub fn function_name(&self) -> &str {
        &self.function.name
    }

    /// Get the entry IP
    pub fn entry_ip(&self) -> usize {
        self.entry_ip
    }

    /// Set the entry IP
    pub fn set_entry_ip(
        &mut self,
        ip: usize,
    ) {
        self.entry_ip = ip;
    }

    /// Get the number of local variables
    pub fn local_count(&self) -> usize {
        self.locals.len()
    }

    /// Get the number of upvalues
    pub fn upvalue_count(&self) -> usize {
        self.upvalues.len()
    }

    /// Get mutable access to upvalues (for closure capture)
    pub fn upvalues_mut(&mut self) -> &mut Vec<RuntimeValue> {
        &mut self.upvalues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::bytecode::BytecodeFunction;
    use std::collections::HashMap;

    fn make_test_function() -> BytecodeFunction {
        BytecodeFunction {
            name: "test".to_string(),
            params: vec![],
            return_type: crate::middle::core::ir::Type::Void,
            local_count: 2,
            upvalue_count: 0,
            instructions: vec![],
            labels: HashMap::new(),
            exception_handlers: vec![],
        }
    }

    #[test]
    fn test_frame_new() {
        let func = make_test_function();
        let frame = Frame::new(func);
        assert_eq!(frame.local_count(), 2);
        assert_eq!(frame.ip, 0);
    }

    #[test]
    fn test_frame_local_access() {
        let func = make_test_function();
        let mut frame = Frame::new(func);
        frame.set_local(0, RuntimeValue::Int(42));
        assert_eq!(frame.get_local(0).unwrap().to_int(), Some(42));
    }
}
