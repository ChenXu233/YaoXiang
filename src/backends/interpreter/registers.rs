//! Register file for the interpreter
//!
//! Provides a flat register array for the bytecode interpreter.

use crate::backends::common::RuntimeValue;

/// Number of general purpose registers
pub const GENERAL_PURPOSE_REGS: usize = 32;

/// Register file for the virtual machine
///
/// The register file provides fast access to values during execution.
/// Registers are indexed from 0 to N-1.
#[derive(Debug, Clone)]
pub struct RegisterFile {
    /// Register values
    registers: Vec<RuntimeValue>,
    /// Number of valid registers
    count: usize,
}

impl Default for RegisterFile {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterFile {
    /// Create a new register file with default size
    pub fn new() -> Self {
        Self::with_size(GENERAL_PURPOSE_REGS)
    }

    /// Create a register file with specified size
    pub fn with_size(size: usize) -> Self {
        Self {
            registers: vec![RuntimeValue::default(); size],
            count: size,
        }
    }

    /// Get the number of registers
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get a register value
    pub fn get(
        &self,
        index: usize,
    ) -> Option<&RuntimeValue> {
        self.registers.get(index)
    }

    /// Get a mutable register value
    pub fn get_mut(
        &mut self,
        index: usize,
    ) -> Option<&mut RuntimeValue> {
        self.registers.get_mut(index)
    }

    /// Get a register value (panics if out of bounds)
    ///
    /// # Panics
    /// If index >= count
    pub fn at(
        &self,
        index: usize,
    ) -> &RuntimeValue {
        &self.registers[index]
    }

    /// Get a mutable register value (panics if out of bounds)
    pub fn at_mut(
        &mut self,
        index: usize,
    ) -> &mut RuntimeValue {
        &mut self.registers[index]
    }

    /// Set a register value
    pub fn set(
        &mut self,
        index: usize,
        value: RuntimeValue,
    ) {
        if index >= self.registers.len() {
            self.registers.resize(index + 1, RuntimeValue::default());
        }
        self.registers[index] = value;
        self.count = self.count.max(index + 1);
    }

    /// Copy a value between registers
    pub fn copy(
        &mut self,
        dst: usize,
        src: usize,
    ) {
        self.set(dst, self.at(src).clone());
    }

    /// Clear all registers
    pub fn clear(&mut self) {
        for reg in &mut self.registers {
            *reg = RuntimeValue::default();
        }
        self.count = 0;
    }

    /// Get all registers as a slice
    pub fn as_slice(&self) -> &[RuntimeValue] {
        &self.registers[..self.count]
    }

    /// Get mutable access to all registers
    pub fn as_mut_slice(&mut self) -> &mut [RuntimeValue] {
        &mut self.registers[..self.count]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_file_new() {
        let rf = RegisterFile::new();
        assert_eq!(rf.len(), GENERAL_PURPOSE_REGS);
    }

    #[test]
    fn test_register_set_get() {
        let mut rf = RegisterFile::new();
        rf.set(0, RuntimeValue::Int(42));
        assert_eq!(rf.at(0).to_int(), Some(42));
    }

    #[test]
    fn test_register_copy() {
        let mut rf = RegisterFile::new();
        rf.set(0, RuntimeValue::Int(42));
        rf.copy(1, 0);
        assert_eq!(rf.at(1).to_int(), Some(42));
    }
}
