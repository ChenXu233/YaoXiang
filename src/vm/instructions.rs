//! VM instructions

/// VM instruction
#[derive(Debug, Clone, Copy)]
pub struct Instruction {
    /// Opcode
    pub opcode: u8,
    /// Operands
    pub operands: [u32; 4],
}

impl Instruction {
    /// Create a new instruction
    pub fn new(opcode: u8, operands: [u32; 4]) -> Self {
        Self { opcode, operands }
    }

    /// Get opcode
    pub fn get_opcode(&self) -> u8 {
        self.opcode
    }

    /// Get operand at index
    pub fn get_operand(&self, index: usize) -> Option<u32> {
        self.operands.get(index).copied()
    }
}



